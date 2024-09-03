#!/bin/bash

APP_NAME="PyCharm"
FLATPAK_CMD="flatpak"
NALA_CMD="nala"
FLATHUB_REPO_URL="https://flathub.org/repo/flathub.flatpakrepo"
FLATPAK_APP_ID="com.jetbrains.PyCharm-Community"

# Function to display error messages and exit the script
function error_exit {
    echo -e "\033[0;31m$1\033[0m" >&2
    exit 1
}

# Header
echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

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

# Install Flatpak using Nala
echo "Installing Flatpak..."
sudo nala install -y flatpak || error_exit "Failed to install Flatpak."

# Add the Flathub repository
echo "Adding the Flathub repository..."
if ! flatpak remote-add --if-not-exists flathub $FLATHUB_REPO_URL; then
    echo "Failed to add Flathub repository. It may already be added."
fi

# Install PyCharm from Flathub
echo "Installing $APP_NAME from Flathub..."
if ! flatpak install -y flathub $FLATPAK_APP_ID; then
    error_exit "Failed to install $APP_NAME. Please check your system settings or internet connection."
fi

echo "$APP_NAME has been successfully installed."
