#!/bin/bash

NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mCleaning Up System"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null
then
    echo "Nala is not installed. Installing now..."
    sudo apt update
    sudo apt install -y nala
fi

# Update the package list using Nala
sudo nala update

# Clean package cache and remove unnecessary packages
sudo nala autoremove -y
sudo nala clean

# Clean system logs
echo "Cleaning system logs..."
sudo journalctl --vacuum-time=7d

# Remove unused Flatpak runtimes
echo "Removing unused Flatpak runtimes..."
flatpak uninstall --unused -y

# Clean thumbnail cache
echo "Cleaning thumbnail cache..."
rm -rf ~/.cache/thumbnails/*

# Clear systemd journal logs
echo "Clearing systemd journal logs..."
sudo journalctl --vacuum-size=100M

# Remove old system logs
echo "Removing old system logs..."
sudo find /var/log -type f -name "*.log" -exec truncate -s 0 {} \;

# Clean apt cache
echo "Cleaning apt cache..."
sudo apt-get clean

echo -e "\033[0;32mSystem cleanup completed!\033[0m"