#!/bin/bash

APP_NAME="stacer"
NALA_CMD="nala"
PACKAGE_NAME="stacer"

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

# Check if the app is already installed
if ! dpkg -l | grep -qw $PACKAGE_NAME; then
    echo "$APP_NAME is not installed. Installing now..."
    # Install the app
    sudo nala install -y $PACKAGE_NAME || error_exit "Failed to install $APP_NAME. Please check your system settings."
else
    echo "$APP_NAME is already installed. Skipping installation."
fi
