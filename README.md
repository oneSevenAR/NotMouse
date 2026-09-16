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
| **GNOME Shell (Wayland / Mutter)** | **Tier 1 (Recommended)** | Fully automated. `install.sh` configures shortcuts, `/dev/uinput` uaccess, and systemd services out of the box. |
| **KDE Plasma (Wayland / KWin)** | Supported (Manual setup) | Register shortcut manually in System Settings → Shortcuts to run `notmouse overlay`. |
| **Sway / Hyprland / wlroots** | Supported (Manual setup) | Add compositor floating window rules (e.g. `for_window [app_id="notmouse"] floating enable`) and keybinding. |
| **X11 Desktops** | Fallback | Fullscreen matrix navigation functions; Wayland is the primary design target. |

### Prerequisites & Dependencies

1. **Kernel `/dev/uinput` Access**:
   `!mouse` creates a virtual input device to dispatch pointer and keyboard events. `install.sh` automatically checks access and installs a standard uaccess udev rule (`/etc/udev/rules.d/99-notmouse.rules`) via `sudo` if permissions are needed.
2. **GTK 4, Cairo & Pango**:
   Native desktop UI runtime and development headers (only needed when compiling from source):
   - Debian / Ubuntu: `sudo apt install libgtk-4-1 libcairo2 libpango-1.0-0` (Build: `libgtk-4-dev libcairo2-dev libpango1.0-dev`)
   - Fedora: `sudo dnf install gtk4 cairo pango` (Build: `gtk4-devel cairo-devel pango-devel`)
   - Arch Linux: `sudo pacman -S gtk4 cairo pango`

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

`install.sh` performs automated preflight checks, builds the release binary, starts the systemd background daemon, and configures the `Super+Shift+M` shortcut in GNOME.

### Installer Flags

- Skip shortcut registration: `./install.sh --no-shortcut`
- Complete uninstallation: `./install.sh --uninstall`

---

## Usage

Press **`Super+Shift+M`** (or run `notmouse overlay`) to summon the overlay.

1. **Stroke 1**: Press a home-row key (`a s d f j k l g h`) to focus a screen zone. Pointer cursor instantly hops to the zone centroid, waking autohiding player overlays and tooltips.
2. **Stroke 2**: Press a second key to lock onto the target sub-cell. Pointer cursor snaps to the cell centroid or nearest detected interactive AT-SPI element.
3. **Action**: Choose an action from below.

### Keybindings Reference

| Key | Mode | Action |
| :--- | :--- | :--- |
| `Enter` / `Space` | Any | Left-click target and dismiss overlay |
| `c` | Grid / Roam | **Click & Stay**: Click target with animated ripple; keep overlay active |
| `p` | Grid / Roam | **Point & Hover**: Dismiss overlay and leave cursor parked without clicking |
| `P` | Grid | **Hover & Stay**: Park cursor, conceal overlay for 120 ms to reveal flyout menus, and re-present with refreshed scan |
| `f` | Grid / Locked | Enter **Free Roam Mode**: 60 Hz kinematic cursor glide |
| `h` `j` `k` `l` / Arrows | Roam / Locked | Kinematic glide / micro-nudge (Tap: 2.5 px; Hold: quadratic acceleration up to 2400 px/s) |
| `Shift` (hold) | Roam / Scroll | 2.5× Turbo speed multiplier |
| `Ctrl` / `Alt` (hold) | Roam | 0.35× Crawl precision dampening |
| `v` | Grid / Roam | **Live Click & Drag / Text Selection**: Emits `BTN_LEFT DOWN`, shrinks to HUD pill (`680×48`), real-time glide, releases on `v`/`Enter` |
| `Tab` / `Shift+Tab` | Grid (Locked) | Cycle through nearest interactive AT-SPI candidates in radius |
| `s` / `w` | Grid | Enter **Instant Scroll Mode** (`s` = down, `w` = up; shrinks overlay to HUD pill) |
| `j` / `k` | Scroll | Continuous kinetic scroll (Down / Up) |
| `d` / `u` | Scroll | Half-page scroll (Down / Up) |
| `h` / `l` | Scroll | Horizontal scroll (Left / Right) |
| `Tab` | Scroll / Roam | Return cleanly from Scroll or Roam back to Fullscreen Grid Mode |
| `r` | Grid / Roam | Right-click target and dismiss overlay |
| `d` | Grid / Roam | Double-click target and dismiss overlay |
| `m` | Grid / Roam | Middle-click target and dismiss overlay |
| `Backspace` | Grid | Undo last key stroke / step back one level |
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

# Launch interactive test bench & warm overlay playground
notmouse playground
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for internal daemon protocols and Wayland event routing.

---

## License

Licensed under the [GNU General Public License v3.0](LICENSE).
