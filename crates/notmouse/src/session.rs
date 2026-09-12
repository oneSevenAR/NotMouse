use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::OverlayEvent;
use crate::input::{InputDevice, MouseButton};

/// Returns the path to the Unix domain socket used for resident sessions.
#[must_use]
pub fn socket_path() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map_or_else(std::env::temp_dir, PathBuf::from)
        .join("notmouse.sock")
}

/// Attempts to connect to an existing resident session daemon.
///
/// If a stale socket file exists from an unclean shutdown, it is automatically removed.
#[must_use]
pub fn try_connect() -> Option<UnixStream> {
    try_connect_at(&socket_path())
}

/// Attempts to connect to a resident session daemon at a specific socket path.
#[must_use]
pub fn try_connect_at(path: &Path) -> Option<UnixStream> {
    if !path.exists() {
        return None;
    }
    if let Ok(stream) = UnixStream::connect(path) {
        Some(stream)
    } else {
        // Remove stale socket from previous terminated session
        let _ = std::fs::remove_file(path);
        None
    }
}

/// Sends a single event to the resident daemon session.
///
/// Returns `Ok(true)` if sent successfully, `Ok(false)` if no daemon is running,
/// or an error message if serialization/writing failed.
pub fn send_event(event: &OverlayEvent) -> Result<bool, String> {
    let Some(mut stream) = try_connect() else {
        return Ok(false);
    };

    let line = serde_json::to_string(event).map_err(|e| format!("serialization error: {e}"))?;
    writeln!(stream, "{line}").map_err(|e| format!("socket write error: {e}"))?;
    stream
        .flush()
        .map_err(|e| format!("socket flush error: {e}"))?;
    Ok(true)
}

/// Handle representing a running resident session server.
///
/// Dropping this handle cleanly signals the background listener to exit
/// and removes the socket file.
pub struct ServerHandle {
    shutdown_flag: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    socket_path: PathBuf,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let _ = UnixStream::connect(&self.socket_path);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

/// Starts the resident session server on a background thread using the default socket path.
pub fn start_server(device: InputDevice) -> Result<ServerHandle, String> {
    start_server_at(device, socket_path())
}

/// Starts the resident session server on a background thread using the specified socket path.
pub fn start_server_at(device: InputDevice, path: PathBuf) -> Result<ServerHandle, String> {
    let _ = std::fs::remove_file(&path);

    let listener = UnixListener::bind(&path)
        .map_err(|err| format!("failed to bind socket at {}: {err}", path.display()))?;

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown_flag);
    let device_arc = Arc::new(Mutex::new(device));
    let path_clone = path.clone();

    let thread_handle = thread::spawn(move || {
        while !shutdown_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    if shutdown_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    let dev = Arc::clone(&device_arc);
                    let shutdown_child = Arc::clone(&shutdown_clone);

                    thread::spawn(move || {
                        let reader = BufReader::new(stream);
                        for line in reader.lines() {
                            if shutdown_child.load(Ordering::SeqCst) {
                                break;
                            }
                            let Ok(line) = line else { break };
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }
                            if let Ok(event) = serde_json::from_str::<OverlayEvent>(line) {
                                if matches!(event, OverlayEvent::Shutdown) {
                                    shutdown_child.store(true, Ordering::SeqCst);
                                    break;
                                }
                                if let Ok(mut dev_guard) = dev.lock() {
                                    let _ = execute_event(&mut dev_guard, &event);
                                }
                            }
                        }
                    });
                }
                Err(e) => {
                    if !shutdown_clone.load(Ordering::SeqCst) {
                        eprintln!("!mouse: session listener error: {e}");
                    }
                    break;
                }
            }
        }
        let _ = std::fs::remove_file(&path_clone);
    });

    Ok(ServerHandle {
        shutdown_flag,
        thread_handle: Some(thread_handle),
        socket_path: path,
    })
}

/// Dispatches an `OverlayEvent` to the warm `InputDevice`.
pub fn execute_event(device: &mut InputDevice, event: &OverlayEvent) -> Result<(), String> {
    match event {
        OverlayEvent::Move { x, y } => {
            device
                .move_to_normalized(*x, *y)
                .map_err(|e| format!("live move failed: {e}"))?;
            thread::sleep(Duration::from_millis(15));
        }
        OverlayEvent::Scroll { dx, dy } => {
            device
                .scroll(*dy, *dx)
                .map_err(|e| format!("live scroll failed: {e}"))?;
        }
        OverlayEvent::Press { button } => {
            let btn = parse_button(button);
            device
                .press(btn)
                .map_err(|e| format!("live press failed: {e}"))?;
        }
        OverlayEvent::Release { button } => {
            let btn = parse_button(button);
            device
                .release(btn)
                .map_err(|e| format!("live release failed: {e}"))?;
        }
        OverlayEvent::Click { button, normalized } => {
            // Wait briefly for Mutter to unmap the overlay and focus the window beneath
            thread::sleep(Duration::from_millis(80));
            if let Some(p) = normalized {
                println!("!mouse: live click '{button}' at ({:.3}, {:.3})", p.x, p.y);
                let _ = device.move_to_normalized(p.x, p.y);
                thread::sleep(Duration::from_millis(30));
            } else {
                println!("!mouse: live click '{button}' at current position");
            }
            if button == "double" || button == "double-click" {
                let _ = device.double_click(MouseButton::Left);
            } else {
                let btn = parse_button(button);
                let _ = device.click(btn);
            }
        }
        OverlayEvent::Selected {
            strokes,
            action,
            normalized,
        } => {
            // Wait briefly for Mutter to unmap the overlay and focus the window beneath
            thread::sleep(Duration::from_millis(120));
            println!(
                "!mouse: applying action '{action}' at ({:.3}, {:.3}) (strokes: {strokes})",
                normalized.x, normalized.y
            );
            device
                .move_to_normalized(normalized.x, normalized.y)
                .map_err(|err| format!("failed to move pointer: {err}"))?;
            thread::sleep(Duration::from_millis(50));
            execute_action(device, action)?;
            thread::sleep(Duration::from_millis(200));
        }
        OverlayEvent::Cancelled => {
            println!("!mouse: session cancelled");
        }
        OverlayEvent::Shutdown => {}
    }
    Ok(())
}

/// Parses a button name into a [`MouseButton`].
#[must_use]
pub fn parse_button(name: &str) -> MouseButton {
    match name {
        "right" | "right-click" => MouseButton::Right,
        "middle" | "middle-click" => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

/// Dispatches an action command string on the given device.
pub fn execute_action(device: &mut InputDevice, action: &str) -> Result<(), String> {
    match action {
        "click" => device
            .click(MouseButton::Left)
            .map_err(|err| format!("click failed: {err}")),
        "right-click" => device
            .click(MouseButton::Right)
            .map_err(|err| format!("right-click failed: {err}")),
        "double-click" => device
            .double_click(MouseButton::Left)
            .map_err(|err| format!("double-click failed: {err}")),
        "middle-click" => device
            .click(MouseButton::Middle)
            .map_err(|err| format!("middle-click failed: {err}")),
        "drag" => {
            device
                .press(MouseButton::Left)
                .map_err(|err| format!("drag failed: {err}"))?;
            println!("!mouse: drag engaged (Left button held down)");
            Ok(())
        }
        "scroll" | "scroll-down" => device
            .scroll(-5, 0)
            .map_err(|err| format!("scroll failed: {err}")),
        "scroll-up" => device
            .scroll(5, 0)
            .map_err(|err| format!("scroll failed: {err}")),
        other => {
            eprintln!("!mouse: unrecognized action '{other}', performing left-click");
            device
                .click(MouseButton::Left)
                .map_err(|err| format!("click failed: {err}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let p = socket_path();
        assert!(p.ends_with("notmouse.sock"));
    }

    #[test]
    fn test_server_lifecycle_and_event() {
        let dev = match InputDevice::new() {
            Ok(d) => d,
            Err(e) => {
                eprintln!(
                    "skipping test_server_lifecycle_and_event: /dev/uinput not accessible ({e})"
                );
                return;
            }
        };
        let test_sock =
            std::env::temp_dir().join(format!("notmouse_test_{}.sock", std::process::id()));
        let server = start_server_at(dev, test_sock.clone()).expect("server start should succeed");
        assert!(test_sock.exists());

        let connected = try_connect_at(&test_sock);
        assert!(connected.is_some());

        drop(server);
        thread::sleep(Duration::from_millis(50));
        assert!(!test_sock.exists());
    }
}
