#!/usr/bin/env bash
# install.sh — !mouse installer
#
# Builds the release binary, installs assets to XDG-standard locations,
# registers the systemd user service, configures uinput & accessibility,
# and registers desktop keyboard shortcuts.
#
# Usage:
#   ./install.sh                # full install
#   ./install.sh --no-shortcut  # skip desktop shortcut registration
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
UDEV_DEST="/etc/udev/rules.d/99-notmouse.rules"
ENV_CONF="${HOME}/.config/environment.d/99-notmouse.conf"

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
    rm -f "${ENV_CONF}"

    if [[ -f "${UDEV_DEST}" ]] && command -v sudo &>/dev/null; then
        info "Removing udev rule ${UDEV_DEST}…"
        sudo rm -f "${UDEV_DEST}" || true
        sudo udevadm control --reload-rules || true
    fi

    ok "Uninstall complete."
    echo
    warn "If you registered a desktop keyboard shortcut, remove it in your desktop settings or window manager config."
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
require_cmd systemctl

if [[ ! -f "${SERVICE_SRC}" ]]; then
    die "Service file not found at ${SERVICE_SRC} — are you running from the repo root?"
fi

# ── /dev/uinput permissions check ──────────────────────────────────────────────

info "Checking /dev/uinput permissions…"
if [[ -w /dev/uinput ]]; then
    ok "/dev/uinput is writable by ${USER}."
else
    warn "Current user (${USER}) does NOT have write permission to /dev/uinput."
    echo "  !mouse requires write access to /dev/uinput to emit virtual keyboard and pointer events."
    UDEV_RULE='KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uinput"'

    if command -v sudo &>/dev/null; then
        info "Attempting to install uaccess rule to ${UDEV_DEST} with sudo…"
        if echo "${UDEV_RULE}" | sudo tee "${UDEV_DEST}" >/dev/null && \
           sudo udevadm control --reload-rules && \
           (sudo udevadm trigger --name-match=uinput 2>/dev/null || sudo udevadm trigger); then
            ok "Installed udev rule: ${UDEV_DEST}"
        fi
    fi

    if [[ -w /dev/uinput ]]; then
        ok "/dev/uinput is now writable."
    else
        warn "Could not configure /dev/uinput automatically."
        echo "  To configure manually, run:"
        echo "    echo '${UDEV_RULE}' | sudo tee ${UDEV_DEST}"
        echo "    sudo udevadm control --reload-rules && sudo udevadm trigger"
        echo "  Or add your user to the input group (requires logout/login):"
        echo "    sudo usermod -aG input \$USER"
        echo
    fi
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

# ── Desktop accessibility support (AT-SPI / Chromium / Electron) ─────────────

DESKTOP="${XDG_CURRENT_DESKTOP:-}"

if [[ "${DESKTOP}" =~ [Gg][Nn][Oo][Mm][Ee] ]] && command -v gsettings &>/dev/null; then
    info "Enabling desktop accessibility (AT-SPI) in GNOME…"
    gsettings set org.gnome.desktop.interface toolkit-accessibility true 2>/dev/null || true
    ok "toolkit-accessibility enabled."
fi

ENV_D="$(dirname "${ENV_CONF}")"
if mkdir -p "${ENV_D}" 2>/dev/null; then
    echo "ACCESSIBILITY_ENABLED=1" > "${ENV_CONF}"
    ok "Session environment configured (${ENV_CONF})."
fi

# Chromium / Chrome / Brave config flags for web-content accessibility
for flag_file in "${HOME}/.config/chromium-flags.conf" "${HOME}/.config/chrome-flags.conf" "${HOME}/.config/brave-flags.conf"; do
    flag_dir="$(dirname "${flag_file}")"
    if [[ -d "${flag_dir}" ]]; then
        if [[ ! -f "${flag_file}" ]]; then
            echo "--force-renderer-accessibility" > "${flag_file}" 2>/dev/null || true
        elif ! grep -q "force-renderer-accessibility" "${flag_file}" 2>/dev/null; then
            echo "--force-renderer-accessibility" >> "${flag_file}" 2>/dev/null || true
        fi
    fi
done

# ── desktop custom shortcut ───────────────────────────────────────────────────

if $REGISTER_SHORTCUT; then
    if [[ "${DESKTOP}" =~ [Gg][Nn][Oo][Mm][Ee] ]] && command -v gsettings &>/dev/null; then
        info "Registering GNOME custom shortcut (${SHORTCUT_KEY})…"

        SCHEMA="org.gnome.settings-daemon.plugins.media-keys"
        KEY="custom-keybindings"
        CUSTOM_BASE="/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings"
        SLOT="${CUSTOM_BASE}/notmouse/"

        EXISTING="$(gsettings get ${SCHEMA} ${KEY} 2>/dev/null || echo '@as []')"

        if echo "${EXISTING}" | grep -q "notmouse"; then
            info "  Shortcut slot already registered — updating command/binding."
        else
            if [[ "${EXISTING}" == "@as []" || "${EXISTING}" == "[]" ]]; then
                NEW_LIST="['${SLOT}']"
            else
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
        warn "Non-GNOME or custom desktop detected (${DESKTOP:-Unknown})."
        echo "  Automatic shortcut registration is only supported on GNOME."
        echo "  Please bind '${SHORTCUT_KEY}' manually to '${SHORTCUT_CMD}' in your window manager config:"
        echo "    - Sway:     bindsym \$mod+Shift+m exec ${SHORTCUT_CMD}"
        echo "    - Hyprland: bind = SUPER SHIFT, M, exec, ${SHORTCUT_CMD}"
        echo "    - KDE:      Settings → Shortcuts → Custom Shortcuts → Add '${SHORTCUT_CMD}'"
    fi
else
    info "Skipping shortcut registration (--no-shortcut)."
fi

# ── done ──────────────────────────────────────────────────────────────────────

echo
ok "!mouse installed successfully!"
echo
echo "  Binary:   ${BIN_DEST}"
echo "  Service:  ${SERVICE_DEST} (active)"
echo "  Shortcut: ${SHORTCUT_KEY} → notmouse overlay"
echo
echo "  Try it now: press ${SHORTCUT_KEY} or run 'notmouse overlay'"
