#!/bin/bash

APP_NAME="Opera"
NALA_CMD="nala"
FLATPAK_CMD="flatpak"
FLATPAK_REMOTE="flathub"
FLATPAK_REPO_URL="https://flathub.org/repo/flathub.flatpakrepo"
FLATPAK_APP="com.opera.Opera"

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
    sudo apt install -y nala || error_exit "Failed to install $NALA_CMD. Please check your internet connection or package manager settings."
else
    echo "Nala is already installed."
fi

# Update the package list using Nala
echo "Updating package list..."
sudo nala update -qq || error_exit "Failed to update package list with Nala."

# Install Flatpak using Nala if not already installed
if ! command -v $FLATPAK_CMD &> /dev/null; then
    echo "Flatpak is not installed. Installing now..."
    sudo nala install -y flatpak || error_exit "Failed to install Flatpak. Please check your system settings."
else
    echo "Flatpak is already installed."
fi

# Add the Flathub repository if not already added
if ! $FLATPAK_CMD remotes | grep -q $FLATPAK_REMOTE; then
    echo "Adding the Flathub repository..."
    $FLATPAK_CMD remote-add --if-not-exists $FLATPAK_REMOTE $FLATPAK_REPO_URL || error_exit "Failed to add the Flathub repository. Please check your internet connection."
else
    echo "Flathub repository is already added."
fi

# Install Opera from Flathub
echo "Installing $APP_NAME from Flathub..."
$FLATPAK_CMD install -y $FLATPAK_REMOTE $FLATPAK_APP || error_exit "Failed to install $APP_NAME. Please check your system settings or internet connection."

echo "$APP_NAME has been successfully installed."
