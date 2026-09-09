use notmouse_core::{Rect, grid};

fn main() {
    let command = std::env::args().nth(1);

    match command.as_deref() {
        Some("demo") => print_demo(),
        Some("--version" | "-V") => println!("notmouse {}", env!("CARGO_PKG_VERSION")),
        _ => print_help(),
    }
}

fn print_help() {
    println!(
        "!mouse {}\n\nUsage:\n  notmouse demo       Show the zone prototype\n  notmouse --version  Show the version",
        env!("CARGO_PKG_VERSION")
    );
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
