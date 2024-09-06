#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy Toolbox"
echo -e "\033[1;32mEnable Bluetooth"
echo -e "\033[0;32m=====================================\033[0m"

# Unblock Bluetooth
rfkill unblock bluetooth

# Start Bluetooth service
sudo systemctl start bluetooth.service

# Enable Bluetooth service to start on boot
sudo systemctl enable bluetooth.service

echo -e "\033[1;32mBluetooth has been enabled.\033[0m"