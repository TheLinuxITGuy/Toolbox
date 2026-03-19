#!/usr/bin/env bash
set -euo pipefail

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy Toolbox\n'
printf '[1;32mDisable Bluetooth\n'
printf '[0;32m=====================================[0m\n'

sudo rfkill block bluetooth
sudo systemctl disable --now bluetooth.service

echo "Bluetooth has been disabled."
