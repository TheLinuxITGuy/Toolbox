#!/bin/bash

# Append the line to /etc/sysctl.conf
echo "vm.swappiness=10" | sudo tee -a /etc/sysctl.conf

# Prompt the user to restart
echo "The swappiness value has been updated. It is recommended to restart your system for the changes to take effect."
read -p "Do you want to restart now? (y/n): " choice

if [[ "$choice" == "y" || "$choice" == "Y" ]]; then
    sudo reboot
else
    echo "Please remember to restart your system later to apply the changes."
fi
