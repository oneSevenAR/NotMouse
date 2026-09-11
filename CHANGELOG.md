# Changelog

All notable changes to this project will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial Rust workspace.
- Platform-neutral screen-zone generation.
- Ergonomic keyboard hint generation.
- Terminal demonstration of a 3-by-3 navigation grid.
- Fullscreen GTK 4 zone overlay adapter for GNOME.
- Recursive home-row zone selection with backtracking and cancellation.
- 2-stroke spatial matrix engine in `notmouse-core` with 81 distinct home-row targets.
- Interactive 2-stroke overlay adapter in GNOME with simultaneous macro and micro hint previews.
- Lock & Action mode on second stroke: renders target reticle and stays open for confirmation.
- Pixel-by-pixel directional nudging (`h j k l` / arrows) with `Shift` acceleration.
- Dedicated action triggers: `Space`/`Enter` (click), `r` (right-click), `d` (double-click), `m` (middle-click), `v` (drag), `s` (scroll), `Backspace` (undo), and `Esc` (cancel).
- Linux `uinput` virtual pointer backend in `notmouse` supporting absolute positioning, relative motion, button clicks, drag-lock, and wheel scrolling.
- Direct execution pipeline connecting overlay selection events to hardware mouse events.
- CLI subcommands for manual testing: `notmouse click <x> <y>` and `notmouse move <x> <y>`.

[Unreleased]: https://github.com/oneSevenAR/NotMouse/compare/v0.1.0-alpha.1...HEAD
