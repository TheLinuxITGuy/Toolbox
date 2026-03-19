#!/usr/bin/env bash
set -euo pipefail

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy Toolbox\n'
printf '[1;32mInstalling fastfetch\n'
printf '[0;32m=====================================[0m\n'

if command -v apt-get >/dev/null 2>&1; then
    if command -v nala >/dev/null 2>&1; then
        sudo nala update
        sudo nala install -y fastfetch
        sudo nala install -f -y
    else
        sudo apt-get update
        sudo apt-get install -y fastfetch
    fi
elif command -v pacman >/dev/null 2>&1; then
    sudo pacman -Syu --noconfirm
    sudo pacman -S --noconfirm fastfetch
elif command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y fastfetch
else
    echo "Unsupported distribution."
    exit 1
fi
