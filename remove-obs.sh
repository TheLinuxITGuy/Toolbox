
#!/bin/bash

APP_NAME="obs-studio"

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mRemoving $APP_NAME"
echo -e "\033[0;32m=====================================\033[0m"

# Check if the app is already installed
if dpkg -l | grep -q obs-studio; then
    echo "$APP_NAME is installed. Removing now..."
    # Commands to remove the app
    sudo apt remove --purge -y $APP_NAME
    sudo apt clean -y
    sudo apt autoremove -y
else
    echo "$APP_NAME is not installed. Skipping removal."
fi