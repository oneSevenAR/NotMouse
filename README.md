# !mouse

[![CI](https://github.com/oneSevenAR/NotMouse/actions/workflows/ci.yml/badge.svg)](https://github.com/oneSevenAR/NotMouse/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/oneSevenAR/NotMouse)](https://github.com/oneSevenAR/NotMouse/releases)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)

A fast, keyboard-first pointer navigation and accessibility layer for Linux desktops (GNOME / Wayland).

`!mouse` lets you target, click, cycle interactive UI elements, and kinetically scroll anywhere on your screen using 2-stroke home-row chords on a transparent spatial grid, eliminating mouse reach.

---

## Features

- **2-Stroke Spatial Matrix**: Reach 81 discrete screen zones instantly using home-row keys (`a s d f j k l g h`).
- **Accessible Element Snapping & Cycling (`Tab` / `Shift+Tab`)**:
  Asynchronously queries the desktop AT-SPI tree to snap reticles directly onto buttons, links, input fields, tabs, and GTK 4 Nautilus rows. Pressing `Tab` cycles through candidates within the target radius with amber badges.
- **Instant Kinetic Scrolling (`s` / `w`)**:
  Transitions the overlay into a compact acrylic HUD pill (`560×48`), unblocking GNOME Mutter pointer focus to route wheel events directly to the underlying window without requiring manual clicks. Kinetic scrolling with `j`/`k`/`d`/`u`/`h`/`l` (`Shift` for 3× speed); press `Tab` to return cleanly to Grid Mode.
- **Click & Stay (`c`)**:
  Execute clicks while keeping the overlay active and responsive with animated neon-green ripple feedback—ideal for navigating menus, checklists, and dense web pages.
- **GNOME Top Bar Mode (`t`)**: Direct targeting for Activities, Clock/Calendar, and Quick Settings.
- **Two-Phase Drag & Drop (`v`)**: Pin a source location and drop at any destination.
- **Pixel Micro-Nudge**: Fine-tune cursor coordinates with `h j k l` or arrow keys (`Shift` for 5× step).
- **Warm Resident Daemon**: Background systemd user daemon maintains persistent uinput device nodes and pre-warmed GTK 4 overlay contexts (<25ms trigger latency).

---

## Platform Support & System Requirements

### Supported Compositors

| Compositor | Support Level | Notes |
| :--- | :--- | :--- |
| **GNOME Shell (Wayland / Mutter)** | **Tier 1 (Recommended)** | Fully automated. `install.sh` configures shortcuts, AT-SPI, and workarea offsets out of the box. |
| **KDE Plasma (Wayland / KWin)** | Supported (Manual setup) | Requires installing `gjs` and `libadwaita-1`. Register shortcut manually in System Settings → Shortcuts. |
| **Sway / Hyprland / wlroots** | Supported (Manual setup) | Requires installing `gjs`. Add compositor floating window rules (e.g. `for_window [app_id="notmouse"] floating enable`) and keybinding. |
| **X11 Desktops** | Fallback | Basic grid navigation functions; Wayland is the primary design target. |

### Prerequisites & Dependencies

1. **Kernel `/dev/uinput` Access**:
   `!mouse` creates a virtual input device to dispatch pointer and keyboard events. `install.sh` automatically checks access and installs a standard uaccess udev rule (`/etc/udev/rules.d/99-notmouse.rules`) via `sudo` if permissions are needed.
2. **GJS & GTK 4**:
   - Debian / Ubuntu: `sudo apt install gjs libgtk-4-1 libadwaita-1-0`
   - Fedora: `sudo dnf install gjs gtk4 libadwaita`
   - Arch Linux: `sudo pacman -S gjs gtk4 libadwaita`
3. **Python 3 & AT-SPI Accessibility Typelibs**:
   Required for interactive UI element scanning (`atspi_scanner.py`):
   - Debian / Ubuntu: `sudo apt install python3-gi gir1.2-atspi-2.0 at-spi2-core`
   - Fedora: `sudo dnf install python3-gobject at-spi2-core`
   - Arch Linux: `sudo pacman -S python-gobject at-spi2-core`

---

## Web Browser Accessibility Setup

- **Firefox**: Enables AT-SPI web-content accessibility automatically when desktop accessibility is active.
- **Chromium / Google Chrome / Brave / Vivaldi / Electron Apps**:
  Chromium-based applications disable web accessibility by default. `install.sh` automatically adds `ACCESSIBILITY_ENABLED=1` to `~/.config/environment.d/99-notmouse.conf` and adds `--force-renderer-accessibility` to `chrome-flags.conf`, `chromium-flags.conf`, and `brave-flags.conf`.
  *(Note: Restart your browser or log out and back in after initial installation for flags to take effect).*

---

## Installation

### Quick Install

```sh
git clone https://github.com/oneSevenAR/NotMouse.git
cd NotMouse
./install.sh
```

`install.sh` performs automated preflight checks, builds the release binary, installs assets to `~/.local/share/notmouse/`, starts the systemd background daemon, and configures the `Super+Shift+M` shortcut in GNOME.

### Installer Flags

- Skip shortcut registration: `./install.sh --no-shortcut`
- Complete uninstallation: `./install.sh --uninstall`

---

## Usage

Press **`Super+Shift+M`** (or run `notmouse overlay`) to summon the overlay.

1. **Stroke 1**: Press a home-row key (`a s d f j k l g h`) to focus a screen zone (or `t` for Top Bar).
2. **Stroke 2**: Press a second key to lock onto the target sub-cell.
3. **Action**: Choose an action from below.

### Keybindings Reference

| Key | Mode | Action |
| :--- | :--- | :--- |
| `Enter` / `Space` | Any | Left-click target and dismiss overlay |
| `c` | Grid / Snap | **Click & Stay**: Click target with animated green ripple; keep overlay active |
| `Tab` / `Shift+Tab` | Grid (Locked) | Cycle through nearest interactive AT-SPI candidates in radius |
| `s` / `w` | Grid | Enter **Instant Scroll Mode** (`s` = down, `w` = up; shrinks overlay to HUD pill) |
| `j` / `k` | Scroll | Continuous kinetic scroll (Down / Up) |
| `d` / `u` | Scroll | Half-page scroll (Down / Up) |
| `h` / `l` | Scroll | Horizontal scroll (Left / Right) |
| `Shift` (hold) | Scroll | 3× kinetic scroll multiplier |
| `Tab` | Scroll | Return cleanly from Scroll Mode back to Fullscreen Grid Mode |
| `r` | Grid / Snap | Right-click target and dismiss overlay |
| `d` | Grid / Snap | Double-click target and dismiss overlay |
| `m` | Grid / Snap | Middle-click target and dismiss overlay |
| `v` | Grid | Two-phase drag & drop (1st press pins source; 2nd press drops target) |
| `t` | Macro | Top Bar mode (`a` = Activities, `s` = Clock/Calendar, `d` = Quick Settings) |
| `h` `j` `k` `l` / Arrows | Locked | Pixel micro-nudge cursor (hold `Shift` for 5× step) |
| `Backspace` | Any | Undo last key stroke / step back one level |
| `Esc` / `q` | Any | Cancel and dismiss overlay |

---

## Known Limitations & Roadmap

- **Multi-Monitor Setups**:
  Pointer coordinates are mapped across the combined virtual desktop canvas. The overlay currently opens on the active monitor surface. Interactive per-monitor overlay hopping via keybinding is planned for `v0.3.0`.
- **Tiling Compositors**:
  In Sway or Hyprland, `!mouse` requires configuring a floating window rule for `app_id = "notmouse"` so the compositor does not tile the overlay as a split window.
- **Sandboxed Applications (Flatpak / Snap)**:
  Strictly sandboxed applications that isolate their D-Bus session bus may prevent AT-SPI from querying their internal widget hierarchy. In these applications, fallback 2-stroke spatial matrix grid navigation continues to work with full precision.

---

## Development

```sh
# Build release binary
cargo build --release --workspace

# Run workspace test suite and linter
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check

# Test overlay self-tests
gjs platform/linux/gnome/overlay.js --self-test

# Launch interactive test bench & warm overlay playground
notmouse playground
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for internal daemon protocols and Wayland event routing.

---

## License

Licensed under the [GNU General Public License v3.0](LICENSE).
