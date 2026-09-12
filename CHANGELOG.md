# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-09-12

### Added

- **Resident Input Daemon** (`notmouse daemon` / `notmouse.service`): persistent Unix-socket server that holds the virtual input device bound to the compositor across overlay invocations, reducing launch latency from ~2.5 s to ~200 ms.
- **Top Bar Mode** (`t` or `` ` `` on Stroke 1): dedicated quick-target mode for the GNOME panel with three named slots — `a` = Activities, `s` = Clock/Calendar, `d` = Quick Settings. Double-tap a key to confirm, `Enter` to click, `Tab` to return to grid.
- **Click & Stay** (`c` in locked mode): momentary unmap approach cedes Wayland focus to the underlying window, fires a click, then re-presents the overlay — enabling seamless multi-click workflows without dismissing.
- **Workarea offset compensation** in `getScreenCoordinates()`: dynamically accounts for the 29 px GNOME panel height so coordinates map correctly to the full monitor.
- **Upward nudge into top bar**: `k`/`↑` in locked mode can drive the crosshair above the workarea boundary and into the 29 px GNOME panel.
- **Self-contained script resolution**: `overlay.js` and `test_bench.js` are embedded in the binary via `include_str!()` and automatically extracted to `~/.local/share/notmouse/` on first run if not found in any standard installation location.
- **One-line installer** (`install.sh`): builds the release binary, installs assets, registers the systemd user service, and optionally sets the GNOME keyboard shortcut via `gsettings`. Supports `--no-shortcut` and `--uninstall`.
- **Canonical systemd service** (`platform/linux/systemd/notmouse.service`): tracked in-repo with `%h` home-dir specifier; tied to `graphical-session.target`.
- **MIT and Apache-2.0 dual license** (`LICENSE-MIT`, `LICENSE-APACHE`).
- Initial Rust workspace with platform-neutral screen-zone generation.
- Ergonomic keyboard hint generation.
- Terminal demonstration of the 3×3 navigation grid (`notmouse demo`).
- Fullscreen GTK 4 / GJS zone overlay adapter for GNOME Wayland.
- Recursive home-row zone selection with backtracking and cancellation.
- 2-stroke spatial matrix engine in `notmouse-core` with 81 distinct home-row targets.
- Interactive 2-stroke overlay with simultaneous macro and micro hint previews.
- Lock & Action mode on second stroke: renders target reticle and stays open for confirmation.
- Pixel-by-pixel directional nudging (`h j k l` / arrows) with `Shift` acceleration.
- Action triggers: `Space`/`Enter` (left-click), `r` (right-click), `d` (double-click), `m` (middle-click), `v` (drag), `s`/`w` (scroll), `Backspace` (undo), `Esc` (cancel).
- Linux `uinput` virtual pointer backend supporting absolute positioning, relative motion, button clicks, drag-lock, and wheel scrolling.
- CLI subcommands: `notmouse click <x> <y>`, `notmouse move <x> <y>`, `notmouse scroll <dy> [x] [y]`.
- Interactive GTK 4 test bench window (`notmouse test-bench`) for safely observing click, drag, and scroll events in real-time.
- Unified `notmouse playground` command launching test bench and overlay together in one step.
- `Tab` / `F1` inside the test bench immediately re-summons the overlay.

### Fixed

- **Wayland compositor transparency**: replaced `window.fullscreen()` with `window.maximize()` — fullscreen triggers Mutter's unredirection path, causing a solid black screen. Maximize preserves alpha compositing on the transparent overlay.
- **Test socket isolation**: `test_server_lifecycle_and_event` no longer kills the production daemon socket; tests now use a per-PID path via `start_server_at(device, path)` / `try_connect_at(path)`.

### Changed

- Decreased overlay grid opacity by over 70% for clearer visibility of underlying windows and text.

---

[Unreleased]: https://github.com/oneSevenAR/NotMouse/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/oneSevenAR/NotMouse/releases/tag/v0.1.0
