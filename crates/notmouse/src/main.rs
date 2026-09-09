use std::path::PathBuf;
use std::process::{Command, ExitCode};

use notmouse_core::{Rect, grid};

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
        "!mouse {}\n\nUsage:\n  notmouse overlay    Open the zone overlay\n  notmouse demo       Show the terminal prototype\n  notmouse --version  Show the version",
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
    let zones = grid(
        Rect {
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
        },
        3,
        3,
    );

    println!("!mouse zone prototype — press a hint to choose a region\n");
    for row in zones.chunks(3) {
        println!(
            "  [ {:^3} ]    [ {:^3} ]    [ {:^3} ]",
            row[0].hint, row[1].hint, row[2].hint
        );
    }
}
