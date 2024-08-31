#!/bin/bash

APP_NAME="google-chrome-stable"
NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mUninstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null
then
    echo "Nala is not installed. Installing now..."
    sudo apt update
    sudo apt install -y nala
fi

# Check if the app is installed using dpkg-query
if dpkg-query -W -f='${Status}' $APP_NAME 2>/dev/null | grep -q "install ok installed"
then
    echo "$APP_NAME is installed. Uninstalling now..."
    # Remove the app
    sudo nala remove -y $APP_NAME
    # Remove any unused dependencies
    sudo nala autoremove -y
else
    echo "$APP_NAME is not installed. Skipping uninstallation."
fi