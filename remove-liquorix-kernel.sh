#!/bin/bash

APP_NAME="liquorix-kernel"
PPA_REPO="ppa:damentz/liquorix"

echo -e "\033[0;31m====================================="
echo -e "\033[1;31mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;31mRemoving $APP_NAME"
echo -e "\033[0;31m=====================================\033[0m"

# Remove Liquorix Kernel package
echo "Removing $APP_NAME package..."
sudo apt remove --purge linux-image-liquorix-amd64 -y

# Remove the Liquorix PPA
echo "Removing Liquorix PPA..."
sudo add-apt-repository --remove $PPA_REPO -y

# Clean up any residual config files and dependencies
echo "Cleaning up residual configuration files and dependencies..."
sudo apt autoremove --purge -y

# Update the package database
echo "Updating package database..."
sudo apt update -qq

echo "Removal complete. Please reboot if you were using the Liquorix kernel."
