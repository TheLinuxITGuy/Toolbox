#!/bin/bash

NALA_CMD="nala"
FLATPAK_CMD="flatpak"
PACMAN_CMD="pacman"
DNF_CMD="dnf"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy Toolbox"
echo -e "\033[1;32mUpdating System"
echo -e "\033[0;32m=====================================\033[0m"

# Function to update Flatpak
update_flatpak() {
    if command -v $FLATPAK_CMD &> /dev/null
    then
        echo "Updating Flatpak..."
        sudo flatpak update -y
    else
        echo "Flatpak is not installed."
    fi
}

# Function to update system using Nala (Debian/Ubuntu-based)
update_nala() {
    if ! command -v $NALA_CMD &> /dev/null
    then
        echo "Nala is not installed. Installing now..."
        sudo apt update
        sudo apt install -y nala
    fi
    sudo nala update && sudo nala upgrade -y
}

# Function to update system using Pacman (Arch-based)
update_pacman() {
    echo "Updating system using Pacman..."
    sudo pacman -Syu --noconfirm
}

# Function to update system using DNF (Fedora-based)
update_dnf() {
    echo "Updating system using DNF..."
    sudo dnf upgrade --refresh -y
}

# Detect distribution and update accordingly
if [ -f /etc/debian_version ]; then
    update_nala
elif [ -f /etc/arch-release ]; then
    update_pacman
elif [ -f /etc/fedora-release ]; then
    update_dnf
else
    echo "Unsupported distribution."
fi

# Update Flatpak
update_flatpak