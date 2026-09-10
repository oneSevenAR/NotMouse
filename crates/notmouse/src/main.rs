use std::path::PathBuf;
use std::process::{Command, ExitCode};

use notmouse_core::{Rect, SpatialMatrix2D};

fn main() -> ExitCode {
    let command = std::env::args().nth(1);

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
        "!mouse {}\n\nUsage:\n  notmouse overlay    Open the 2-stroke spatial matrix overlay\n  notmouse demo       Show the terminal matrix demonstration\n  notmouse --version  Show the version",
        env!("CARGO_PKG_VERSION")
    );
}

fn launch_overlay() -> Result<(), String> {
    let script = overlay_script();
    if !script.is_file() {
        return Err(format!(
            "overlay adapter was not found at {}",
            script.display()
        ));
    }

    let status = Command::new("gjs")
        .arg(script)
        .status()
        .map_err(|error| format!("could not start the GNOME overlay adapter: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("the overlay adapter exited with {status}"))
    }
}

fn overlay_script() -> PathBuf {
    std::env::var_os("NOTMOUSE_OVERLAY_SCRIPT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../platform/linux/gnome/overlay.js"),
        PathBuf::from,
    )
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
