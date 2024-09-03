#!/bin/bash

NALA_CMD="nala"

# Check if the script is being run as root
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root."
   exit 1
fi

# Header
echo "====================================="
echo "The Linux IT Guy - Linux Mint Scripts"
echo "Updating System with Nala"
echo "====================================="

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null; then
    echo "Nala is not installed. Installing now..."
    apt update &> /dev/null
    if apt install -y nala &> /dev/null; then
        echo "Nala has been installed successfully."
    else
        echo "Failed to install Nala. Please check your internet connection or package manager settings."
        exit 1
    fi
else
    echo "Nala is already installed."
fi

# Update the system using Nala
echo "Updating system with Nala..."
if nala update &> /dev/null && nala upgrade -y &> /dev/null; then
    echo "System updated successfully with Nala."
else
    echo "There was an issue during the update process. Please try again later."
    exit 1
fi
