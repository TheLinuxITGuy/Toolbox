
#!/bin/bash

APP_NAME="obs-studio"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mInstalling $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if the app is already installed
if ! command -v $APP_NAME &> /dev/null
then
    echo "$APP_NAME is not installed. Installing now..."
    # Update the package database
    sudo apt update
    # Install the app
    sudo apt install -y obs-studio
    # Install any missing dependencies and finish configuring the package
    sudo apt install -f -y
else
    echo "$APP_NAME is already installed. Skipping installation."
fi