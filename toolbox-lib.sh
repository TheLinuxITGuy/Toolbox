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
