#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling Microsoft Visual Studio Code"
echo -e "\033[0;32m=====================================\033[0m"

# Update the package list
sudo apt update

# Install flatpak
sudo apt install -y flatpak

# Add the Flathub repository
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Install Code
flatpak install -y flathub com.visualstudio.code
