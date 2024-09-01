#!/bin/bash

APP_NAME="liquorix-kernel"
PPA_REPO="ppa:damentz/liquorix"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if add-apt-repository is installed
if ! command -v add-apt-repository &> /dev/null
then
    echo "add-apt-repository is not installed. Installing now..."
    sudo apt update
    sudo apt install -y software-properties-common
fi

# Add Liquorix PPA
echo "Adding Liquorix PPA..."
sudo add-apt-repository $PPA_REPO -y

# Update the package database
echo "Updating package database..."
sudo apt update -qq

# Install Liquorix Kernel
echo "Installing Liquorix Kernel..."
sudo apt install linux-image-liquorix-amd64 -y -qq

echo "Installation complete. Please reboot to use the new kernel."
