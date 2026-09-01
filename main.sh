#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=toolbox-lib.sh
source "${SCRIPT_DIR}/toolbox-lib.sh"

ACTION=""
APP_LABEL=""
PACKAGE_NAME=""
FLATPAK_ID=""
EXEC_NAME=""

print_header() {
    local title=$1
    printf '[0;32m=====================================\n'
    printf '[1;32mThe Linux IT Guy Toolbox\n'
    printf '[1;32m%s\n' "$title"
    printf '[0;32m=====================================[0m\n'
}

usage() {
    cat <<EOF
Usage: $(basename "$0") --label <label> [--package <package>] [--flatpak <flatpak-id>] [--exec <exec-name>] <install|remove>
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --label)
            APP_LABEL=${2:-}
            shift 2
            ;;
        --package)
            PACKAGE_NAME=${2:-}
            shift 2
            ;;
        --flatpak)
            FLATPAK_ID=${2:-}
            shift 2
            ;;
        --exec)
            EXEC_NAME=${2:-}
            shift 2
            ;;
        install|remove)
            ACTION=$1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

require_action "$ACTION"
require_label "$APP_LABEL"

if [[ -n "$PACKAGE_NAME" ]]; then
    require_package_name "$PACKAGE_NAME"
fi
if [[ -n "$FLATPAK_ID" ]]; then
    require_flatpak_id "$FLATPAK_ID"
fi
if [[ -n "$EXEC_NAME" ]]; then
    require_exec_name "$EXEC_NAME"
fi

if [[ -z "$PACKAGE_NAME" && -z "$FLATPAK_ID" ]]; then
    echo "At least one install target is required (package or Flatpak ID)." >&2
    exit 1
fi

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

detect_package_manager() {
    if command_exists apt-get; then
        echo "apt"
    elif command_exists pacman; then
        echo "pacman"
    elif command_exists dnf; then
        echo "dnf"
    else
        echo "unknown"
    fi
}

PACKAGE_MANAGER=$(detect_package_manager)

require_supported_pm() {
    if [[ "$PACKAGE_MANAGER" == "unknown" ]]; then
        echo "Unsupported package manager." >&2
        exit 1
    fi
}

apt_install_tool() {
    if command_exists nala; then
        echo "nala"
    else
        echo "apt-get"
    fi
}

APT_TOOL=$(apt_install_tool)

apt_update() {
    if [[ "$APT_TOOL" == "nala" ]]; then
        sudo nala update
    else
        sudo apt-get update
    fi
}

apt_install() {
    if [[ "$APT_TOOL" == "nala" ]]; then
        sudo nala install -y "$@"
        sudo nala install -f -y
    else
        sudo apt-get install -y -- "$@"
        sudo apt-get install -f -y
    fi
}

apt_remove() {
    if [[ "$APT_TOOL" == "nala" ]]; then
        sudo nala remove -y "$@"
    else
        sudo apt-get remove -y -- "$@"
        sudo apt-get autoremove -y
    fi
}

native_installed() {
    local package=$1
    require_package_name "$package"
    case "$PACKAGE_MANAGER" in
        apt)
            dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q "install ok installed"
            ;;
        pacman)
            pacman -Q "$package" >/dev/null 2>&1
            ;;
        dnf)
            rpm -q "$package" >/dev/null 2>&1
            ;;
        *)
            return 1
            ;;
    esac
}

flatpak_installed() {
    local app_id=$1
    require_flatpak_id "$app_id"
    flatpak info "$app_id" >/dev/null 2>&1
}

ensure_flatpak() {
    if command_exists flatpak; then
        return 0
    fi

    echo "Flatpak is not installed. Installing now..."
    case "$PACKAGE_MANAGER" in
        apt)
            apt_update
            apt_install flatpak
            ;;
        pacman)
            sudo pacman -S --needed --noconfirm -- flatpak
            ;;
        dnf)
            sudo dnf install -y -- flatpak
            ;;
        *)
            echo "Unsupported package manager." >&2
            exit 1
            ;;
    esac
}

install_native() {
    local package=$1
    require_package_name "$package"
    if native_installed "$package"; then
        echo "$package is already installed. Skipping installation."
        return 0
    fi

    echo "$package is not installed. Installing now..."

    case "$PACKAGE_MANAGER" in
        apt)
            apt_update
            apt_install "$package"
            ;;
        pacman)
            if [[ "$package" == "steam" ]] && ! arch_multilib_enabled; then
                echo "Steam on Arch requires the multilib repository. Enable [multilib] in /etc/pacman.conf, then retry." >&2
                exit 1
            fi
            sudo pacman -S --needed --noconfirm -- "$package"
            ;;
        dnf)
            sudo dnf install -y -- "$package"
            ;;
    esac
}

remove_native() {
    local package=$1
    require_package_name "$package"
    if ! native_installed "$package"; then
        echo "$package is not installed. Skipping removal."
        return 0
    fi

    case "$PACKAGE_MANAGER" in
        apt)
            apt_remove "$package"
            ;;
        pacman)
            sudo pacman -R --noconfirm -- "$package"
            ;;
        dnf)
            sudo dnf remove -y -- "$package"
            ;;
    esac
}

install_flatpak_app() {
    local app_id=$1
    require_flatpak_id "$app_id"
    ensure_flatpak

    if flatpak_installed "$app_id"; then
        echo "$app_id is already installed. Skipping installation."
        return 0
    fi

    flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak install --user -y -- flathub "$app_id"
}

remove_flatpak_app() {
    local app_id=$1
    require_flatpak_id "$app_id"
    ensure_flatpak

    if ! flatpak_installed "$app_id"; then
        echo "$app_id is not installed. Skipping removal."
        return 0
    fi

    flatpak uninstall -y -- "$app_id"
}

print_header "${ACTION^} ${APP_LABEL}"
require_supported_pm

if [[ "$ACTION" == "install" ]]; then
    if [[ -n "$PACKAGE_NAME" ]]; then
        install_native "$PACKAGE_NAME"
    fi
    if [[ -n "$FLATPAK_ID" ]]; then
        install_flatpak_app "$FLATPAK_ID"
    fi
elif [[ "$ACTION" == "remove" ]]; then
    if [[ -n "$PACKAGE_NAME" ]]; then
        remove_native "$PACKAGE_NAME"
    fi
    if [[ -n "$FLATPAK_ID" ]]; then
        remove_flatpak_app "$FLATPAK_ID"
    fi
fi
