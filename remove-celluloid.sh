#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mRemoving Celluloid"
echo -e "\033[0;32m=====================================\033[0m"

# Commands to remove Celluloid
sudo apt remove --purge -y celluloid*
sudo apt clean -y
sudo apt autoremove -y