#!/bin/bash

APP_NAME="xanmod-kernel"
REPO_URL="http://deb.xanmod.org"
KEYRING_PATH="/usr/share/keyrings/xanmod-archive-keyring.gpg"
SOURCE_LIST_PATH="/etc/apt/sources.list.d/xanmod-release.list"
PACKAGE_NAME="linux-xanmod-x64v3"

echo -e "\033[0;31m====================================="
echo -e "\033[1;31mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;31mRemoving $APP_NAME"
echo -e "\033[0;31m=====================================\033[0m"

# Remove XanMod Kernel package
echo "Removing $APP_NAME package..."
sudo apt remove --purge $PACKAGE_NAME -y

# Remove the XanMod repository and GPG key
echo "Removing XanMod repository and GPG key..."
sudo rm -f $SOURCE_LIST_PATH
sudo rm -f $KEYRING_PATH

# Clean up any residual config files and dependencies
echo "Cleaning up residual configuration files and dependencies..."
sudo apt autoremove --purge -y

# Update the package database
echo "Updating package database..."
sudo apt update -qq

echo "Removal complete. Please reboot if you were using the XanMod kernel."
