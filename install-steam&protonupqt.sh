#!/bin/bash

# Navigate to the directory where you want to download the .deb file
cd ~/Downloads

# Update the package list
sudo apt update

# Download the latest Steam .deb package
wget http://media.steampowered.com/client/installer/steam.deb

# Install the package
sudo dpkg -i steam.deb

# Install any missing dependencies and finish configuring the package
sudo apt-get install -f -y

# Download ProtonUp-Qt
wget https://github.com/DavidoTek/ProtonUp-Qt/releases/download/v2.9.2/ProtonUp-Qt-2.9.2-x86_64.AppImage
