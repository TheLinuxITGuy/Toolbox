#!/bin/bash

APP_NAME=$1
FLATPAK_LOCATION=$2
ACTION=$3
NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy Toolbox"
echo -e "\033[1;32m$ACTION $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Function to detect the package manager
detect_package_manager() {
    if command -v apt &> /dev/null; then
        echo "apt"
    elif command -v pacman &> /dev/null; then
        echo "pacman"
    elif command -v dnf &> /dev/null; then
        echo "dnf"
    else
        echo "unknown"
    fi
}

PACKAGE_MANAGER=$(detect_package_manager)

# Check if Nala is installed (only for Debian-based systems)
if [ "$PACKAGE_MANAGER" == "apt" ] && ! command -v $NALA_CMD &> /dev/null; then
    echo "Nala is not installed. Installing now..."
    sudo apt update
    sudo apt install -y nala
fi

# Check if Flatpak is installed
if ! command -v flatpak &> /dev/null; then
    echo "Flatpak is not installed. Installing now..."
    case $PACKAGE_MANAGER in
        apt)
            sudo nala update
            sudo nala install -y flatpak
            ;;
        pacman)
            sudo pacman -Syu --noconfirm
            sudo pacman -S --noconfirm flatpak
            ;;
        dnf)
            sudo dnf check-update
            sudo dnf install -y flatpak
            ;;
        *)
            echo "Unsupported package manager."
            exit 1
            ;;
    esac
fi

flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

if [ "$ACTION" == "install" ]; then
    # Check if the app is already installed
    if ! command -v $APP_NAME &> /dev/null; then
        echo "$APP_NAME is not installed. Installing now..."
        if [ -z "$FLATPAK_LOCATION" ]; then
            # Install the app using the detected package manager
            case $PACKAGE_MANAGER in
                apt)
                    sudo nala update
                    sudo nala install -y $APP_NAME
                    sudo nala install -f -y
                    ;;
                pacman)
                    if [ "$APP_NAME" == "steam" ]; then
                        # enable_multilib
                        if ! grep -q "^\[multilib\]" /etc/pacman.conf; then
                            echo "Enabling Multilib repository..."
                            sudo sed -i '/\[multilib\]/,/Include/s/^#//' /etc/pacman.conf
                            echo "Multilib repository enabled. Updating package database..."
                            sudo pacman -Syu --noconfirm
                        else
                            echo "Multilib repository is already enabled."
                        fi
                    fi
                    sudo pacman -Syu --noconfirm
                    sudo pacman -S --noconfirm $APP_NAME
                    ;;
                dnf)
                    sudo dnf check-update
                    sudo dnf install -y $APP_NAME
                    ;;
                *)
                    echo "Unsupported package manager."
                    exit 1
                    ;;
            esac
        else
            # Install the app using flatpak
            flatpak install -y $FLATPAK_LOCATION
        fi
    else
        echo "$APP_NAME is already installed. Skipping installation."
    fi
elif [ "$ACTION" == "remove" ]; then
    # Remove the app
    if [ -z "$FLATPAK_LOCATION" ]; then
        case $PACKAGE_MANAGER in
            apt)
                sudo nala remove -y $APP_NAME
                ;;
            pacman)
                sudo pacman -R --noconfirm $APP_NAME
                ;;
            dnf)
                sudo dnf remove -y $APP_NAME
                ;;
            *)
                echo "Unsupported package manager."
                exit 1
                ;;
        esac
    else
        flatpak uninstall -y $FLATPAK_LOCATION
    fi
fi
