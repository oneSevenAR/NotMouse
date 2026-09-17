#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_arguments,
    clippy::items_after_statements,
    clippy::unnecessary_wraps,
    clippy::similar_names,
    clippy::many_single_char_names
)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

mod atspi;
mod input;
mod overlay;
mod session;
mod test_bench;

use input::{InputDevice, MouseButton};
use notmouse_core::{Rect, SpatialMatrix2D};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum OverlayEvent {
    Selected {
        strokes: String,
        action: String,
        normalized: NormalizedPoint,
    },
    Move {
        x: f64,
        y: f64,
    },
    MoveRelative {
        dx: i32,
        dy: i32,
    },
    Scroll {
        dx: i32,
        dy: i32,
    },
    Press {
        button: String,
    },
    Release {
        button: String,
    },
    Click {
        button: String,
        normalized: Option<NormalizedPoint>,
    },
    Cancelled,
    Shutdown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NormalizedPoint {
    pub x: f64,
    pub y: f64,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        Some("demo") => {
            print_demo();
            ExitCode::SUCCESS
        }
        Some("atspi-test") => match zbus::block_on(atspi::test_scan()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("atspi error: {err}");
                ExitCode::FAILURE
            }
        },
        Some("daemon") => match run_daemon() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("notmouse: {error}");
                ExitCode::FAILURE
            }
        },
        Some("overlay") => {
            let is_resident = args.any(|a| a == "--resident" || a == "-r" || a == "--background");
            if is_resident {
                match overlay::run_overlay(true) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => {
                        eprintln!("notmouse: {error}");
                        ExitCode::FAILURE
                    }
                }
            } else {
                match launch_overlay() {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(error) => {
                        eprintln!("notmouse: {error}");
                        ExitCode::FAILURE
                    }
                }
            }
        }
        Some("click") => {
            let x: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
            let y: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
            match click_at(x, y) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("notmouse: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("move") => {
            let x: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
            let y: f64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(0.5);
            match move_to(x, y) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("notmouse: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("scroll") => {
            let steps_y: i32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(-3);
            let x: Option<f64> = args.next().and_then(|s| s.parse().ok());
            let y: Option<f64> = args.next().and_then(|s| s.parse().ok());
            match scroll_at(steps_y, x, y) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("notmouse: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Some("playground" | "test") => match launch_playground() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("notmouse: {error}");
                ExitCode::FAILURE
            }
        },
        Some("test-bench") => match launch_test_bench() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("notmouse: {error}");
                ExitCode::FAILURE
            }
        },
        Some("--version" | "-V") => {
            println!("notmouse {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

fn print_help() {
    println!(
        "!mouse {}\n\nUsage:\n  notmouse playground          Open test bench and overlay together (warm session)\n  notmouse daemon              Run the persistent input session daemon in the background\n  notmouse overlay             Open the 2-stroke spatial matrix overlay and perform action\n  notmouse test-bench          Open the interactive test bench window alone (Esc to exit)\n  notmouse click <x> <y>       Move to normalized (x, y) and left-click\n  notmouse move <x> <y>        Move to normalized (x, y)\n  notmouse scroll <dy> [x] [y] Scroll vertically (negative=down, positive=up)\n  notmouse demo                Show the terminal matrix demonstration\n  notmouse --version           Show the version",
        env!("CARGO_PKG_VERSION")
    );
}

struct DaemonGuard {
    warm_overlay: Option<std::process::Child>,
    monitor_running: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        self.monitor_running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        if let Some(mut child) = self.warm_overlay.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_file(session::overlay_socket_path());
        let _ = std::fs::remove_file(session::active_pid_path());
    }
}

fn run_daemon() -> Result<(), String> {
    if session::try_connect().is_some() {
        return Err(format!(
            "a !mouse resident daemon is already running on {}",
            session::socket_path().display()
        ));
    }

    println!("!mouse: initializing persistent virtual input device...");
    let device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;
    let path = session::socket_path();
    let _server = session::start_server(device)?;

    println!(
        "!mouse: resident daemon ready (listening on {})",
        path.display()
    );

    // Pre-warm the native warm overlay in the background so cold launch latency is eliminated!
    println!("!mouse: pre-warming native resident overlay adapter...");
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("notmouse"));
    let warm_overlay = Command::new(&exe)
        .arg("overlay")
        .arg("--resident")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .ok();

    // Start background AT-SPI active window monitor so the overlay always knows the foreground app
    println!("!mouse: starting native AT-SPI foreground application tracker...");
    let monitor_running = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
    let m_running = monitor_running.clone();
    std::thread::spawn(move || {
        let rt = match zbus::block_on(atspi::AccessibilityConnection::new()) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("!mouse: active window monitor failed to connect to AT-SPI: {e}");
                return;
            }
        };
        let pid_path = session::active_pid_path();
        let mut last_pid = 0;
        while m_running.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(200));
            if let Some(pid) = zbus::block_on(atspi::get_active_window_pid(&rt))
                && pid != last_pid
            {
                last_pid = pid;
                let tmp = pid_path.with_extension("tmp");
                if std::fs::write(&tmp, format!("{pid}\n")).is_ok() {
                    let _ = std::fs::rename(&tmp, &pid_path);
                }
            }
        }
    });

    println!("!mouse: compositor binding warm and permanent. Press Ctrl+C to terminate.");

    let mut guard = DaemonGuard {
        warm_overlay,
        monitor_running,
    };
    loop {
        thread::sleep(Duration::from_secs(5));
        if let Some(ref mut child) = guard.warm_overlay
            && let Ok(Some(_status)) = child.try_wait()
        {
            guard.warm_overlay = Command::new(&exe)
                .arg("overlay")
                .arg("--resident")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .ok();
        }
    }
}

fn click_at(x: f64, y: f64) -> Result<(), String> {
    let event = OverlayEvent::Click {
        button: "left".into(),
        normalized: Some(NormalizedPoint { x, y }),
    };
    if session::send_event(&event).unwrap_or(false) {
        println!("!mouse: click at ({x:.3}, {y:.3}) dispatched to resident session");
        return Ok(());
    }

    let mut device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;
    println!("!mouse: moving to ({x:.3}, {y:.3}) and clicking");
    device
        .move_to_normalized(x, y)
        .map_err(|err| format!("move failed: {err}"))?;
    thread::sleep(Duration::from_millis(50));
    device
        .click(MouseButton::Left)
        .map_err(|err| format!("click failed: {err}"))?;
    thread::sleep(Duration::from_millis(350));
    Ok(())
}

fn move_to(x: f64, y: f64) -> Result<(), String> {
    let event = OverlayEvent::Move { x, y };
    if session::send_event(&event).unwrap_or(false) {
        println!("!mouse: move to ({x:.3}, {y:.3}) dispatched to resident session");
        return Ok(());
    }

    let mut device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;
    println!("!mouse: moving to ({x:.3}, {y:.3})");
    device
        .move_to_normalized(x, y)
        .map_err(|err| format!("move failed: {err}"))?;
    thread::sleep(Duration::from_millis(350));
    Ok(())
}

fn scroll_at(steps_y: i32, x: Option<f64>, y: Option<f64>) -> Result<(), String> {
    if let (Some(x), Some(y)) = (x, y) {
        let move_evt = OverlayEvent::Move { x, y };
        let scroll_evt = OverlayEvent::Scroll { dx: 0, dy: steps_y };
        if session::send_event(&move_evt).unwrap_or(false) {
            thread::sleep(Duration::from_millis(50));
            let _ = session::send_event(&scroll_evt);
            println!("!mouse: scroll at ({x:.3}, {y:.3}) dispatched to resident session");
            return Ok(());
        }
    } else {
        let scroll_evt = OverlayEvent::Scroll { dx: 0, dy: steps_y };
        if session::send_event(&scroll_evt).unwrap_or(false) {
            println!("!mouse: scroll {steps_y} steps dispatched to resident session");
            return Ok(());
        }
    }

    let mut device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;
    if let (Some(x), Some(y)) = (x, y) {
        println!("!mouse: moving to ({x:.3}, {y:.3}) and scrolling {steps_y} steps");
        device
            .move_to_normalized(x, y)
            .map_err(|err| format!("move failed: {err}"))?;
        thread::sleep(Duration::from_millis(50));
    } else {
        println!("!mouse: scrolling {steps_y} steps");
    }
    device
        .scroll(steps_y, 0)
        .map_err(|err| format!("scroll failed: {err}"))?;
    thread::sleep(Duration::from_millis(350));
    Ok(())
}

fn launch_overlay() -> Result<(), String> {
    let start_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);

    // Ultra-fast path: if resident warm overlay socket is active, trigger unhide in <0.5ms!
    let overlay_sock = session::overlay_socket_path();
    if overlay_sock.exists()
        && let Ok(mut stream) = std::os::unix::net::UnixStream::connect(&overlay_sock)
    {
        let target_pid_part = session::read_active_pid()
            .map(|pid| format!(",\"target_pid\":{pid}"))
            .unwrap_or_default();
        let msg = format!("{{\"action\":\"show\",\"start_us\":{start_us}{target_pid_part}}}\n");
        if stream.write_all(msg.as_bytes()).is_ok() && stream.flush().is_ok() {
            return Ok(());
        }
    }

    // Direct native GTK 4 overlay launch! Zero external scripts!
    overlay::run_overlay(false)
}

fn launch_test_bench() -> Result<(), String> {
    test_bench::run_test_bench()
}

fn launch_playground() -> Result<(), String> {
    // Keep persistent resident session warm for entire playground lifetime
    let _server_guard = if session::try_connect().is_none() {
        println!("!mouse: initializing persistent virtual input device (warm session)...");
        let device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;
        let server = session::start_server(device)?;
        println!(
            "!mouse: persistent session active on {}",
            session::socket_path().display()
        );
        Some(server)
    } else {
        println!(
            "!mouse: using active persistent session on {}",
            session::socket_path().display()
        );
        None
    };

    println!("!mouse: launching interactive test bench...");
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("notmouse"));
    let mut bench_child = Command::new(&exe)
        .arg("test-bench")
        .spawn()
        .map_err(|error| format!("could not spawn test bench: {error}"))?;

    // Give test bench window a moment to map to the display
    thread::sleep(Duration::from_millis(650));

    println!("!mouse: summoning overlay directly over test bench...");
    let _ = launch_overlay();

    println!(
        "!mouse: test bench running! Inside the window, press Tab to summon !mouse again, or Esc to exit."
    );
    let status = bench_child
        .wait()
        .map_err(|error| format!("failed waiting for test bench: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("the test bench exited with {status}"))
    }
}

fn print_demo() {
    let screen = Rect::new(0.0, 0.0, 1920.0, 1080.0);
    let matrix = SpatialMatrix2D::new(screen);
    let macro_zones = matrix.macro_zones();

    println!("!mouse 2-Stroke Spatial Matrix Demo");
    println!(
        "Total reachable target points: {} in 2 keystrokes\n",
        matrix.all_zones().len()
    );

    println!("Stroke 1: Choose a macro region (home row keys):");
    for row in macro_zones.chunks(3) {
        println!(
            "  [  {:^2}  ]    [  {:^2}  ]    [  {:^2}  ]",
            row[0].hint.to_uppercase(),
            row[1].hint.to_uppercase(),
            row[2].hint.to_uppercase()
        );
    }

    println!("\nStroke 2: Typing 'd' focuses region D and reveals 9 sub-zones:");
    if let Some(micro_zones) = matrix.micro_zones('d') {
        for row in micro_zones.chunks(3) {
            println!(
                "  [ {:^4} ]    [ {:^4} ]    [ {:^4} ]",
                row[0].hint.to_uppercase(),
                row[1].hint.to_uppercase(),
                row[2].hint.to_uppercase(),
            );
        }
    }
    println!("\nTyping 'k' resolves to 'DK' -> normalized coordinates (X, Y) and fires click.");
}
