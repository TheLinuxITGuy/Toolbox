#!/bin/bash

# Update the package list
sudo apt update

# Install flatpak
sudo apt install -y flatpak

# Add the Flathub repository
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Install Lutris
flatpak install -y flathub net.lutris.Lutris