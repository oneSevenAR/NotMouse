# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] — 2026-09-17

### Added

- **100% Pure Rust Native Migration**:
  - Completely eliminated all external JavaScript (`overlay.js` via GJS) and Python (`atspi_scanner.py` via PyGObject).
  - Consolidated daemon, virtual uinput device, transparent Cairo/Pango GTK4 overlay window, and asynchronous AT-SPI scanner into a single, self-contained Rust binary.
  - Resident memory consumption reduced by 85% (from 214 MB down to ~32 MB) with zero child process spawning overhead.
- **Transparent GTK4 Native Overlay Window**:
  - Full-workarea coverage using native GTK 4 `ApplicationWindow` with CSS `background: transparent;` and per-pixel Cairo drawing.
  - Automatically compensates for GNOME Wayland compositor transparency semantics by utilizing maximized surface mode and retaining dedicated Top Bar mode (`t` -> `a`/`s`/`d`) for precision access to system panel status indicators, clock, and dash dock.
- **Live Pointer Coupling ("Wake on Aim")**:
  - Dispatches physical OS cursor coordinates in real-time on chord entry (Stroke 1 zone selection and Stroke 2 cell lock), waking autohiding controls (YouTube player overlays, video controls, tooltips, flyouts) before any click is triggered.
- **Hover Primitives (`p` / `P`)**:
  - `p` (**Point & Hover**): Dismisses the overlay, leaving the cursor parked at the target coordinate without emitting click events.
  - `P` (**Hover & Stay**): Parks cursor at target, briefly conceals overlay (120 ms) to let hover flyout menus and dropdowns reveal themselves, and re-presents overlay with a freshly refreshed element scan.
- **Free Roam Mode with Kinematic Acceleration (`f` / `h j k l`)**:
  - Smooth 60 Hz cursor glide with micro-tap precision (2.5 px micro-steps) and quadratic acceleration ($v_{\min} = 150$, $v_{\max} = 2400$ px/s) on key hold with instant freeze on release.
  - `Shift` Turbo multiplier (2.5×) and `Ctrl`/`Alt` Crawl precision dampening (0.35×).
  - Instant action dispatch from roam: `Enter`/`Space` (click), `c` (click & stay), `r` (right-click), `m` (middle-click), `d` (double-click), `Tab` (return to grid).
- **Live Click & Drag / Text Selection (`v`)**:
  - Emits `BTN_LEFT DOWN` at source anchor and transitions overlay into a compact acrylic HUD pill (`680×48`), allowing Wayland compositor focus to pass through directly to underlying application surfaces for real-time text selection or window dragging.
  - Pressing `v` or `Enter` emits `BTN_LEFT UP` at destination and dismisses the HUD.
- **Pure Rust Asynchronous AT-SPI Element Snapping**:
  - Direct D-Bus session query to `org.a11y.Bus` bypassing connection deadlocks, scanning UI trees in single-digit milliseconds (<15 ms).
  - Instant background system daemon filtering via `/proc/{pid}/comm` (`ibus`, `evolution`, `gpaste`, `gnome-shell`, `xdg-desktop-portal`).
  - Active window scoring and Gecko/LibreWolf window detection, preventing stale target bleeding across application switching.
  - Reading-order row banding, Nautilus file/folder width clamping, and `rstar` R-tree spatial indexing for microsecond candidate lookups and cycling.

### Removed

- Removed `gjs` and Python 3 / PyGObject dependencies and runtime checks.

## [0.2.1] — 2026-09-16

### Added

- **Automated `/dev/uinput` Permission Provisioning**: `install.sh` checks write access to `/dev/uinput` during preflight and automatically configures `/etc/udev/rules.d/99-notmouse.rules` with `TAG+="uaccess"` via `sudo`, ensuring out-of-the-box operation on vanilla Linux installations.
- **Python AT-SPI Typelib Diagnostics**: `install.sh` verifies Python 3 PyGObject and AT-SPI typelib availability, outputting package manager installation commands for Debian/Ubuntu, Fedora, and Arch Linux if missing.
- **Universal Chromium & Electron Web Accessibility Flags**: Installer automatically configures `--force-renderer-accessibility` across `chrome-flags.conf`, `chromium-flags.conf`, and `brave-flags.conf` in addition to systemd session environment.
- **Multi-Monitor Global Geometry Normalization**: Updated `getScreenCoordinates()`, `showOverlay()`, and scroll/nudge handlers in `overlay.js` to dynamically detect the active monitor surface and map coordinates across the full virtual desktop bounding box.
- **Compositor Detection & Configuration Guidance**: Detects desktop environment on install and outputs exact configuration directives for Sway, Hyprland, and KDE Plasma.
- **Documentation Overhaul**: Fully rewritten `README.md` detailing supported compositors, system dependencies, browser setup, complete keybindings reference, and known limitations.

### Fixed

- **Install Binary Collision on Symlinks**: Removed destination binary prior to `install` invocation to avoid same-file collisions when installed via development symlinks.
- **Multi-Monitor Coordinate Drift**: Fixed hardcoded primary monitor references in overlay geometry calculations.

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

[Unreleased]: https://github.com/oneSevenAR/NotMouse/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/oneSevenAR/NotMouse/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/oneSevenAR/NotMouse/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/oneSevenAR/NotMouse/releases/tag/v0.1.0
