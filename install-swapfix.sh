#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy Toolbox'"
echo -e "\033[1;32mApplying SWAP Fix"
echo -e "\033[0;32m=====================================\033[0m"

# Function to update swappiness value
update_swappiness() {
    echo "vm.swappiness=10" | sudo tee -a /etc/sysctl.conf
    echo "The swappiness value has been updated. It is recommended to restart your system for the changes to take effect."
    read -p "Do you want to restart now? (y/n): " choice

    if [[ "$choice" == "y" || "$choice" == "Y" ]]; then
        sudo reboot
    else
        echo "Please remember to restart your system later to apply the changes."
    fi
}

# Function to update swappiness value in Arch
# https://wiki.archlinux.org/title/Swap#Swappiness
# https://arcolinuxforum.com/viewtopic.php?t=1480
update_swappiness_arch() {
    echo "vm.swappiness=10" | sudo tee -a /etc/sysctl.d/99-swappiness.conf
    echo "The swappiness value has been updated. It is recommended to restart your system for the changes to take effect."
    read -p "Do you want to restart now? (y/n): " choice

    if [[ "$choice" == "y" || "$choice" == "Y" ]]; then
        sudo reboot
    else
        echo "Please remember to restart your system later to apply the changes."
    fi
}

# Detect distribution and update accordingly
if [ -f /etc/debian_version ]; then
    echo "Detected Debian/Ubuntu-based distribution."
    update_swappiness
elif [ -f /etc/arch-release ]; then
    echo "Detected Arch-based distribution."
    update_swappiness_arch
elif [ -f /etc/fedora-release ]; then
    echo "Detected Fedora-based distribution."
    update_swappiness
else
    echo "Unsupported distribution."
fi
