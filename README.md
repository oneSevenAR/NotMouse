# !mouse

[![CI](https://github.com/oneSevenAR/NotMouse/actions/workflows/ci.yml/badge.svg)](https://github.com/oneSevenAR/NotMouse/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/oneSevenAR/NotMouse)](https://github.com/oneSevenAR/NotMouse/releases)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

A fast, keyboard-driven pointer navigation layer for Linux desktops (GNOME/Wayland).

`!mouse` lets you target and click anywhere on your screen using 2-stroke home-row chords on a transparent spatial grid, eliminating mouse reach.

---

## Features

- **2-Stroke Spatial Matrix**: Reach 81 discrete screen zones instantly using home-row keys (`a s d f j k l g h`).
- **GNOME Top Bar Mode (`t`)**: Direct targeting for Activities, Clock/Calendar, and Quick Settings.
- **Click & Stay (`c`)**: Execute clicks without dismissing the overlay for fast multi-click workflows.
- **Kinetic Scrolling (`s` / `w`)**: Expose the active window and scroll smoothly with `j`/`k` and `d`/`u`.
- **Drag & Drop (`v`)**: Pin a source location and drop at any destination.
- **Pixel Nudge**: Fine-tune cursor position with `h j k l` or arrow keys (`Shift` for 5× step).
- **Persistent Daemon**: Background resident service eliminates kernel binding lag (~200ms launch).

---

## Installation

### Prerequisites

- **Linux** (GNOME Wayland recommended)
- **GJS & GTK 4**: `sudo apt install gjs`
- **uinput access**: Your user must belong to the `input` group:
  ```sh
  sudo usermod -aG input $USER
  ```
  *(Log out and back in after running this command)*

### Quick Install

```sh
git clone https://github.com/oneSevenAR/NotMouse.git
cd NotMouse
./install.sh
```

`install.sh` builds the release binary, copies assets to `~/.local/share/notmouse/`, starts the systemd background daemon, and configures the `Super+Shift+M` shortcut in GNOME.

To uninstall:
```sh
./install.sh --uninstall
```

---

## Usage

Press **`Super+Shift+M`** (or run `notmouse overlay`) to summon the overlay.

1. **Stroke 1**: Press a home-row key (`a s d f j k l g h`) to focus a screen zone (or `t` for Top Bar).
2. **Stroke 2**: Press a second key to lock onto the target sub-cell.
3. **Action**: Choose an action from below.

### Keybindings

| Key | Action |
| --- | --- |
| `Enter` / `Space` | Left-click and dismiss |
| `c` | **Click & Stay** (click without dismissing overlay) |
| `r` | Right-click |
| `d` | Double-click |
| `m` | Middle-click |
| `s` / `w` | Enter scroll mode (`j`/`k` = line, `d`/`u` = page, `Esc` = exit) |
| `v` | Two-phase drag & drop (pin source / drop target) |
| `t` | Top Bar mode (`a` = Activities, `s` = Clock, `d` = Quick Settings) |
| `h` `j` `k` `l` / Arrows | Micro-nudge cursor (hold `Shift` for 5× speed) |
| `Backspace` | Undo last stroke |
| `Esc` | Cancel / dismiss overlay |

---

## Development

```sh
# Build release
cargo build --release

# Run tests and linter
cargo test --workspace
cargo clippy --all-targets -- -D warnings
cargo fmt --check

# Test overlay and events interactively
cargo run -p notmouse -- playground
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for internal design and Wayland considerations.

---

## License

Licensed under the [GNU General Public License v3.0](LICENSE).
