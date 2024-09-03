#!/bin/bash

APP_NAME="Microsoft Edge"
NALA_CMD="nala"
FLATPAK_CMD="flatpak"
APP_ID="com.microsoft.Edge"
FLATHUB_REPO_URL="https://flathub.org/repo/flathub.flatpakrepo"

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
    apt install -y nala || error_exit "Failed to install Nala. Check your internet connection or package manager settings."
else
    echo "Nala is already installed."
fi

# Update the package list using Nala
echo "Updating package list using Nala..."
nala update -qq || error_exit "Failed to update package list with Nala."

# Install Flatpak using Nala
echo "Installing Flatpak..."
nala install -y flatpak || error_exit "Failed to install Flatpak."

# Add the Flathub repository
echo "Adding the Flathub repository..."
flatpak remote-add --if-not-exists flathub $FLATHUB_REPO_URL || error_exit "Failed to add Flathub repository."

# Install Microsoft Edge from Flathub
echo "Installing $APP_NAME from Flathub..."
flatpak install -y flathub $APP_ID || error_exit "Failed to install $APP_NAME. Check your system settings or internet connection."

echo -e "\033[0;32m$APP_NAME has been installed successfully!\033[0m"
