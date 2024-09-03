#!/bin/bash

APP_NAMES="tlp tlp-rdw"
NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAMES"
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

# Check if the apps are installed
for APP in $APP_NAMES; do
    if ! dpkg -l | grep -qw $APP; then
        echo "$APP is not installed. Installing now..."
        sudo nala install -y $APP &> /dev/null
    else
        echo "$APP is already installed. Skipping installation."
    fi
done

# Enable and start tlp service
echo "Configuring and starting TLP..."
sudo systemctl enable tlp
sudo systemctl start tlp
