#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
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

if [[ -z "$ACTION" || -z "$APP_LABEL" ]]; then
    usage >&2
    exit 1
fi

if [[ -z "$PACKAGE_NAME" && -z "$FLATPAK_ID" ]]; then
    echo "At least one install target is required (package or Flatpak ID)." >&2
    exit 1
fi

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

validate_native_package() {
    local package=$1
    [[ "$package" =~ ^[A-Za-z0-9][A-Za-z0-9+._:-]*$ ]]
}

validate_flatpak_id() {
    local app_id=$1
    [[ "$app_id" =~ ^[A-Za-z0-9_]+(\.[A-Za-z0-9_]+)+$ ]]
}

validate_exec_name() {
    local exec_name=$1
    [[ -z "$exec_name" || "$exec_name" =~ ^[A-Za-z0-9][A-Za-z0-9+._-]*$ ]]
}

validate_inputs() {
    if [[ -n "$PACKAGE_NAME" ]] && ! validate_native_package "$PACKAGE_NAME"; then
        echo "Invalid native package name: $PACKAGE_NAME" >&2
        exit 1
    fi
    if [[ -n "$FLATPAK_ID" ]] && ! validate_flatpak_id "$FLATPAK_ID"; then
        echo "Invalid Flatpak ID: $FLATPAK_ID" >&2
        exit 1
    fi
    if ! validate_exec_name "$EXEC_NAME"; then
        echo "Invalid executable name: $EXEC_NAME" >&2
        exit 1
    fi
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

validate_inputs

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
        sudo nala install -y -- "$@"
        sudo nala install -f -y
    else
        sudo apt-get install -y -- "$@"
        sudo apt-get install -f -y
    fi
}

apt_remove() {
    if [[ "$APT_TOOL" == "nala" ]]; then
        sudo nala remove -y -- "$@"
    else
        sudo apt-get remove -y -- "$@"
        sudo apt-get autoremove -y
    fi
}

native_installed() {
    local package=$1
    case "$PACKAGE_MANAGER" in
        apt)
            dpkg-query -W -f='${Status}' -- "$package" 2>/dev/null | grep -q "install ok installed"
            ;;
        pacman)
            pacman -Q -- "$package" >/dev/null 2>&1
            ;;
        dnf)
            rpm -q -- "$package" >/dev/null 2>&1
            ;;
        *)
            return 1
            ;;
    esac
}

flatpak_installed() {
    local app_id=$1
    flatpak info -- "$app_id" >/dev/null 2>&1
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
            sudo pacman -Syu --noconfirm
            sudo pacman -S --noconfirm flatpak
            ;;
        dnf)
            sudo dnf install -y flatpak
            ;;
        *)
            echo "Unsupported package manager." >&2
            exit 1
            ;;
    esac
}

install_native() {
    local package=$1
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
            if [[ "$package" == "steam" ]] && ! grep -q '^\[multilib\]' /etc/pacman.conf; then
                echo "Enabling multilib repository for Steam..."
                sudo sed -i '/\[multilib\]/,/Include/s/^#//' /etc/pacman.conf
            fi
            sudo pacman -Syu --noconfirm
            sudo pacman -S --noconfirm -- "$package"
            ;;
        dnf)
            sudo dnf install -y -- "$package"
            ;;
    esac
}

remove_native() {
    local package=$1
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
    ensure_flatpak

    if flatpak_installed "$app_id"; then
        echo "$app_id is already installed. Skipping installation."
        return 0
    fi

    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak install -y -- flathub "$app_id"
}

remove_flatpak_app() {
    local app_id=$1
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
