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
echo -e "\033[1;32mEnabling Bluetooth"
echo -e "\033[0;32m=====================================\033[0m"

# Unblock Bluetooth
echo "Unblocking Bluetooth..."
if rfkill unblock bluetooth &> /dev/null; then
    echo "Bluetooth has been unblocked."
else
    error_exit "Failed to unblock Bluetooth. Please check your system settings."
fi

# Start Bluetooth service
echo "Starting Bluetooth service..."
if systemctl start bluetooth.service &> /dev/null; then
    echo "Bluetooth service has been started."
else
    error_exit "Failed to start Bluetooth service. Please check your system settings."
fi

# Enable Bluetooth service to start on boot
echo "Enabling Bluetooth service to start on boot..."
if systemctl enable bluetooth.service &> /dev/null; then
    echo "Bluetooth service has been enabled to start on boot."
else
    error_exit "Failed to enable Bluetooth service to start on boot. Please check your system settings."
fi

echo -e "\033[0;32mBluetooth has been successfully enabled.\033[0m"
