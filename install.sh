#!/usr/bin/env bash
# install.sh — !mouse installer
#
# Builds the release binary, installs assets to XDG-standard locations,
# registers the systemd user service, and optionally registers the GNOME
# keyboard shortcut.
#
# Usage:
#   ./install.sh              # full install
#   ./install.sh --no-shortcut  # skip GNOME shortcut registration
#   ./install.sh --uninstall    # remove everything installed by this script

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_NAME="notmouse"
BIN_DEST="${HOME}/.local/bin/${BIN_NAME}"
ASSET_DEST="${XDG_DATA_HOME:-${HOME}/.local/share}/${BIN_NAME}"
SERVICE_SRC="${REPO_ROOT}/platform/linux/systemd/${BIN_NAME}.service"
SERVICE_DEST="${HOME}/.config/systemd/user/${BIN_NAME}.service"
SHORTCUT_KEY="<Super><Shift>m"
SHORTCUT_NAME="!mouse overlay"
SHORTCUT_CMD="${BIN_DEST} overlay"

# ── helpers ──────────────────────────────────────────────────────────────────

info()  { printf '\e[1;34m  •\e[0m %s\n' "$*"; }
ok()    { printf '\e[1;32m  ✓\e[0m %s\n' "$*"; }
warn()  { printf '\e[1;33m  !\e[0m %s\n' "$*" >&2; }
die()   { printf '\e[1;31m  ✗\e[0m %s\n' "$*" >&2; exit 1; }

require_cmd() {
    command -v "$1" &>/dev/null || die "Required command not found: $1 — please install it first."
}

# ── uninstall ─────────────────────────────────────────────────────────────────

do_uninstall() {
    echo "!mouse: uninstalling…"

    if systemctl --user is-active --quiet "${BIN_NAME}.service" 2>/dev/null; then
        info "Stopping and disabling systemd service…"
        systemctl --user stop "${BIN_NAME}.service" || true
        systemctl --user disable "${BIN_NAME}.service" || true
    fi
    rm -f "${SERVICE_DEST}"
    systemctl --user daemon-reload 2>/dev/null || true

    rm -f "${BIN_DEST}"
    rm -rf "${ASSET_DEST}"

    ok "Uninstall complete."
    echo
    warn "Your GNOME keyboard shortcut was not removed automatically."
    echo "  To remove it manually, open Settings → Keyboard → Custom Shortcuts."
    exit 0
}

# ── argument parsing ──────────────────────────────────────────────────────────

REGISTER_SHORTCUT=true

for arg in "$@"; do
    case "$arg" in
        --no-shortcut) REGISTER_SHORTCUT=false ;;
        --uninstall)   do_uninstall ;;
        *) die "Unknown argument: $arg  (use --no-shortcut or --uninstall)" ;;
    esac
done

# ── preflight checks ──────────────────────────────────────────────────────────

echo "!mouse installer — $(date)"
echo

require_cmd cargo
require_cmd gjs
require_cmd systemctl

if [[ ! -f "${SERVICE_SRC}" ]]; then
    die "Service file not found at ${SERVICE_SRC} — are you running from the repo root?"
fi

# ── build ─────────────────────────────────────────────────────────────────────

info "Building release binary (cargo build --release)…"
cargo build --release --manifest-path "${REPO_ROOT}/Cargo.toml"
BUILT_BIN="${REPO_ROOT}/target/release/${BIN_NAME}"
[[ -f "${BUILT_BIN}" ]] || die "Build succeeded but binary not found at ${BUILT_BIN}"
ok "Build complete."

# ── install binary ────────────────────────────────────────────────────────────

info "Installing binary to ${BIN_DEST}…"
mkdir -p "$(dirname "${BIN_DEST}")"
rm -f "${BIN_DEST}"
install -m 755 "${BUILT_BIN}" "${BIN_DEST}"
ok "Binary installed."

# Warn if ~/.local/bin is not on PATH.
if ! echo ":${PATH}:" | grep -q ":${HOME}/.local/bin:"; then
    warn "${HOME}/.local/bin is not in your PATH."
    echo "  Add the following to your ~/.bashrc or ~/.profile:"
    echo '  export PATH="$HOME/.local/bin:$PATH"'
fi

# ── install overlay scripts ───────────────────────────────────────────────────

info "Installing overlay assets to ${ASSET_DEST}…"
mkdir -p "${ASSET_DEST}"
install -m 644 "${REPO_ROOT}/platform/linux/gnome/overlay.js"    "${ASSET_DEST}/overlay.js"
install -m 644 "${REPO_ROOT}/platform/linux/gnome/test_bench.js" "${ASSET_DEST}/test_bench.js"
install -m 755 "${REPO_ROOT}/platform/linux/gnome/atspi_scanner.py" "${ASSET_DEST}/atspi_scanner.py"
ok "Assets installed."

# ── systemd user service ──────────────────────────────────────────────────────

info "Installing systemd user service…"
mkdir -p "$(dirname "${SERVICE_DEST}")"
install -m 644 "${SERVICE_SRC}" "${SERVICE_DEST}"
systemctl --user daemon-reload

if systemctl --user is-active --quiet "${BIN_NAME}.service" 2>/dev/null; then
    info "Restarting running daemon…"
    systemctl --user restart "${BIN_NAME}.service"
else
    info "Enabling and starting daemon…"
    systemctl --user enable --now "${BIN_NAME}.service"
fi
ok "Daemon service enabled and running."

# ── /dev/uinput permissions ───────────────────────────────────────────────────

# Check if the user can already access /dev/uinput.
if ! ls /dev/uinput &>/dev/null 2>&1; then
    warn "/dev/uinput is not accessible."
    echo "  !mouse needs write access to /dev/uinput to create a virtual input device."
    echo "  Run one of the following and log out / back in:"
    echo "    sudo usermod -aG input \$USER"
    echo "  Or add a udev rule:"
fi

# ── Desktop accessibility support (AT-SPI / Chromium / Electron) ─────────────

if command -v gsettings &>/dev/null; then
    info "Enabling desktop accessibility (AT-SPI) in GNOME…"
    gsettings set org.gnome.desktop.interface toolkit-accessibility true 2>/dev/null || true
    ok "toolkit-accessibility enabled."
fi

ENV_D="${HOME}/.config/environment.d"
if mkdir -p "${ENV_D}" 2>/dev/null; then
    echo "ACCESSIBILITY_ENABLED=1" > "${ENV_D}/99-notmouse.conf"
    ok "Session environment configured (${ENV_D}/99-notmouse.conf)."
fi

# ── GNOME custom shortcut ─────────────────────────────────────────────────────

if $REGISTER_SHORTCUT; then
    if command -v gsettings &>/dev/null; then
        info "Registering GNOME custom shortcut (${SHORTCUT_KEY})…"

        SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
        KEY="custom-keybindings"
        CUSTOM_BASE="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"
        SLOT="${CUSTOM_BASE}/notmouse/"

        # Collect existing bindings list (may be empty).
        EXISTING="$(gsettings get ${SCHEMA} ${KEY} 2>/dev/null || echo '@as []')"

        # Only add if not already present.
        if echo "${EXISTING}" | grep -q "notmouse"; then
            info "  Shortcut slot already registered — updating command/binding."
        else
            # Append the new slot path.
            if [[ "${EXISTING}" == "@as []" || "${EXISTING}" == "[]" ]]; then
                NEW_LIST="['${SLOT}']"
            else
                # Strip trailing ] and append.
                NEW_LIST="${EXISTING%]}, '${SLOT}']"
            fi
            gsettings set ${SCHEMA} ${KEY} "${NEW_LIST}"
        fi

        SLOT_SCHEMA="${SCHEMA}.custom-keybinding"
        gsettings set "${SLOT_SCHEMA}:${SLOT}" name    "${SHORTCUT_NAME}"
        gsettings set "${SLOT_SCHEMA}:${SLOT}" command "${SHORTCUT_CMD}"
        gsettings set "${SLOT_SCHEMA}:${SLOT}" binding "${SHORTCUT_KEY}"

        ok "Shortcut ${SHORTCUT_KEY} → '${SHORTCUT_CMD}' registered."
    else
        warn "gsettings not found — skipping GNOME shortcut registration."
        echo "  Register manually: Settings → Keyboard → Custom Shortcuts"
        echo "  Command: ${SHORTCUT_CMD}"
        echo "  Shortcut: ${SHORTCUT_KEY}"
    fi
else
    info "Skipping GNOME shortcut registration (--no-shortcut)."
fi

# ── done ──────────────────────────────────────────────────────────────────────

echo
ok "!mouse installed successfully!"
echo
echo "  Binary:   ${BIN_DEST}"
echo "  Assets:   ${ASSET_DEST}/"
echo "  Service:  ${SERVICE_DEST} (active)"
echo "  Shortcut: ${SHORTCUT_KEY} → notmouse overlay"
echo
echo "  Try it now: press ${SHORTCUT_KEY} or run 'notmouse overlay'"
