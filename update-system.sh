#!/bin/bash

NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mUpdating System with Nala"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null
then
    echo "Nala is not installed. Installing now..."
    sudo apt update
    sudo apt install -y nala
fi

# Update the system using Nala
sudo nala update && sudo nala upgrade -y
