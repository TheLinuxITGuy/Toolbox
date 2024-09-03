#!/bin/bash

APP_NAMES="qemu-system-x86 qemu-utils libvirt-daemon-system libvirt-clients bridge-utils virt-manager ovmf"
NALA_CMD="nala"

# Function to display error messages and exit the script
function error_exit {
    echo -e "\033[0;31m$1\033[0m" >&2
    exit 1
}

# Header
echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling Packages"
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

# Install the packages
echo "Installing packages..."
sudo nala install -y $APP_NAMES || error_exit "Failed to install some packages. Please check your system settings or internet connection."

# Install any missing dependencies and finish configuring the packages
echo "Installing missing dependencies..."
sudo nala install -f -y || error_exit "Failed to install missing dependencies."

echo "Packages have been successfully installed."
