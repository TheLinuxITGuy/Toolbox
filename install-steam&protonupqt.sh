#!/bin/bash

NALA_CMD="nala"
FLATPAK_CMD="flatpak"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling Steam & ProtonUp-Qt"
echo -e "\033[0;32m=====================================\033[0m"

# Function to display error messages and exit the script
function error_exit {
    echo -e "\033[0;31m$1\033[0m" >&2
    exit 1
}

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null; then
    echo "Nala is not installed. Installing now..."
    sudo apt update -qq || error_exit "Failed to update package list."
    sudo apt install -y nala || error_exit "Failed to install Nala. Please check your internet connection or package manager settings."
else
    echo "Nala is already installed."
fi

# Update the package list using Nala
echo "Updating package list with Nala..."
sudo nala update -qq || error_exit "Failed to update package list with Nala."

# Install Steam using Nala
echo "Installing Steam..."
sudo nala install -y steam || error_exit "Failed to install Steam. Please check your system settings."

# Install Flatpak using Nala
if ! command -v $FLATPAK_CMD &> /dev/null; then
    echo "Flatpak is not installed. Installing now..."
    sudo nala install -y flatpak || error_exit "Failed to install Flatpak. Please check your system settings."
else
    echo "Flatpak is already installed."
fi

# Add the Flathub repository
echo "Adding the Flathub repository..."
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo || error_exit "Failed to add the Flathub repository."

# Install ProtonUp-Qt
echo "Installing ProtonUp-Qt..."
flatpak install --noninteractive flathub net.davidotek.pupgui2 || error_exit "Failed to install ProtonUp-Qt. Please check your system settings."

echo -e "\033[0;32mInstallation of Steam and ProtonUp-Qt completed successfully.\033[0m"
