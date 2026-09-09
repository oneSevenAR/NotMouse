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
