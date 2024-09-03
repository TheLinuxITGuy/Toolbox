#!/bin/bash

# Function to display error messages and exit the script
function error_exit {
    echo -e "\033[0;31m$1\033[0m" >&2
    exit 1
}

# Check if the script is being run as root
if [[ $EUID -ne 0 ]]; then
    error_exit "This script must be run as root."
fi

# Header
echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mDisabling Bluetooth"
echo -e "\033[0;32m=====================================\033[0m"

# Block Bluetooth
echo "Blocking Bluetooth..."
if rfkill block bluetooth &> /dev/null; then
    echo "Bluetooth has been blocked."
else
    error_exit "Failed to block Bluetooth. Please check your system settings."
fi

# Stop Bluetooth service
echo "Stopping Bluetooth service..."
if systemctl stop bluetooth.service &> /dev/null; then
    echo "Bluetooth service has been stopped."
else
    error_exit "Failed to stop Bluetooth service. Please check your system settings."
fi

# Disable Bluetooth service from starting at boot
echo "Disabling Bluetooth service from starting on boot..."
if systemctl disable bluetooth.service &> /dev/null; then
    echo "Bluetooth service has been disabled from starting on boot."
else
    error_exit "Failed to disable Bluetooth service. Please check your system settings."
fi

echo -e "\033[0;32mBluetooth has been successfully disabled.\033[0m"
