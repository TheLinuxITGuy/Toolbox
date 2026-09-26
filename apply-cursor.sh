#!/usr/bin/env bash
# Install a pinned Bibata v2.0.7 Linux left-pointer theme for the current user
# and select it on GNOME. User-level install only. Archives are the .tar.xz builds, not the
# -Right themes and not the Windows zips.
set -euo pipefail

cursor_download_url() {
    case "${1:-}" in
        Bibata-Modern-Classic)
            printf '%s\n' "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/Bibata-Modern-Classic.tar.xz"
            ;;
        Bibata-Modern-Ice)
            printf '%s\n' "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/Bibata-Modern-Ice.tar.xz"
            ;;
        Bibata-Modern-Amber)
            printf '%s\n' "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/Bibata-Modern-Amber.tar.xz"
            ;;
        *)
            echo "Unknown cursor theme: ${1:-}" >&2
            return 1
            ;;
    esac
}

theme_is_present() {
    local theme="${1:-}"
    [[ -f "${HOME}/.local/share/icons/${theme}/index.theme" ]] && return 0
    [[ -f "/usr/share/icons/${theme}/index.theme" ]] && return 0
    return 1
}

require_gnome_cursor_schema() {
    if ! command -v gsettings >/dev/null 2>&1; then
        echo "The Cursor tab needs GNOME. gsettings is not available." >&2
        return 1
    fi

    local schemas
    if ! schemas="$(gsettings list-schemas 2>/dev/null)"; then
        echo "The Cursor tab needs GNOME. The org.gnome.desktop.interface schema is not available." >&2
        return 1
    fi
    if ! grep -qx 'org.gnome.desktop.interface' <<<"${schemas}"; then
        echo "The Cursor tab needs GNOME. The org.gnome.desktop.interface schema is not available." >&2
        return 1
    fi

    local keys
    if ! keys="$(gsettings list-keys org.gnome.desktop.interface 2>/dev/null)"; then
        echo "The Cursor tab needs GNOME. The org.gnome.desktop.interface schema is not available." >&2
        return 1
    fi
    if ! grep -qx 'cursor-theme' <<<"${keys}"; then
        echo "The Cursor tab needs GNOME. The org.gnome.desktop.interface schema is not available." >&2
        return 1
    fi
}

# Refuse to extract unless every member lives under the expected theme
# directory, index.theme is present, and symlink targets stay relative.
archive_is_safe() {
    local theme="$1"
    local archive="$2"
    local listing
    listing="$(tar -tJf "${archive}")" || return 1

    local found=0
    local line
    while IFS= read -r line || [[ -n "${line}" ]]; do
        [[ -z "${line}" ]] && continue
        case "${line}" in
            /*)
                echo "Refusing absolute archive path: ${line}" >&2
                return 1
                ;;
            *..*)
                echo "Refusing archive path with ..: ${line}" >&2
                return 1
                ;;
        esac
        case "${line}" in
            "${theme}" | "${theme}/" | "${theme}/"*) ;;
            *)
                echo "Archive member is outside ${theme}: ${line}" >&2
                return 1
                ;;
        esac
        if [[ "${line}" == "${theme}/index.theme" ]]; then
            found=1
        fi
    done <<<"${listing}"

    if [[ "${found}" -ne 1 ]]; then
        echo "Archive does not contain the ${theme} theme directory." >&2
        return 1
    fi

    local verbose target
    verbose="$(tar -tvJf "${archive}")" || return 1
    while IFS= read -r line || [[ -n "${line}" ]]; do
        [[ "${line}" == l* && "${line}" == *" -> "* ]] || continue
        target="${line##* -> }"
        case "${target}" in
            /*)
                echo "Refusing absolute symlink target: ${target}" >&2
                return 1
                ;;
            *..*)
                echo "Refusing symlink target with ..: ${target}" >&2
                return 1
                ;;
        esac
    done <<<"${verbose}"
}

symlinks_stay_inside() {
    local root="$1"
    local link target
    while IFS= read -r link; do
        target="$(readlink -- "${link}")" || return 1
        case "${target}" in
            /* | *..*)
                echo "Refusing symlink ${link} -> ${target}" >&2
                return 1
                ;;
        esac
    done < <(find "${root}" -type l -print)
}

install_theme_from_archive() {
    local theme="$1"
    local archive="$2"
    archive_is_safe "${theme}" "${archive}"

    local extract
    extract="$(mktemp -d)"
    tar -xJf "${archive}" -C "${extract}"
    if [[ ! -f "${extract}/${theme}/index.theme" ]]; then
        rm -rf "${extract}"
        echo "Archive did not extract ${theme}/index.theme." >&2
        return 1
    fi
    if ! symlinks_stay_inside "${extract}/${theme}"; then
        rm -rf "${extract}"
        return 1
    fi

    local dest_root="${HOME}/.local/share/icons"
    mkdir -p "${dest_root}"
    rm -rf "${dest_root}/${theme}.partial"
    mv "${extract}/${theme}" "${dest_root}/${theme}.partial"
    rm -rf "${extract}"
    rm -rf "${dest_root}/${theme}"
    mv "${dest_root}/${theme}.partial" "${dest_root}/${theme}"
}

download_theme() {
    local theme="$1"
    local archive="$2"
    local url
    url="$(cursor_download_url "${theme}")" || return 1
    case "${url}" in
        "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/${theme}.tar.xz") ;;
        *)
            echo "Refusing unexpected cursor download URL." >&2
            return 1
            ;;
    esac

    if command -v curl >/dev/null 2>&1; then
        curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
            --output "${archive}" -- "${url}"
    elif command -v wget >/dev/null 2>&1; then
        wget -q -O "${archive}" -- "${url}"
    else
        echo "curl or wget is required to download the cursor theme." >&2
        return 1
    fi
}

write_user_cursor_files() {
    local theme="$1"
    local xdg_default="${HOME}/.local/share/icons/default"
    local legacy_default="${HOME}/.icons/default"
    local env_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/environment.d"
    mkdir -p "${xdg_default}" "${legacy_default}" "${env_dir}"
    cat >"${xdg_default}/index.theme" <<EOF
[Icon Theme]
Name=Default
Comment=User cursor default
Inherits=${theme}
EOF
    cat >"${legacy_default}/index.theme" <<EOF
[Icon Theme]
Name=Default
Comment=User cursor default
Inherits=${theme}
EOF
    printf 'XCURSOR_THEME=%s\n' "${theme}" >"${env_dir}/99-toolbox-cursor.conf"
}

publish_session_cursor() {
    local theme="$1"
    if command -v dbus-update-activation-environment >/dev/null 2>&1; then
        dbus-update-activation-environment --systemd "XCURSOR_THEME=${theme}" || true
    elif command -v systemctl >/dev/null 2>&1; then
        systemctl --user set-environment "XCURSOR_THEME=${theme}" || true
    fi
}

persist_user_cursor() {
    local theme="$1"
    write_user_cursor_files "${theme}"
    publish_session_cursor "${theme}"
}

tmpdir=""
cleanup_cursor_tmp() {
    if [[ -n "${tmpdir}" ]]; then
        rm -rf "${tmpdir}"
    fi
}

main() {
    if [[ "$#" -ne 1 ]]; then
        echo "Usage: apply-cursor.sh <Bibata-Modern-Classic|Bibata-Modern-Ice|Bibata-Modern-Amber>" >&2
        exit 1
    fi

    local theme="$1"
    cursor_download_url "${theme}" >/dev/null
    require_gnome_cursor_schema

    if theme_is_present "${theme}"; then
        echo "Cursor theme ${theme} is already installed."
    else
        echo "Installing cursor theme ${theme} into ~/.local/share/icons."
        tmpdir="$(mktemp -d)"
        trap cleanup_cursor_tmp EXIT
        local archive="${tmpdir}/${theme}.tar.xz"
        download_theme "${theme}" "${archive}"
        install_theme_from_archive "${theme}" "${archive}"
    fi

    gsettings set org.gnome.desktop.interface cursor-theme "${theme}"
    persist_user_cursor "${theme}"
    echo "GNOME cursor theme set to ${theme}."
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
