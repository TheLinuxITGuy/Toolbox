#!/bin/bash

APP_NAME="zram-tools"
CONFIG_FILE="/etc/default/zramswap"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mEnabling ZRAM"
echo -e "\033[0;32m=====================================\033[0m"

# Check if zram-tools is installed
if ! command -v zramswap &> /dev/null
then
    echo "$APP_NAME is not installed. Installing now..."
    sudo apt update
    sudo apt install -y $APP_NAME
else
    echo "$APP_NAME is already installed. Skipping installation."
fi

# Enable and configure ZRAM
echo "Enabling ZRAM for memory optimization..."
if [[ ! -f $CONFIG_FILE ]]; then
    echo -e "ALGO=zstd\nPERCENT=60" | sudo tee $CONFIG_FILE
else
    echo "Configuration file already exists. Skipping configuration."
fi

# Reload the ZRAM service
sudo systemctl restart zramswap

# Check if ZRAM was successfully enabled
if [[ $? -eq 0 ]]; then
    echo "ZRAM has been successfully enabled."
else
    echo "There was an error enabling ZRAM."
fi
