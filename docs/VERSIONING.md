# Versioning and Releases

`!mouse` follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) (`MAJOR.MINOR.PATCH`).

---

## Versioning Policy

During the `0.x` developmental phase:
- **Patch releases** (`0.x.Y`): Compatible bug fixes, platform installer enhancements, and minor performance improvements.
- **Minor releases** (`0.X.0`): New interactive modes, substantial architectural capabilities (e.g. resident daemon architecture, AT-SPI element snapping, instant kinetic scrolling), or protocol modifications.
- **Major releases** (`1.0.0`+): Long-term stable API, mature multi-compositor support, and permanent config schema.

---

## Source of Truth & Release Process

The version declared in the root [`Cargo.toml`](../Cargo.toml) (`workspace.package.version`) is the canonical source of truth for the entire workspace.

A formal release consists of:

1. **Workspace Version Bump**: Updating `Cargo.toml` and syncing `Cargo.lock` via `cargo check --workspace`.
2. **Changelog**: Documenting all additions, fixes, and breaking changes in [`CHANGELOG.md`](../CHANGELOG.md) adhering to [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
3. **Automated Quality Checks**:
   - `cargo test --workspace`
   - `cargo clippy --workspace -- -D warnings`
   - `cargo fmt --check`
4. **Git Annotated Tag**: An annotated tag matching `v<MAJOR>.<MINOR>.<PATCH>` with release summary notes:
   ```sh
   git tag -a v0.2.1 -m "Release v0.2.1: ..."
   git push origin main --tags
   ```
5. **GitHub Release**: Created from the pushed tag with changelog release notes.

---

## Packaging Roadmap

- **One-Line Installer (`install.sh`)**: The primary installation method for Linux users, building from source, configuring systemd user services, udev rules for `/dev/uinput`, and desktop accessibility.
- **Binary Releases & Distro Packaging**:
  - Native `.deb` (Ubuntu / Debian) and `.rpm` (Fedora) packages with automated systemd unit registration.
  - Arch User Repository (`PKGBUILD` for AUR).
  - Standalone release tarballs containing pre-built release binaries and desktop assets.
