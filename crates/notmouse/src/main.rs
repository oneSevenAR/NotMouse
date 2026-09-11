use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Duration;

mod input;

use input::{InputDevice, MouseButton};
use notmouse_core::{Rect, SpatialMatrix2D};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum OverlayEvent {
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
}

#[derive(Debug, Deserialize)]
struct NormalizedPoint {
    x: f64,
    y: f64,
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        Some("demo") => {
            print_demo();
            ExitCode::SUCCESS
        }
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
        "!mouse {}\n\nUsage:\n  notmouse playground          Open test bench and overlay together in one command\n  notmouse overlay             Open the 2-stroke spatial matrix overlay and perform action\n  notmouse test-bench          Open the interactive test bench window alone (Esc to exit)\n  notmouse click <x> <y>       Move to normalized (x, y) and left-click\n  notmouse move <x> <y>        Move to normalized (x, y)\n  notmouse scroll <dy> [x] [y] Scroll vertically (negative=down, positive=up)\n  notmouse demo                Show the terminal matrix demonstration\n  notmouse --version           Show the version",
        env!("CARGO_PKG_VERSION")
    );
}

fn click_at(x: f64, y: f64) -> Result<(), String> {
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
    let mut device = InputDevice::new().map_err(|err| format!("input device error: {err}"))?;
    println!("!mouse: moving to ({x:.3}, {y:.3})");
    device
        .move_to_normalized(x, y)
        .map_err(|err| format!("move failed: {err}"))?;
    thread::sleep(Duration::from_millis(350));
    Ok(())
}

fn scroll_at(steps_y: i32, x: Option<f64>, y: Option<f64>) -> Result<(), String> {
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

    // Initialize virtual input device before launching overlay
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
        if let Ok(event) = serde_json::from_str::<OverlayEvent>(&line) {
            match event {
                OverlayEvent::Move { x, y } => {
                    if let Err(err) = device.move_to_normalized(x, y) {
                        eprintln!("!mouse: live move failed: {err}");
                    }
                    thread::sleep(Duration::from_millis(30));
                }
                OverlayEvent::Scroll { dx, dy } => {
                    if let Err(err) = device.scroll(dy, dx) {
                        eprintln!("!mouse: live scroll failed: {err}");
                    }
                }
                OverlayEvent::Press { button } => {
                    let btn = parse_button(&button);
                    if let Err(err) = device.press(btn) {
                        eprintln!("!mouse: live press failed: {err}");
                    }
                }
                OverlayEvent::Release { button } => {
                    let btn = parse_button(&button);
                    if let Err(err) = device.release(btn) {
                        eprintln!("!mouse: live release failed: {err}");
                    }
                }
                OverlayEvent::Click { button, normalized } => {
                    if let Some(ref p) = normalized {
                        let _ = device.move_to_normalized(p.x, p.y);
                        thread::sleep(Duration::from_millis(30));
                    }
                    if button == "double" || button == "double-click" {
                        let _ = device.double_click(MouseButton::Left);
                    } else {
                        let btn = parse_button(&button);
                        let _ = device.click(btn);
                    }
                }
                OverlayEvent::Selected {
                    strokes,
                    action,
                    normalized,
                } => {
                    selected_event = Some((strokes, action, normalized));
                }
                OverlayEvent::Cancelled => {
                    println!("!mouse: session cancelled");
                }
            }
        } else if !line.trim().is_empty() {
            println!("{line}");
        }
    }

    let status = child
        .wait()
        .map_err(|err| format!("failed to wait on overlay process: {err}"))?;

    if !status.success() {
        return Err(format!("the overlay adapter exited with {status}"));
    }

    if let Some((strokes, action, point)) = selected_event {
        // Wait briefly for Mutter to unmap the overlay and focus the window beneath
        thread::sleep(Duration::from_millis(120));

        println!(
            "!mouse: applying action '{action}' at ({:.3}, {:.3}) (strokes: {strokes})",
            point.x, point.y
        );

        device
            .move_to_normalized(point.x, point.y)
            .map_err(|err| format!("failed to move pointer: {err}"))?;

        // Give compositor brief moment to settle pointer position before button dispatch
        thread::sleep(Duration::from_millis(50));

        execute_action(&mut device, &action)?;

        // Give compositor time to process release event before device teardown
        thread::sleep(Duration::from_millis(350));
    }

    Ok(())
}

fn parse_button(name: &str) -> MouseButton {
    match name {
        "right" | "right-click" => MouseButton::Right,
        "middle" | "middle-click" => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

fn execute_action(device: &mut InputDevice, action: &str) -> Result<(), String> {
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

fn overlay_script() -> PathBuf {
    std::env::var_os("NOTMOUSE_OVERLAY_SCRIPT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../platform/linux/gnome/overlay.js"),
        PathBuf::from,
    )
}

fn test_bench_script() -> PathBuf {
    std::env::var_os("NOTMOUSE_TEST_BENCH_SCRIPT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../platform/linux/gnome/test_bench.js"),
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
    thread::sleep(Duration::from_millis(450));

    println!("!mouse: summoning overlay directly over test bench...");
    let _ = launch_overlay();

    println!("!mouse: test bench running! Inside the window, press Tab to summon !mouse again, or Esc to exit.");
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
    println!("Total reachable target points: {} in 2 keystrokes\n", matrix.all_zones().len());

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
