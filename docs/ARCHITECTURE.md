# Architecture

`!mouse` is designed around a decoupled, layered architecture: a high-performance platform-neutral interaction core in Rust, a persistent background input daemon managing Linux kernel `uinput` devices, an asynchronous desktop accessibility scanner, and an acrylic GTK 4 Wayland overlay.

```
                  ┌─────────────────────────────────────┐
                  │          Wayland Desktop            │
                  │   (GNOME / Mutter, Sway, etc.)      │
                  └───────────────┬─────────────────────┘
                                  │ Global Shortcut (<Super><Shift>M)
                                  ▼
┌─────────────────────────┐  Unix Socket   ┌───────────────────────────────────┐
│     notmouse overlay    │◄───────────────┤          notmouse daemon          │
│    (GJS / GTK 4 / Adw)  ├───────────────►│        (Persistent Server)        │
└────────────┬────────────┘                └───────────────┬───────────────────┘
             │                                             │
             │ spawns worker                               │ evdev / uinput
             ▼                                             ▼
┌─────────────────────────┐                        ┌───────────────────┐
│    atspi_scanner.py     │                        │    /dev/uinput    │
│  (AT-SPI Accessibility) │                        │ (Virtual Pointer) │
└─────────────────────────┘                        └───────────────────┘
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
- **Resident Socket Server (`session.rs`)**: Listens on `$XDG_RUNTIME_DIR/notmouse.sock` for low-latency (<25ms) input dispatch from the overlay, eliminating device re-binding delays.
- **Foreground App Supervision**: Supervises `atspi_scanner.py --monitor` in the background, listening to desktop `window:activate` events and recording active foreground window PIDs to `$XDG_RUNTIME_DIR/notmouse-active-pid`.

### 3. GNOME / Wayland Overlay (`overlay.js`)
Located in `platform/linux/gnome/overlay.js`. Executed via GJS (GTK 4 / Libadwaita):
- **Fullscreen Spatial Grid**: Presents an interactive transparent grid overlay with dimming backdrop and high-contrast labels.
- **Element Snapping & Badge Cycling**: Integrates with `atspi_scanner.py` to highlight buttons, links, text inputs, and table cells. `Tab` / `Shift+Tab` cycles nearby candidate badges.
- **Instant Kinetic Scroll HUD**: On entering scroll mode (`s`/`w`), dynamically unmaximizes from a fullscreen surface to a compact `560×48` acrylic HUD pill. This unblocks GNOME Mutter's pointer tracker, allowing wheel events to route directly to the underlying application.
- **Multi-Monitor Global Geometry**: Evaluates `Gdk.Display.get_monitor_at_surface()` and computes total desktop bounds across all displays, mapping coordinates accurately to the global virtual canvas.

### 4. Background AT-SPI Scanner (`atspi_scanner.py`)
Located in `platform/linux/gnome/atspi_scanner.py`:
- Connects to the desktop accessibility D-Bus session (`org.a11y.Bus`).
- Traverses widget trees for focused/active application windows.
- Resolves tight inner label bounds for wide GTK 4 / Nautilus column/list containers so click centroids land accurately on file names and icons.

---

## Event & Navigation Lifecycle

1. **Summon**: The user presses `<Super><Shift>M`. GNOME invokes `notmouse overlay`.
2. **Socket Trigger**: The CLI reads the active foreground PID and sends an activation command to the resident `overlay.js` via `$XDG_RUNTIME_DIR/notmouse-overlay.sock`.
3. **Display & Scan**: The overlay renders immediately (<25ms) and asynchronously launches `atspi_scanner.py --pid <PID>` to fetch interactive candidates.
4. **Targeting**:
   - **Stroke 1**: Highlights a macro zone (or `t` for Top Bar).
   - **Stroke 2**: Refines to a micro-zone, snapping to the nearest AT-SPI element if present.
   - **Cycling**: `Tab` cycles through candidates within the target radius.
5. **Action**:
   - **Click (`Enter`/`Space`)**: Dispatches click event to `notmouse.sock` and dismisses overlay.
   - **Click & Stay (`c`)**: Momentarily hides the overlay surface to route pointer focus to the underlying application, dispatches click, displays a neon green ripple, and keeps the overlay active.
   - **Scroll (`s`/`w`)**: Unmaximizes overlay to HUD pill and routes continuous kinetic wheel events (`j`/`k`/`d`/`u`/`h`/`l`). Pressing `Tab` re-maximizes to Grid Mode.
   - **Nudge (`h`/`j`/`k`/`l` / Arrows)**: Shifts normalized reticle coordinates by single-pixel or 5× increments (`Shift`).
