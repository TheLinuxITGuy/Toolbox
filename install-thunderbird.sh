#!/bin/bash

APP_NAME="thunderbird"
NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null; then
    echo "Nala is not installed. Installing now..."
    sudo apt update -y &> /dev/null
    sudo apt install -y nala &> /dev/null
fi

# Update the package list using Nala
echo "Updating package list..."
sudo nala update -y &> /dev/null

# Check if the app is already installed
if ! dpkg -l | grep -qw $APP_NAME; then
    echo "$APP_NAME is not installed. Installing now..."
    # Install the app
    sudo nala install -y $APP_NAME &> /dev/null
else
    echo "$APP_NAME is already installed. Skipping installation."
fi
