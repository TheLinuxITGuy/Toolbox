#!/bin/bash

NALA_CMD="nala"
FLATPAK_CMD="flatpak"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling Vivaldi"
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

# Check if Flatpak is installed
if ! command -v $FLATPAK_CMD &> /dev/null; then
    echo "Flatpak is not installed. Installing now..."
    sudo nala install -y flatpak &> /dev/null
fi

# Add the Flathub repository
echo "Adding Flathub repository..."
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo &> /dev/null

# Install Vivaldi
echo "Installing Vivaldi..."
flatpak install -y flathub com.vivaldi.Vivaldi &> /dev/null

echo "Vivaldi installation complete."
