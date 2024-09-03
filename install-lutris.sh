#!/bin/bash

NALA_CMD="nala"
FLATPAK_CMD="flatpak"
APP_NAME="Lutris"

# Function to display error messages and exit the script
function error_exit {
    echo -e "\033[0;31m$1\033[0m" >&2
    exit 1
}

# Check if the script is being run as root
if [[ $EUID -ne 0 ]]; then
    error_exit "This script must be run as root."
fi

# Header
echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null; then
    echo "Nala is not installed. Installing now..."
    apt update -qq || error_exit "Failed to update package list."
    apt install -y nala || error_exit "Failed to install Nala. Please check your internet connection or package manager settings."
else
    echo "Nala is already installed."
fi

# Update the package list using Nala
echo "Updating package list..."
nala update -qq || error_exit "Failed to update package list. Please try again later."

# Check if Flatpak is installed
if ! command -v $FLATPAK_CMD &> /dev/null; then
    echo "Flatpak is not installed. Installing now..."
    nala install -y flatpak || error_exit "Failed to install Flatpak. Please check your system settings."
else
    echo "Flatpak is already installed."
fi

# Update Flatpak metadata
echo "Updating Flatpak metadata..."
flatpak update --appstream -y || error_exit "Failed to update Flatpak metadata."

# Add the Flathub repository
echo "Adding the Flathub repository..."
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo || error_exit "Failed to add the Flathub repository. Please check your internet connection."

# Install Lutris from Flathub
echo "Installing $APP_NAME from Flathub..."
flatpak install -y flathub net.lutris.Lutris || error_exit "Failed to install $APP_NAME. Please check your system settings or internet connection."

echo "$APP_NAME has been installed successfully."
