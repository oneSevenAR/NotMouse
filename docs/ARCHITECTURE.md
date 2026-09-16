# Architecture

`!mouse` is built around a 100% pure Rust architecture: a high-performance platform-neutral interaction core, a persistent background input daemon managing Linux kernel `uinput` devices, an asynchronous desktop accessibility scanner communicating directly with the AT-SPI D-Bus bus, and a native acrylic GTK 4 Wayland overlay.

```
                  ┌─────────────────────────────────────────┐
                  │            Wayland Desktop              │
                  │       (GNOME / Mutter, Sway, etc.)      │
                  └────────────────────┬────────────────────┘
                                       │ Global Shortcut (<Super><Shift>M)
                                       ▼
┌─────────────────────────────┐   Unix Socket   ┌───────────────────────────────────┐
│       notmouse overlay      │◄────────────────┤          notmouse daemon          │
│     (Native Rust / GTK 4)   ├────────────────►│        (Persistent Server)        │
└──────────────┬──────────────┘                 └───────────────┬───────────────────┘
               │                                                │
               │ in-process async D-Bus                         │ evdev / uinput
               ▼                                                ▼
┌─────────────────────────────┐                         ┌───────────────────┐
│     Pure Rust AT-SPI        │                         │    /dev/uinput    │
│ (atspi / zbus async client) │                         │ (Virtual Pointer) │
└─────────────────────────────┘                         └───────────────────┘
```

---

## Components

### 1. `notmouse-core` (Rust Crate)
Located in `crates/notmouse-core/`. Pure, zero-dependency library handling:
- **Spatial Matrix Geometry**: Recursive division of screen bounds into 9 macro-zones and 81 micro-zones.
- **Key Chord Mapping**: Ergonomic home-row key chords (`a s d f j k l g h`) for unambiguous 2-stroke navigation.
- **Normalization & Math**: Bounding box arithmetic, centroid resolution, and coordinate clamping.

### 2. `notmouse` Daemon & CLI (Rust Binary)
Located in `crates/notmouse/`. The systemd user service and command-line entry point:
- **Virtual Input Device (`input.rs`)**: Creates and persists a Linux kernel `uinput` device (`VirtualDevice`) supporting absolute pointer coordinates (`ABS_X`, `ABS_Y`), relative motion twitches (`REL_X`, `REL_Y`), mouse clicks (left, right, middle, double-click), and kinetic wheel events (`REL_WHEEL`, `REL_HWHEEL`).
- **Resident Socket Server (`session.rs`)**: Listens on `$XDG_RUNTIME_DIR/notmouse.sock` for low-latency input dispatch from the overlay, eliminating device re-binding delays.
- **Pre-Warmed Native Overlay**: Supervises a resident warm overlay process (`notmouse overlay --resident`), listening on `$XDG_RUNTIME_DIR/notmouse-overlay.sock` for sub-millisecond summon response.

### 3. Native GTK 4 Wayland Overlay (`overlay/`)
Located in `crates/notmouse/src/overlay/`. 100% pure Rust GTK 4 application:
- **True Fullscreen Display Coverage**: Calls `window.fullscreen()` to cover 100% of the display including top bars, eliminating workarea offsets and top-bar special modes.
- **Live Pointer Coupling ("Wake on Aim")**: Real-time OS cursor synchronization as chords are entered, waking autohiding UI (e.g. YouTube controls, tooltips, flyouts) before clicking.
- **Free Roam Mode with Kinematic Acceleration (`kinematics.rs`)**: Smooth 60Hz cursor glide with tap micro-steps (2.5px), smooth quadratic acceleration, turbo multiplier (`Shift`), precision crawl (`Ctrl`/`Alt`), and instant freeze on release.
- **Interactive Primitives (`p`, `P`, `v`)**: Point & Hover (`p`), Hover & Stay (`P`), and Live Click & Drag (`v`) with focus yielding.
- **Instant Kinetic Scroll HUD**: On entering scroll mode (`s`/`w`), transforms into a compact `560×48` acrylic HUD pill. Wheel events route directly to the underlying application.

### 4. Pure Rust AT-SPI Accessibility Client (`atspi.rs`)
Located in `crates/notmouse/src/atspi.rs`:
- Connects directly to the accessibility D-Bus session (`org.a11y.Bus` / `/run/user/1000/at-spi/bus`) using pure Rust `zbus` and `atspi` crates without external Python or C dependencies.
- Subtree bounding box pruning: skips recursing into off-screen or disjoint UI subtrees, yielding sub-millisecond targeted scans.
- Reading-order sorting with row banding (14px): controls before links, left-to-right.
- In-memory spatial index (`rstar`) for instantaneous point and micro-cell snapping queries.

---

## Event & Navigation Lifecycle

1. **Summon**: The user presses `<Super><Shift>M`. GNOME invokes `notmouse overlay`.
2. **Socket Trigger**: The CLI sends an activation command to the resident overlay via `$XDG_RUNTIME_DIR/notmouse-overlay.sock`.
3. **Display & Scan**: The overlay renders immediately (<5ms) and asynchronously scans the active application via AT-SPI in the background.
4. **Targeting**:
   - **Stroke 1**: Real cursor moves to macro-zone center immediately ("Wake on Aim").
   - **Stroke 2**: Real cursor snaps to the micro-zone center or nearest accessible element.
   - **Cycling**: `Tab` / `Shift+Tab` cycles through candidates in reading order; real cursor moves to each candidate.
5. **Interactive Primitives**:
   - **Click (`Enter`/`Space`)**: Dispatches click event to `notmouse.sock` and dismisses overlay.
   - **Point & Hover (`p`)**: Moves cursor to target point and dismisses overlay without clicking.
   - **Hover & Stay (`P`)**: Moves cursor, temporarily yields focus for 100ms so menus open, then stays active.
   - **Click & Drag (`v`)**: Presses `BTN_LEFT`, switches to Free Roam HUD for live highlight gliding, and releases on `v` / `Enter`.
   - **Free Roam (`f`)**: Switches to acrylic glide HUD; navigate smoothly with `h`/`j`/`k`/`l` with quadratic acceleration.
   - **Click & Stay (`c`)**: Momentarily hides overlay surface, dispatches click, displays neon ripple, and keeps overlay active.
   - **Scroll (`s`/`w`)**: Unfullscreens overlay to compact HUD pill and routes continuous kinetic wheel events (`j`/`k`/`d`/`u`/`h`/`l`). Pressing `Tab` returns to Grid Mode.
   - **Nudge (`h`/`j`/`k`/`l` / Arrows)**: Shifts reticle coordinates and OS cursor position.
