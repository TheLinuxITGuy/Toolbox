#!/bin/bash

APP_NAME="nala"
NALA_CMD="nala"

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
    sudo apt install -y nala || error_exit "Failed to install $APP_NAME. Please check your internet connection or package manager settings."
else
    echo "$APP_NAME is already installed."
fi

# Fetch updates using Nala
echo "Fetching updates with $APP_NAME..."
sudo nala fetch --auto || error_exit "Failed to fetch updates. Please check your system settings or internet connection."

echo "$APP_NAME has been successfully installed and updated."
