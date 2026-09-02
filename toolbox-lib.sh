#!/usr/bin/env bash
# Shared validation helpers for Toolbox scripts. Source this file; do not execute it.

is_valid_package_name() {
    local value=${1:-}
    [[ "$value" =~ ^[A-Za-z0-9][A-Za-z0-9._+-]{0,127}$ ]]
}

is_valid_flatpak_id() {
    local value=${1:-}
    [[ ${#value} -le 255 ]] || return 1
    [[ "$value" =~ ^[A-Za-z0-9_][A-Za-z0-9_-]*(\.[A-Za-z0-9_][A-Za-z0-9_-]*)+$ ]]
}

is_valid_exec_name() {
    is_valid_package_name "${1:-}"
}

is_valid_label() {
    local value=${1:-}
    [[ -n "$value" && ${#value} -le 80 ]] || return 1
    [[ "$value" != -* && "$value" != *..* ]] || return 1
    [[ "$value" =~ ^[[:alnum:]][[:alnum:][:space:]._+()-]*$ ]]
}

is_valid_action() {
    [[ "${1:-}" == "install" || "${1:-}" == "remove" ]]
}

require_package_name() {
    if ! is_valid_package_name "${1:-}"; then
        echo "Invalid package name." >&2
        exit 1
    fi
}

require_flatpak_id() {
    if ! is_valid_flatpak_id "${1:-}"; then
        echo "Invalid Flatpak ID." >&2
        exit 1
    fi
}

require_exec_name() {
    if ! is_valid_exec_name "${1:-}"; then
        echo "Invalid executable name." >&2
        exit 1
    fi
}

require_label() {
    if ! is_valid_label "${1:-}"; then
        echo "Invalid application label." >&2
        exit 1
    fi
}

require_action() {
    if ! is_valid_action "${1:-}"; then
        echo "Action must be install or remove." >&2
        exit 1
    fi
}

arch_multilib_enabled() {
    [[ -r /etc/pacman.conf ]] && grep -q '^\[multilib\]' /etc/pacman.conf
}

# Official Brave Origin installer. These are the only download URLs this
# toolbox is allowed to fetch; the signature is verified before the script runs.
BRAVE_INSTALL_SH_URL="https://dl.brave.com/install.sh"
BRAVE_INSTALL_SH_ASC_URL="https://dl.brave.com/install.sh.asc"
BRAVE_INSTALL_SH_FINGERPRINT="D16166072CACDF2C9429CBF11BF41E37D039F691"

is_pinned_brave_download_url() {
    local url=${1:-}
    [[ "$url" == "$BRAVE_INSTALL_SH_URL" || "$url" == "$BRAVE_INSTALL_SH_ASC_URL" ]]
}

# Debian/Fedora package: brave-origin. Arch AUR: brave-origin-bin.
brave_origin_package_candidates() {
    local package_manager=${1:-}
    case "$package_manager" in
        pacman)
            printf '%s\n' "brave-origin-bin" "brave-origin"
            ;;
        *)
            printf '%s\n' "brave-origin"
            ;;
    esac
}

download_pinned_brave_file() {
    local url=${1:-}
    local dest=${2:-}
    if ! is_pinned_brave_download_url "$url"; then
        echo "Refusing to download unpinned URL." >&2
        return 1
    fi
    if [[ -z "$dest" ]]; then
        echo "Download destination is required." >&2
        return 1
    fi
    # Follow HTTPS redirects from the pinned URL (the .asc file is served via CDN).
    # Do not name any other download URL in this toolbox.
    if ! curl -fsSL --proto '=https' --proto-redir '=https' --tlsv1.2 --max-time 60 -o "$dest" "$url"; then
        echo "Failed to download $url" >&2
        return 1
    fi
    if [[ ! -s "$dest" ]]; then
        echo "Downloaded empty file from $url" >&2
        return 1
    fi
}

# Import the vendored Brave installer key into an isolated GNUPGHOME and
# require VALIDSIG for D16166072CACDF2C9429CBF11BF41E37D039F691.
verify_brave_install_script() {
    local script=${1:-}
    local signature=${2:-}
    local key_file=${3:-}
    local gnupg_home status_file

    if [[ ! -f "$script" || ! -s "$script" ]]; then
        echo "Brave installer script is missing." >&2
        return 1
    fi
    if [[ ! -f "$signature" || ! -s "$signature" ]]; then
        echo "Brave installer signature is missing." >&2
        return 1
    fi
    if [[ ! -f "$key_file" || ! -s "$key_file" ]]; then
        echo "Brave installer signing key is missing." >&2
        return 1
    fi
    if ! command -v gpg >/dev/null 2>&1; then
        echo "gpg is required to verify the Brave installer." >&2
        return 1
    fi

    gnupg_home=$(mktemp -d) || return 1
    status_file=$(mktemp) || {
        rm -rf "$gnupg_home"
        return 1
    }
    chmod 700 "$gnupg_home"

    if ! GNUPGHOME="$gnupg_home" gpg --batch --quiet --import "$key_file" >/dev/null 2>&1; then
        rm -rf "$gnupg_home" "$status_file"
        echo "Failed to import Brave installer signing key." >&2
        return 1
    fi

    if ! GNUPGHOME="$gnupg_home" gpg --batch --status-file "$status_file" --verify "$signature" "$script" >/dev/null 2>&1; then
        rm -rf "$gnupg_home" "$status_file"
        echo "GPG verification of the Brave installer failed." >&2
        return 1
    fi

    if ! grep -q "^\[GNUPG:\] VALIDSIG ${BRAVE_INSTALL_SH_FINGERPRINT} " "$status_file"; then
        rm -rf "$gnupg_home" "$status_file"
        echo "GPG verification of the Brave installer failed." >&2
        return 1
    fi

    rm -rf "$gnupg_home" "$status_file"
    return 0
}

# Prepend a `sudo -n` wrapper so nested sudo in the Brave installer cannot
# hang on an interactive password prompt. Relies on Toolbox's `sudo -S -v` cache.
install_noninteractive_sudo_wrapper() {
    local wrap_dir=${1:-}
    local real_sudo
    if [[ -z "$wrap_dir" ]]; then
        echo "sudo wrapper directory is required." >&2
        return 1
    fi
    real_sudo=$(command -v sudo) || {
        echo "sudo was not found on PATH." >&2
        return 1
    }
    mkdir -p "$wrap_dir"
    cat >"$wrap_dir/sudo" <<EOF
#!/bin/sh
exec $(printf '%q' "$real_sudo") -n "\$@"
EOF
    chmod 700 "$wrap_dir/sudo"
}
