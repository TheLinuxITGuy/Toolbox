#!/usr/bin/env bash
set -euo pipefail

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy Toolbox\n'
printf '[1;32mUpdating System\n'
printf '[0;32m=====================================[0m\n'

update_flatpak() {
    if command -v flatpak >/dev/null 2>&1; then
        echo "Updating user Flatpaks..."
        flatpak update -y
        echo "Updating system Flatpaks..."
        sudo flatpak update -y
    else
        echo "Flatpak is not installed. Skipping Flatpak update."
    fi
}

if command -v apt-get >/dev/null 2>&1; then
    if command -v nala >/dev/null 2>&1; then
        sudo nala update
        sudo nala upgrade -y
    else
        sudo apt-get update
        sudo apt-get upgrade -y
    fi
elif command -v pacman >/dev/null 2>&1; then
    sudo pacman -Syu --noconfirm
elif command -v dnf >/dev/null 2>&1; then
    sudo dnf upgrade --refresh -y
else
    echo "Unsupported distribution."
    exit 1
fi

update_flatpak
