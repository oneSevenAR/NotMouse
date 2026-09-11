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
  - `c`: **Click & Stay** (execute click and immediately re-arm overlay for next target)
  - `s` or `w`: **Continuous Scroll Mode** (locks cursor, exposes underlying app, streams `j`/`k`/`d`/`u` scrolls until `Esc`/`Space`)
  - `v`: **Two-Phase Drag Mode** (Phase 1 captures source; re-arms grid to navigate and drop at destination with `v` or `Space`)
  - `r`: Right-click and close
  - `d`: Double-click and close
  - `m`: Middle-click and close
  - `h j k l` or arrow keys: Micro-nudge the reticle pixel-by-pixel (hold `Shift` for larger steps)
  - `Backspace`: Undo last stroke
  - `Esc`: Cancel

### Interactive Playground
To test mouse interactions easily with the overlay directly over a safe target window:

```sh
cargo run -p notmouse -- playground
```
This single command launches the test bench window and immediately summons the overlay on top of it. Inside the test bench, press `Tab` or `F1` anytime to summon the overlay again, and press `Esc` to close the test bench.

You can also launch the components individually:
```sh
cargo run -p notmouse -- overlay      # Just the 2-stroke overlay
cargo run -p notmouse -- test-bench   # Just the interactive test bench
```
The test bench stays open persistently and logs all detected events in real-time. Press `Esc` to close it.

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
