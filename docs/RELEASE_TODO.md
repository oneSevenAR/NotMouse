# !mouse - Release Readiness To-Do List

**Target Version:** `v0.1.0` (or `v0.1.0-alpha.2`)  
**Status:** Core features verified, functional, and committed on `main`. Remaining tasks focus on packaging, distribution, and documentation polish.

---

## 1. High-Priority Packaging & Distribution

- [x] **Self-Contained Script & Asset Resolution**
  - **Issue:** Currently, `crates/notmouse/src/main.rs` looks for `overlay.js` via `env!("CARGO_MANIFEST_DIR")`. If a user installs via `cargo install --path crates/notmouse` or downloads a binary release, it fails unless the development repo is cloned.
  - **Action Plan:**
    1. Update `overlay_script()` to search in standard hierarchical order:
       - `NOTMOUSE_OVERLAY_SCRIPT` environment variable (override).
       - Executable sibling directory: `<exe_dir>/overlay.js`.
       - User data directory: `~/.local/share/notmouse/overlay.js` (`$XDG_DATA_HOME/notmouse/overlay.js`).
       - System data directory: `/usr/local/share/notmouse/overlay.js` and `/usr/share/notmouse/overlay.js`.
       - Development fallback: `CARGO_MANIFEST_DIR/../../platform/linux/gnome/overlay.js`.
    2. Embed fallback script into binary with `include_str!("../../platform/linux/gnome/overlay.js")` and write to `~/.local/share/notmouse/overlay.js` on first run if no file exists.
    3. Apply the same resolution logic to `test_bench_script()`.

- [x] **Repository Systemd Service File**
  - **Issue:** The running systemd service was installed manually into `~/.config/systemd/user/notmouse.service` and is not tracked in git.
  - **Action Plan:**
    - Create `platform/linux/systemd/notmouse.service` using standard specifiers:
      ```ini
      [Unit]
      Description=!mouse Persistent Input Daemon
      PartOf=graphical-session.target
      After=graphical-session.target

      [Service]
      ExecStart=%h/.local/bin/notmouse daemon
      Restart=on-failure
      RestartSec=2s

      [Install]
      WantedBy=graphical-session.target
      ```

- [x] **One-Line Installer Script (`install.sh`)**
  - **Issue:** Setting up !mouse currently requires several manual steps (compilation, symlinking, systemd enablement, GNOME shortcut registration).
  - **Action Plan:**
    - Create `install.sh` in the repository root that:
      1. Builds the release binary (`cargo build --release`).
      2. Installs binary to `~/.local/bin/notmouse`.
      3. Installs `overlay.js` to `~/.local/share/notmouse/overlay.js`.
      4. Installs and starts `notmouse.service` via `systemctl --user enable --now notmouse.service`.
      5. Optionally registers the GNOME custom shortcut (`<Super><Shift>M`) via `gsettings`.

---

## 2. Licensing & Cargo Metadata

- [x] **License Definition**
  - **Issue:** Root and crate `Cargo.toml` files do not declare a `license` field, and no `LICENSE` file exists in the repository.
  - **Action Plan:**
    - Choose standard open-source license (e.g. `MIT OR Apache-2.0` or `GPL-3.0`).
    - Add `license = "MIT OR Apache-2.0"` to `[workspace.package]` in root `Cargo.toml`.
    - Add `LICENSE-MIT` and `LICENSE-APACHE` files to repository root.

- [x] **Crate Manifest Polish**
  - Ensure crates have complete metadata:
    - `description`: Polish description for `crates.io` publishing.
    - `keywords`: e.g. `["accessibility", "keyboard", "mouse", "wayland", "gnome"]`.
    - `categories`: `["command-line-utilities", "accessibility"]`.
    - `readme = "../../README.md"` linked properly.

---

## 3. Documentation & Changelog

- [x] **Update `README.md`**
  - Document all current capabilities:
    - **Top Bar Mode (<kbd>t</kbd>):** Dedicated quick targets for Activities (<kbd>a</kbd>), Clock/Date (<kbd>s</kbd>), Quick Settings (<kbd>d</kbd>).
    - **Click & Stay (<kbd>c</kbd>):** Momentary unmap multi-click workflow across application windows.
    - **Continuous Kinetic Scroll (<kbd>s</kbd> / <kbd>w</kbd>):** Real-time interactive scrolling.
    - **Two-Phase Drag & Drop (<kbd>v</kbd>):** Source pinning and target release.
    - **Micro-Nudging (<kbd>h</kbd><kbd>j</kbd><kbd>k</kbd><kbd>l</kbd>):** Directional fine-tuning.
    - **Resident Daemon & Shortcut Setup:** How to configure `notmouse.service` and the `<Super><Shift>M` shortcut.
    - **Prerequisites:** Note `/dev/uinput` group permissions (`input` / `uinput` group) and `gjs` / GTK 4 dependencies.

- [x] **Update `CHANGELOG.md`**
  - Move unreleased features into a dedicated `## [0.1.0] - 2026-09-12` section.
  - Detail resident daemon architecture, Wayland unredirection transparency fixes, and momentary unmap pass-through.

---

## 4. Continuous Integration (CI)

- [x] **Add GitHub Actions Workflow (`.github/workflows/ci.yml`)**
  - Automate on `push` and `pull_request` against `main`:
    - `cargo fmt --check`
    - `cargo clippy --all-targets -- -D warnings`
    - `cargo test --workspace`
    - `gjs platform/linux/gnome/overlay.js --self-test`

---

## 5. Release Tagging

- [x] **Git Tag & Release Creation (v0.1.0)**
  - Create annotated git tag: `git tag -a v0.1.0 -m "Release v0.1.0: Keyboard-first interaction layer for Linux"`
  - Push tag: `git push origin v0.1.0`
  - Generate GitHub Release with changelog notes and binary artifacts.

---

## 6. Active Bugs & Next Milestones (v0.2.0)

- [ ] **Release v0.2.0 Milestone Tagging**
  - Consolidate merged features since v0.1.0 (PR #5, PR #6, PR #7, PR #9):
    - Sub-25ms resident warm overlay architecture (`notmouse-overlay.sock`, async scanner)
    - Strict cell candidate bounds for snapping
    - Click & Stay (`c`) target preservation + Mutter remap + neon green ripple animation
    - Reading order sort with row banding
    - Curated link support (heuristics for nav links / tabs with amber badges)
  - Bump workspace version to `0.2.0` in `Cargo.toml`.
  - Update `CHANGELOG.md` with `## [0.2.0]` section.
  - Tag and push `v0.2.0` on GitHub.

- [x] **Scroll Mode Wayland Pointer Focus on Window Switch ([Issue #4](https://github.com/oneSevenAR/NotMouse/issues/4))**
  - **Symptom:** When Alt+Tabbing to a window (e.g. Vivaldi on Reddit) and entering scroll mode (`s`), scroll inputs either take time to kick in or fail entirely until the user manually clicks somewhere on the page.
  - **Root Cause:** In GNOME Mutter on Wayland, virtual uinput scroll events (`REL_WHEEL`) are only dispatched to the surface that currently holds Wayland pointer focus. Moving the virtual cursor via `ABS_X`/`ABS_Y` under an empty input region does not force Mutter to transfer pointer focus to an unfocused window without relative motion or pointer interaction.
  - **Fix Implemented:**
    1. Added `REL_X` and `REL_Y` axes to `VirtualDevice` in `crates/notmouse/src/input.rs`.
    2. Implemented zero-net-delta relative twitch (`+1` then `-1` px) in `move_to_normalized` and `poke_pointer_focus`. Relative events are never deduplicated by kernel `evdev`, forcing Mutter to re-evaluate actor pick and dispatch `wl_pointer.enter`.
    3. In `overlay.js`, added `ensureScrollFocused(state, window)` to guarantee cursor position and pointer focus are committed before any scroll keystroke (`j`, `k`, `h`, `l`, etc.) is dispatched.

- [x] **Cross-Application Element Bleeding in AT-SPI Scanner ([Issue #11](https://github.com/oneSevenAR/NotMouse/issues/11))**
  - **Symptom:** On Reddit in Vivaldi, cycling through snap candidates in a region shows buttons/tabs from LibreWolf (which is open in the background).
  - **Root Cause:** In `atspi_scanner.py`, `active_frames` merged frames from every application reporting `ACTIVE`, and the fallback appended every window on the desktop. In Gecko (LibreWolf/Firefox), `StateType.ACTIVE` remains permanently `True` on the top-level frame even when backgrounded.
  - **Fix Implemented:**
    1. Replaced multi-app frame merging with an application priority ranking engine (`FOCUSED` descendant +150, `ACTIVE` frame +100, target bounds +50, desktop z-order index) that strictly selects ONE single foreground application.
    2. Converted element coordinate extraction from `CoordType.WINDOW` to absolute screen coordinates (`fx + bx, fy + by`) based on top-level window extents.
    3. Tagged every element with `app_name` and `app_pid`.
    4. In `overlay.js` `snapToNearestElement`, filtered candidates strictly to the cell's primary application.

