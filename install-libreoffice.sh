#!/bin/bash

APP_NAME="libreoffice"
NALA_CMD="nala"

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

# Check if the app is already installed
if ! command -v $APP_NAME &> /dev/null; then
    echo "$APP_NAME is not installed. Installing now..."
    # Install the app
    nala install -y $APP_NAME || error_exit "Failed to install $APP_NAME. Please check your system settings or internet connection."
    # Install any missing dependencies and finish configuring the package
    nala install -f -y || error_exit "Failed to install missing dependencies for $APP_NAME."
    echo "$APP_NAME has been installed successfully."
else
    echo "$APP_NAME is already installed. Skipping installation."
fi
