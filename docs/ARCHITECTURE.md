# Architecture

`!mouse` is split between a platform-neutral interaction core and small desktop
adapters.

## Components

### `notmouse-core`

Owns concepts that should behave consistently on every Linux desktop:

- logical screen rectangles and zones;
- ergonomic, prefix-safe keyboard hints;
- navigation state and actions as the prototype grows.

### `notmouse`

The command-line entry point. It reports the application version and launches
the appropriate desktop adapter. It will later become the long-running daemon.

### GNOME overlay adapter

`platform/linux/gnome/overlay.js` uses the GTK 4 runtime exposed by GJS. It
creates a focused fullscreen surface on Wayland and handles the interactive
zone overlay without requiring GTK development headers at build time.

The current adapter is intentionally a vertical prototype. It supports recursive
zone refinement, backtracking, confirmation, and cancellation. It does not yet
move or click the pointer, register a global shortcut, or coordinate multiple
monitors.

## Near-term flow

1. A GNOME global shortcut invokes the `notmouse` daemon.
2. The daemon asks the adapter to show the overlay.
3. The core supplies zones and keyboard hints.
4. The adapter returns the chosen logical coordinate.
5. A permission-aware Linux input backend performs the requested action.

Keeping input injection out of the overlay is deliberate: Wayland permissions,
GNOME integration, and alternative desktops can evolve independently from the
interaction model.
