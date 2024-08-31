#!/bin/bash

APP_NAME="hypnotix"
NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null
then
    echo "Nala is not installed. Installing now..."
    sudo apt update
    sudo apt install -y nala
fi

# Check if the app is already installed
if ! command -v $APP_NAME &> /dev/null
then
    echo "$APP_NAME is not installed. Installing now..."
    # Update the package database
    sudo nala update
    # Install the app
    sudo nala install -y $APP_NAME
    # Install any missing dependencies and finish configuring the package
    sudo nala install -f -y
else
    echo "$APP_NAME is already installed. Skipping installation."
fi