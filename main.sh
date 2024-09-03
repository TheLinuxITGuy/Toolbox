#!/bin/bash

APP_NAME=$1
FLATPAK_LOCATION=$2
ACTION=$3
NALA_CMD="nala"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy's Toolbox"
echo -e "\033[1;32m$ACTION $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if Nala is installed
if ! command -v $NALA_CMD &> /dev/null
then
    echo "Nala is not installed. Installing now..."
    sudo apt update
    sudo apt install -y nala
fi

if [ "$ACTION" == "install" ]; then
    # Check if the app is already installed
    if ! command -v $APP_NAME &> /dev/null
    then
        echo "$APP_NAME is not installed. Installing now..."
        if [ -z "$FLATPAK_LOCATION" ]; then
            # Install the app using nala
            sudo nala update
            sudo nala install -y $APP_NAME
            sudo nala install -f -y
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
        sudo nala remove -y $APP_NAME
    else
        flatpak uninstall -y $FLATPAK_LOCATION
    fi
fi
