#!/bin/bash

SWAPPINESS_VALUE=10
SYSCTL_CONF="/etc/sysctl.conf"
SYSCTL_CMD="sysctl -p"

# Function to display error messages and exit
function error_exit {
    echo -e "\033[0;31m$1\033[0m" >&2
    exit 1
}

# Check if the swappiness value is already set
if grep -q "^vm.swappiness=$SWAPPINESS_VALUE" "$SYSCTL_CONF"; then
    echo "The swappiness value is already set to $SWAPPINESS_VALUE."
else
    # Append the line to /etc/sysctl.conf
    echo "vm.swappiness=$SWAPPINESS_VALUE" | sudo tee -a "$SYSCTL_CONF" >/dev/null || error_exit "Failed to update $SYSCTL_CONF."

    # Apply changes immediately
    sudo $SYSCTL_CMD || error_exit "Failed to apply sysctl changes."

    echo "The swappiness value has been updated to $SWAPPINESS_VALUE."
fi

# Prompt the user to restart
echo "It is recommended to restart your system for all changes to take effect."
read -p "Do you want to restart now? (y/n): " choice

case "$choice" in
    [Yy]* )
        sudo reboot
        ;;
    [Nn]* )
        echo "Please remember to restart your system later to apply the changes."
        ;;
    * )
        echo "Invalid response. Please answer with 'y' or 'n'."
        ;;
esac
