#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy Toolbox"
echo -e "\033[1;32mDisable Bluetooth"
echo -e "\033[0;32m=====================================\033[0m"

# Block Bluetooth
rfkill block bluetooth

# Stop Bluetooth service
sudo systemctl stop bluetooth.service

# Disable Bluetooth service to start on boot
sudo systemctl disable bluetooth.service

echo -e "\033[1;32mBluetooth has been disabled.\033[0m"
