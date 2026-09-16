# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] — 2026-09-16

### Added

- **Instant Kinetic Scroll Mode**: Entering scroll mode (`s` or `w`) immediately transitions the overlay window from a maximized surface to a compact acrylic HUD pill (`560×48`), unblocking GNOME Mutter's pointer tracker and allowing wheel events (`REL_WHEEL` / `REL_HWHEEL`) to reach the underlying target window in <1 ms without requiring a physical click.
- **Scroll HUD Collision Avoidance**: Automatically detects if the reticle target coordinate is covered by the centered HUD pill and nudges cursor placement vertically (`0.56` or `0.44`) so pointer focus is guaranteed to land on the underlying viewport.
- **Immediate Scroll Step**: Emits an initial scroll step on the entry keystroke itself (`s` for down, `w` for up), providing instantaneous visual feedback (<50 ms).
- **Interactive Continuous Scrolling**: Continuous kinetic scrolling with `j`/`k`/`d`/`u`/`h`/`l` and `Shift` multiplier (3× speed).
- **Bidirectional Scroll Roundtrip**: Pressing `Tab` from scroll mode cleanly re-maximizes the overlay back to Grid Mode. Pressing `Enter` / `Space` clicks at the scroll target and dismisses the overlay.
- **Accessible Element Snapping & Cycling**: Integrated asynchronous AT-SPI background accessibility scanner (`atspi_scanner.py`) to detect interactive controls (buttons, links, text fields, menu items, table cells, tabs).
- **Candidate Cycling with `Tab`**: Pressing `Tab` or `Shift+Tab` cycles through nearest interactive elements within the lock radius, rendering amber badges and reticle snaps.
- **GTK 4 / Nautilus Table & List Item Refinement**: Added recursive inner label resolution (`find_inner_label_bounds`) for wide GTK 4 `GtkColumnView` / `GtkListView` row containers, centering click centroids on file names and icons instead of blank container margins.
- **Active Foreground Window Tracking**: Resident daemon supervises `atspi_scanner.py --monitor` in the background, listening to `window:activate` and `object:state-changed:active` to record active window PIDs to `$XDG_RUNTIME_DIR/notmouse-active-pid` and isolate element scanning strictly to the active app.
- **Desktop Accessibility Auto-Configuration**: `install.sh` enables `org.gnome.desktop.interface toolkit-accessibility true` and sets `ACCESSIBILITY_ENABLED=1` in `~/.config/environment.d/` for automatic on-demand accessibility support in Chromium, Vivaldi, Electron, and Qt applications.

### Fixed

- **Wayland Maximized Input Interception**: Solved the issue where maximized windows in GTK 4 on Wayland intercept all wheel events regardless of `gdk_surface_set_input_region` empty region calls.
- **Resident Socket Broken Pipe Resilience**: `sendEvent()` in `overlay.js` catches socket write errors and broken pipes, automatically reconnecting to `notmouse.sock` and retrying delivery.
- **Wayland Zero-Delta Motion Deduplication**: `InputDevice::move_to_normalized` in `crates/notmouse/src/input.rs` emits a 1-unit motion nudge when coordinates are identical, ensuring `libinput` and Mutter never suppress cursor repositioning.
- **AT-SPI Focus Inversion**: Fixed window activation timing where overlay presentation stole focus before accessibility scanning.
- **Single Page Application (SPA) Transition Stale Cache**: Invalidate element cache on click-and-stay navigation link activation with settling delay.

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

[Unreleased]: https://github.com/oneSevenAR/NotMouse/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/oneSevenAR/NotMouse/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/oneSevenAR/NotMouse/releases/tag/v0.1.0
