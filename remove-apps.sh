#!/bin/bash

# Commands to remove Libre Office
sudo apt remove --purge -y libreoffice*
sudo apt clean -y
sudo apt autoremove -y

# Commands to remove Firefox
sudo apt remove --purge -y firefox*
sudo apt clean -y
sudo apt autoremove -y

# Commands to remove Thunderbird Mail
sudo apt remove --purge -y thunderbird*
sudo apt clean -y
sudo apt autoremove -y
