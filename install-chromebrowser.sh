#!/bin/bash

# Navigate to the directory where you want to download the .deb file
cd ~/Downloads

# Update the package list
sudo apt update

# Download the latest Google Chrome .deb package
wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb

# Install the package
sudo dpkg -i google-chrome-stable_current_amd64.deb

# Install any missing dependencies and finish configuring the package
sudo apt-get install -f
