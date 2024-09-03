#!/bin/bash

APP_NAME="Google Chrome"
DEB_FILE="google-chrome-stable_current_amd64.deb"
DOWNLOAD_URL="https://dl.google.com/linux/direct/$DEB_FILE"

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

# Navigate to the Downloads directory
cd ~/Downloads || error_exit "Failed to access the Downloads directory."

# Update the package list
echo "Updating package list..."
apt update -qq || error_exit "Failed to update the package list."

# Check if the .deb package is already downloaded
if [ ! -f $DEB_FILE ]; then
    echo "$DEB_FILE not found. Downloading the package..."
    wget $DOWNLOAD_URL || error_exit "Failed to download $DEB_FILE. Please check your internet connection."
else
    echo "$DEB_FILE is already downloaded."
fi

# Install the .deb package
echo "Installing $APP_NAME..."
dpkg -i $DEB_FILE || error_exit "Failed to install $APP_NAME. Please check the .deb file."

# Fix missing dependencies
echo "Fixing missing dependencies..."
apt install -f -y || error_exit "Failed to fix missing dependencies."

echo -e "\033[0;32m$APP_NAME has been installed successfully!\033[0m"
