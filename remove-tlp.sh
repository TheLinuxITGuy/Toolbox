#!/bin/bash

TLP_CMD="tlp"

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
echo -e "\033[1;32mRemoving TLP"
echo -e "\033[0;32m=====================================\033[0m"

# Check if TLP is installed
if command -v $TLP_CMD &> /dev/null; then
    echo "TLP is installed. Removing now..."
    apt remove -y tlp || error_exit "Failed to remove TLP. Please check your system settings."
    echo "TLP has been removed successfully."
else
    echo "TLP is not installed."
fi

# Disable TLP service
echo "Disabling TLP service..."
systemctl disable tlp.service || error_exit "Failed to disable TLP service. Please check your system settings."

# Stop TLP service
echo "Stopping TLP service..."
systemctl stop tlp.service || error_exit "Failed to stop TLP service. Please check your system settings."

echo -e "\033[0;32mTLP has been removed and disabled successfully.\033[0m"
