#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling Google Chrome"
echo -e "\033[0;32m=====================================\033[0m"

# Navigate to the directory where you want to download the .deb file
cd ~/Downloads

# Update the package list
sudo apt update

# Check if the .deb package is already downloaded
if [ ! -f google-chrome-stable_current_amd64.deb ]; then
    # Download the latest Google Chrome .deb package
    wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb
else
    echo "Google Chrome .deb package is already downloaded."
fi

# Install the package
sudo DEBIAN_FRONTEND=noninteractive dpkg -i -y google-chrome-stable_current_amd64.deb

# Install any missing dependencies and finish configuring the package
sudo apt install -f -y
