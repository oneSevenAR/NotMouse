# !mouse

An always-on, keyboard-first interaction layer for Linux desktops.

`!mouse` aims to reduce mouse dependence with accessible screen zones, short
keyboard chords, semantic UI targeting, and automatable workflows.

## Status

Private early development. The first milestone is a Linux proof of concept for
invoking a keyboard overlay and navigating screen zones without a mouse.

The project currently contains the platform-neutral zone and hint engine plus a
small terminal demo. The first supported desktop target will be Ubuntu GNOME on
Wayland.

## Try the current prototype

On GNOME with GTK 4 available through GJS, launch the fullscreen 2-stroke matrix overlay:

```sh
cargo run -p notmouse -- overlay
```

- **Stroke 1:** Press any home-row key (`a s d f j k l g h`) to focus that macro region.
- **Stroke 2:** Press the second key to lock the target reticle onto that sub-cell.
- **Actions & Nudge:**
  - `Space` / `Enter`: Left-click and close
  - `r`: Right-click and close
  - `d`: Double-click and close
  - `m`: Middle-click and close
  - `v`: Drag lock
  - `s`: Scroll mode
  - `h j k l` or arrow keys: Micro-nudge the reticle pixel-by-pixel (hold `Shift` for larger steps)
  - `Backspace`: Undo last stroke
  - `Esc`: Cancel

The terminal matrix demonstration is also available:

```sh
cargo run -p notmouse -- demo
```

Run the checks with:

```sh
cargo test --workspace
```

## Versioning

The project follows Semantic Versioning and is currently in the experimental
`0.x` series. See [docs/VERSIONING.md](docs/VERSIONING.md) for the release and
package policy.

The component boundaries and current Wayland limitations are documented in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
