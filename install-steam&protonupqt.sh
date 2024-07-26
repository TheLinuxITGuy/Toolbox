#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling Steam & ProtonUp-Qt"
echo -e "\033[0;32m=====================================\033[0m"

# Navigate to the directory where you want to download the .deb file
cd ~/Downloads

# Update the package list
sudo apt update

# Check if the .deb package is already downloaded
if [ ! -f steam.deb ]; then
    # Download the latest Steam .deb package
    wget wget http://media.steampowered.com/client/installer/steam.deb
else
    echo "Steam .deb package is already downloaded."
fi

# Install the package
sudo dpkg -i steam.deb

# Install any missing dependencies and finish configuring the package
sudo apt install -f -y

# Install flatpak
sudo apt install -y flatpak

# Add the Flathub repository
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Install ProtonUp-Qt
flatpak install -y flathub net.davidotek.pupgui2
