#!/bin/bash

# Commands to remove Libre Office
sudo apt remove --purge libreoffice*
sudo apt clean
sudo apt autoremove

# Commands to remove Thunderbird Mail
sudo apt remove --purge thunderbird*
sudo apt clean
sudo apt autoremove