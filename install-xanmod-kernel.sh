#!/bin/bash

APP_NAME="xanmod-kernel"
REPO_URL="http://deb.xanmod.org"
KEYRING_PATH="/usr/share/keyrings/xanmod-archive-keyring.gpg"
SOURCE_LIST_PATH="/etc/apt/sources.list.d/xanmod-release.list"
PACKAGE_NAME="linux-xanmod-x64v3"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Add XanMod GPG key
echo "Adding XanMod GPG key..."
wget -qO - https://dl.xanmod.org/archive.key | gpg --dearmor -o $KEYRING_PATH

# Add XanMod repository
echo "Adding XanMod repository..."
echo "deb [signed-by=$KEYRING_PATH] $REPO_URL releases main" | sudo tee $SOURCE_LIST_PATH

# Update the package database
echo "Updating package database..."
sudo apt update -qq

# Install XanMod Kernel
echo "Installing XanMod Kernel..."
sudo apt install $PACKAGE_NAME -y -qq

echo "Installation complete. Please reboot to use the new kernel."
