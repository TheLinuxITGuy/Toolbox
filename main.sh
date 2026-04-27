#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
ACTION=""
APP_LABEL=""
PACKAGE_NAME=""
FLATPAK_ID=""
EXEC_NAME=""
NIX_PACKAGE=""
NIXOS_CONFIG="/etc/nixos/configuration.nix"
NIXOS_TOOLBOX_MODULE="/etc/nixos/toolbox-packages.nix"

print_header() {
    local title=$1
    printf '[0;32m=====================================\n'
    printf '[1;32mThe Linux IT Guy Toolbox\n'
    printf '[1;32m%s\n' "$title"
    printf '[0;32m=====================================[0m\n'
}

usage() {
    cat <<EOF
Usage: $(basename "$0") --label <label> [--package <package>] [--flatpak <flatpak-id>] [--exec <exec-name>] [--nix-package <nix-attr>] <install|remove>
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
        --nix-package)
            NIX_PACKAGE=${2:-}
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

if [[ -z "$PACKAGE_NAME" && -z "$FLATPAK_ID" && -z "$NIX_PACKAGE" ]]; then
    echo "At least one install target is required (package, Flatpak ID, or Nix package)." >&2
    exit 1
fi

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

detect_package_manager() {
    if [[ -f /etc/NIXOS ]]; then
        echo "nixos"
    elif command_exists apt-get; then
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
        sudo apt-get install -y "$@"
        sudo apt-get install -f -y
    fi
}

apt_remove() {
    if [[ "$APT_TOOL" == "nala" ]]; then
        sudo nala remove -y "$@"
    else
        sudo apt-get remove -y "$@"
        sudo apt-get autoremove -y
    fi
}

native_installed() {
    local package=$1
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
        nixos)
            nixos_package_configured "$NIX_PACKAGE"
            ;;
        *)
            return 1
            ;;
    esac
}

flatpak_installed() {
    local app_id=$1
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
            sudo pacman -Syu --noconfirm
            sudo pacman -S --noconfirm flatpak
            ;;
        dnf)
            sudo dnf install -y flatpak
            ;;
        nixos)
            echo "Flatpak is not installed. Enable Flatpak declaratively in your NixOS configuration first."
            exit 1
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
            sudo pacman -S --noconfirm "$package"
            ;;
        dnf)
            sudo dnf install -y "$package"
            ;;
        nixos)
            install_nixos_package "$NIX_PACKAGE"
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
            sudo pacman -R --noconfirm "$package"
            ;;
        dnf)
            sudo dnf remove -y "$package"
            ;;
        nixos)
            remove_nixos_package "$NIX_PACKAGE"
            ;;
    esac
}

nixos_require_package() {
    local package=${1:-}
    if [[ -z "$package" ]]; then
        echo "No Nix package attribute is configured for ${APP_LABEL}." >&2
        echo "Add a Nix Package value for this app in apps_config.csv." >&2
        exit 1
    fi
}

nixos_package_configured() {
    local package=${1:-}
    [[ -n "$package" ]] || return 1
    [[ -f "$NIXOS_TOOLBOX_MODULE" ]] || return 1
    read_nixos_packages | grep -Fxq "$package"
}

ensure_nixos_toolbox_module() {
    if [[ ! -f "$NIXOS_CONFIG" ]]; then
        echo "NixOS configuration file not found at ${NIXOS_CONFIG}." >&2
        exit 1
    fi

    if [[ ! -f "$NIXOS_TOOLBOX_MODULE" ]]; then
        echo "Creating ${NIXOS_TOOLBOX_MODULE}..."
        sudo tee "$NIXOS_TOOLBOX_MODULE" >/dev/null <<'EOF'
{ pkgs, ... }:

{
  # Managed by The Linux IT Guy Toolbox.
  # The Toolbox edits this file instead of editing your main configuration.nix package list.
  environment.systemPackages = with pkgs; [
  ];
}
EOF
    fi

    if ! grep -Eq '^[[:space:]]*\./toolbox-packages\.nix' "$NIXOS_CONFIG"; then
        echo "Adding ./toolbox-packages.nix import to ${NIXOS_CONFIG}..."
        sudo cp "$NIXOS_CONFIG" "${NIXOS_CONFIG}.toolbox.bak"
        local tmp_file
        tmp_file=$(mktemp)
        awk '
            BEGIN { in_imports=0; inserted=0 }
            /imports[[:space:]]*=[[:space:]]*\[/ { in_imports=1 }
            in_imports && /^[[:space:]]*\];/ && !inserted {
                print "    ./toolbox-packages.nix"
                inserted=1
                in_imports=0
            }
            { print }
            END {
                if (!inserted) {
                    exit 42
                }
            }
        ' "$NIXOS_CONFIG" > "$tmp_file" || {
            rm -f "$tmp_file"
            echo "Could not find an imports = [ ... ]; block in configuration.nix." >&2
            echo "Add ./toolbox-packages.nix manually and try again." >&2
            exit 1
        }
        sudo tee "$NIXOS_CONFIG" < "$tmp_file" >/dev/null
        rm -f "$tmp_file"
    else
        echo "${NIXOS_CONFIG} already imports ./toolbox-packages.nix."
    fi
}

rewrite_nixos_packages() {
    local packages=("$@")
    {
        printf '{ pkgs, ... }:\n\n'
        printf '{\n'
        printf '  # Managed by The Linux IT Guy Toolbox.\n'
        printf '  # The Toolbox edits this file instead of editing your main configuration.nix package list.\n'
        printf '  environment.systemPackages = with pkgs; [\n'
        for package in "${packages[@]}"; do
            printf '    %s\n' "$package"
        done
        printf '  ];\n'
        printf '}\n'
    } | sudo tee "$NIXOS_TOOLBOX_MODULE" >/dev/null
}

read_nixos_packages() {
    awk '
        /environment\.systemPackages[[:space:]]*=[[:space:]]*with[[:space:]]+pkgs;[[:space:]]*\[/ { inside=1; next }
        inside && /^[[:space:]]*\]/ { inside=0; next }
        inside {
            line=$0
            sub(/#.*/, "", line)
            gsub(/^[[:space:]]+|[[:space:]]+$/, "", line)
            if (line != "") print line
        }
    ' "$NIXOS_TOOLBOX_MODULE" | sort -u
}

apply_nixos_config() {
    echo "Applying NixOS configuration..."
    sudo nixos-rebuild switch
}

install_nixos_package() {
    local package=$1
    nixos_require_package "$package"
    ensure_nixos_toolbox_module

    if nixos_package_configured "$package"; then
        echo "${package} is already managed by ${NIXOS_TOOLBOX_MODULE}. Skipping edit."
    else
        echo "Adding ${package} to ${NIXOS_TOOLBOX_MODULE}..."
        mapfile -t packages < <(read_nixos_packages)
        packages+=("$package")
        mapfile -t packages < <(printf '%s\n' "${packages[@]}" | awk 'NF' | sort -u)
        rewrite_nixos_packages "${packages[@]}"
    fi

    apply_nixos_config
}

remove_nixos_package() {
    local package=$1
    nixos_require_package "$package"
    ensure_nixos_toolbox_module

    if ! nixos_package_configured "$package"; then
        echo "${package} is not managed by ${NIXOS_TOOLBOX_MODULE}. Skipping edit."
    else
        echo "Removing ${package} from ${NIXOS_TOOLBOX_MODULE}..."
        mapfile -t packages < <(read_nixos_packages | grep -Fxv "$package" || true)
        rewrite_nixos_packages "${packages[@]}"
    fi

    apply_nixos_config
}

install_flatpak_app() {
    local app_id=$1
    ensure_flatpak

    if flatpak_installed "$app_id"; then
        echo "$app_id is already installed. Skipping installation."
        return 0
    fi

    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    flatpak install -y flathub "$app_id"
}

remove_flatpak_app() {
    local app_id=$1
    ensure_flatpak

    if ! flatpak_installed "$app_id"; then
        echo "$app_id is not installed. Skipping removal."
        return 0
    fi

    flatpak uninstall -y "$app_id"
}

print_header "${ACTION^} ${APP_LABEL}"
require_supported_pm

if [[ "$PACKAGE_MANAGER" == "nixos" ]]; then
    if [[ "$ACTION" == "install" ]]; then
        install_nixos_package "$NIX_PACKAGE"
    elif [[ "$ACTION" == "remove" ]]; then
        remove_nixos_package "$NIX_PACKAGE"
    fi
    exit 0
fi

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
