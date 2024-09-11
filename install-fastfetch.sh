#!/bin/bash

APP_NAME="fastfetch"
NALA_CMD="nala"
PACMAN_CMD="pacman"
DNF_CMD="dnf"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy Toolbox"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Function to install fastfetch on Debian/Ubuntu-based systems
install_nala() {
    if ! command -v $NALA_CMD &> /dev/null
    then
        echo "Nala is not installed. Installing now..."
        sudo apt update
        sudo apt install -y nala
    fi

    if ! command -v $APP_NAME &> /dev/null
    then
        echo "$APP_NAME is not installed. Installing now..."
        sudo nala update
        sudo nala install -y git cmake gcc g++ libpci-dev libx11-dev libwayland-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev
        sudo nala install -f -y

        git clone https://github.com/fastfetch-cli/fastfetch.git
        cd fastfetch

        mkdir -p build
        cd build
        cmake ..
        make
        sudo make install

    else
        echo "$APP_NAME is already installed. Skipping installation."
    fi
}

# Function to install fastfetch on Arch-based systems
install_pacman() {
    if ! command -v $PACMAN_CMD &> /dev/null
    then
        echo "Pacman is not installed. Please install it first."
        exit 1
    fi

    if ! command -v $APP_NAME &> /dev/null
    then
        echo "$APP_NAME is not installed. Installing now..."
        sudo pacman -Syu --noconfirm
        sudo pacman -S --noconfirm $APP_NAME
    else
        echo "$APP_NAME is already installed. Skipping installation."
    fi
}

# Function to install fastfetch on Fedora-based systems
install_dnf() {
    if ! command -v $DNF_CMD &> /dev/null
    then
        echo "DNF is not installed. Please install it first."
        exit 1
    fi

    if ! command -v $APP_NAME &> /dev/null
    then
        echo "$APP_NAME is not installed. Installing now..."
        sudo dnf upgrade --refresh -y
        sudo dnf install -y $APP_NAME
    else
        echo "$APP_NAME is already installed. Skipping installation."
    fi
}

# Detect distribution and install accordingly
if [ -f /etc/debian_version ]; then
    install_nala
elif [ -f /etc/arch-release ]; then
    install_pacman
elif [ -f /etc/fedora-release ]; then
    install_dnf
else
    echo "Unsupported distribution."
fi