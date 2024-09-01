#!/bin/bash

APP_NAME="zram-tools"
CONFIG_FILE="/etc/default/zramswap"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mDisabling ZRAM and removing unnecessary packages"
echo -e "\033[0;32m=====================================\033[0m"

# Check if zram-tools is installed
if command -v zramswap &> /dev/null
then
    echo "Disabling ZRAM and removing unnecessary packages..."
    # Stop the ZRAM service
    sudo systemctl stop zramswap
    # Remove the zram-tools package and configuration file
    sudo apt remove --purge -y $APP_NAME -qq
    sudo apt autoremove -y -qq
    sudo apt clean -qq
    # Remove the ZRAM configuration file if it exists
    if [[ -f $CONFIG_FILE ]]; then
        sudo rm $CONFIG_FILE
    fi
    if [[ $? -eq 0 ]]; then
        echo "ZRAM has been disabled and unnecessary packages removed successfully."
    else
        echo "There was an error disabling ZRAM and removing unnecessary packages."
    fi
else
    echo "$APP_NAME is not installed. Skipping removal."
fi
