# !mouse

An always-on, keyboard-first interaction layer for Linux desktops.

`!mouse` lets you move the pointer and click anywhere on screen using short keyboard chords — no mouse required. Two strokes resolve to any of 81 positions on a transparent overlay; semantic shortcuts, scroll, drag, and click-and-stay workflows handle the rest.

---

## Install

```sh
git clone https://github.com/oneSevenAR/NotMouse.git
cd NotMouse
./install.sh
```

`install.sh` will:
1. Build the release binary and install it to `~/.local/bin/notmouse`
2. Install `overlay.js` to `~/.local/share/notmouse/`
3. Enable and start the persistent `notmouse.service` daemon
4. Register `Super+Shift+M` as a GNOME custom keyboard shortcut

Pass `--no-shortcut` to skip step 4, or `--uninstall` to reverse everything.

### Prerequisites

| Requirement | Notes |
|---|---|
| Rust ≥ 1.91 | Install via [rustup](https://rustup.rs) |
| GJS + GTK 4 | `sudo apt install gjs` (Ubuntu 22.04+) |
| `/dev/uinput` access | Add yourself to the `input` group: `sudo usermod -aG input $USER` (then log out/in) |

---

## Quick Start

Once installed, press **`Super+Shift+M`** anywhere on the GNOME desktop.

The transparent overlay appears in under 200 ms. Use two keystrokes to target any screen region:

- **Stroke 1** — Pick a macro zone (home-row keys: `a s d f j k l g h`)
- **Stroke 2** — Pick the sub-cell within that zone

The crosshair locks onto the resolved position. Then act:

| Key | Action |
|---|---|
| `Enter` / `Space` | Left-click and dismiss |
| `c` | **Click & Stay** — click without dismissing; overlay re-arms for the next target |
| `r` | Right-click and dismiss |
| `d` | Double-click and dismiss |
| `m` | Middle-click and dismiss |
| `s` / `w` | **Scroll Mode** — expose the underlying window and stream scroll events (`j`/`k` = line, `d`/`u` = page) until `Esc` or `Space` |
| `v` | **Drag Mode** — Phase 1 pins source; re-arm grid to navigate to destination; `v` or `Space` releases |
| `h j k l` / arrows | Micro-nudge the crosshair (hold `Shift` for ×5 steps) |
| `Backspace` | Undo last stroke, return to Stroke 1 |
| `Esc` | Cancel and dismiss |

---

## Features

### Two-Stroke Spatial Matrix

The screen is divided into a 9-zone macro grid. Each macro zone contains a 9-zone micro grid — giving 81 reachable target points in exactly 2 keystrokes, all from the home row.

```
Stroke 1: a s d f j k l g h   → choose macro zone
Stroke 2: a s d f j k l g h   → choose sub-cell → crosshair locks
```

### Top Bar Mode (`t`)

Press `t` (or `` ` ``) during Stroke 1 to enter dedicated GNOME top-bar targeting:

| Key | Target |
|---|---|
| `a` | Activities button |
| `s` | Clock / Calendar |
| `d` | Quick Settings / Wi-Fi indicator |

Double-tap to click (e.g. `d` selects, `d` again clicks). `h`/`l` nudges horizontally. `Enter` clicks. `Tab` returns to the grid.

### Click & Stay (`c`)

After locking a target, press `c` to click *without* dismissing the overlay. The overlay momentarily unmaps (yielding Wayland focus to the underlying window), delivers the click, then re-presents and re-arms so you can target the next element immediately. Useful for filling forms, navigating menus, and any multi-click workflow.

### Continuous Kinetic Scroll (`s` / `w`)

In locked mode, `s` or `w` exposes the underlying window and enters scroll mode. `j`/`k` scroll by line, `d`/`u` by page. `Esc` or `Space` exits scroll mode and dismisses the overlay.

### Two-Phase Drag & Drop (`v`)

`v` in locked mode pins the source position (mouse-down). The overlay re-arms so you can navigate to the drop target. `v` or `Space` at the destination releases the drag. Works for file manager drag-and-drop and in-app reordering.

### Micro-Nudging (`h j k l`)

After locking, fine-tune the crosshair one pixel at a time with vim-style keys or arrow keys. Hold `Shift` for 5× steps. Works in both grid mode and top-bar mode.

---

## Resident Daemon

The `notmouse.service` systemd user service keeps a persistent virtual input device bound to the compositor at all times. This eliminates the ~2.5 s kernel binding delay on first use, reducing overlay launch latency to **~200 ms**.

```sh
# Check daemon status
systemctl --user status notmouse.service

# Restart after binary update
systemctl --user restart notmouse.service
```

The daemon communicates with `notmouse overlay` via a Unix socket at `$XDG_RUNTIME_DIR/notmouse.sock`. If the daemon is not running, `notmouse overlay` falls back to a standalone device automatically.

---

## Development

```sh
# Run from source (dev fallback path active)
cargo run -p notmouse -- overlay

# Interactive playground (test bench + overlay together)
cargo run -p notmouse -- playground

# Terminal matrix demonstration
cargo run -p notmouse -- demo

# Tests
cargo test --workspace

# Lint
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

The GJS overlay adapter supports a self-test mode:

```sh
gjs platform/linux/gnome/overlay.js --self-test
```

---

## Architecture

```
notmouse (binary)
├── main.rs          CLI dispatch, overlay launch, script resolution
├── session.rs       Unix socket daemon / client
└── input.rs         evdev virtual device (uinput)

notmouse-core (library)
└── lib.rs           Platform-neutral spatial matrix engine (no unsafe, no I/O)

platform/linux/gnome/
├── overlay.js       GTK 4 / GJS transparent fullscreen overlay
└── test_bench.js    Interactive test-bench window

platform/linux/systemd/
└── notmouse.service Systemd user service unit
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for component boundaries and Wayland constraints.

---

## Versioning

The project follows Semantic Versioning. See [docs/VERSIONING.md](docs/VERSIONING.md) for the release policy.

## License

Licensed under either of

- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
