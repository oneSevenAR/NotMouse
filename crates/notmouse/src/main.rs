use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

mod input;
mod session;

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
        Some("daemon") => match run_daemon() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("notmouse: {error}");
                ExitCode::FAILURE
            }
        },
        Some("overlay") => match launch_overlay() {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("notmouse: {error}");
                ExitCode::FAILURE
            }
        },
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
    println!("!mouse: compositor binding warm and permanent. Press Ctrl+C to terminate.");

    loop {
        thread::sleep(Duration::from_secs(3600));
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
    let script = overlay_script();
    if !script.is_file() {
        return Err(format!(
            "overlay adapter was not found at {}",
            script.display()
        ));
    }

    if let Some(mut stream) = session::try_connect() {
        // Fast path: resident session active! Instant overlay launch with zero device churn!
        let mut child = Command::new("gjs")
            .arg(script)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|error| format!("could not start the GNOME overlay adapter: {error}"))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "could not capture overlay stdout".to_string())?;

        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|e| format!("reading overlay stdout failed: {e}"))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('{') {
                let _ = writeln!(stream, "{trimmed}");
                let _ = stream.flush();
            } else {
                println!("{trimmed}");
            }
        }

        let status = child
            .wait()
            .map_err(|err| format!("failed to wait on overlay process: {err}"))?;

        if !status.success() {
            return Err(format!("the overlay adapter exited with {status}"));
        }

        return Ok(());
    }

    // Standalone fallback: no resident daemon running
    println!(
        "!mouse: [notice] starting standalone input device (run 'notmouse playground' or 'notmouse daemon' for zero latency)"
    );
    let mut device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;

    let mut child = Command::new("gjs")
        .arg(script)
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start the GNOME overlay adapter: {error}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "could not capture overlay stdout".to_string())?;

    let reader = BufReader::new(stdout);
    let mut selected_event = None;

    for line in reader.lines() {
        let line = line.map_err(|e| format!("reading overlay stdout failed: {e}"))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(event) = serde_json::from_str::<OverlayEvent>(trimmed) {
            if let OverlayEvent::Selected { .. } = event {
                selected_event = Some(event);
            } else {
                let _ = session::execute_event(&mut device, &event);
            }
        } else {
            println!("{trimmed}");
        }
    }

    let status = child
        .wait()
        .map_err(|err| format!("failed to wait on overlay process: {err}"))?;

    if !status.success() {
        return Err(format!("the overlay adapter exited with {status}"));
    }

    if let Some(selected) = selected_event {
        let _ = session::execute_event(&mut device, &selected);
    }

    Ok(())
}

fn overlay_script() -> PathBuf {
    std::env::var_os("NOTMOUSE_OVERLAY_SCRIPT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../platform/linux/gnome/overlay.js"),
        PathBuf::from,
    )
}

fn test_bench_script() -> PathBuf {
    std::env::var_os("NOTMOUSE_TEST_BENCH_SCRIPT").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../platform/linux/gnome/test_bench.js")
        },
        PathBuf::from,
    )
}

fn launch_test_bench() -> Result<(), String> {
    let script = test_bench_script();
    if !script.is_file() {
        return Err(format!(
            "test bench script was not found at {}",
            script.display()
        ));
    }

    let mut cmd = Command::new("gjs");
    cmd.arg(script);
    if let Ok(exe) = std::env::current_exe() {
        cmd.env("NOTMOUSE_BIN", exe);
    }

    let status = cmd
        .status()
        .map_err(|error| format!("could not start the test bench: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("the test bench exited with {status}"))
    }
}

fn launch_playground() -> Result<(), String> {
    let script = test_bench_script();
    if !script.is_file() {
        return Err(format!(
            "test bench script was not found at {}",
            script.display()
        ));
    }

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
    let mut cmd = Command::new("gjs");
    cmd.arg(script);
    if let Ok(exe) = std::env::current_exe() {
        cmd.env("NOTMOUSE_BIN", exe);
    }

    let mut bench_child = cmd
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
                row[2].hint.to_uppercase()
            );
        }
    }
    println!("\nTyping 'k' resolves to 'DK' -> normalized coordinates (X, Y) and fires click.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_device_dev_nodes() {
        let mut dev = InputDevice::new().expect("device creation should succeed");
        // test dev node discovery
        let path = dev.device.get_syspath();
        println!("Syspath: {path:?}");
        if let Ok(mut nodes) = dev.device.enumerate_dev_nodes_blocking() {
            while let Some(Ok(node)) = nodes.next() {
                println!("Node: {node:?}");
            }
        }
    }
}
