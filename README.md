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

On GNOME with GTK 4 available through GJS, launch the fullscreen zone overlay:

```sh
cargo run -p notmouse -- overlay
```

Use the displayed home-row key to narrow the selected region. `Backspace` moves
up one level, `Enter` confirms the center of the current region, and `Esc` exits.

The terminal-only demonstration remains available:

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
