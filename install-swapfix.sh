#!/usr/bin/env bash
set -euo pipefail

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy Toolbox\n'
printf '[1;32mApplying SWAP Fix\n'
printf '[0;32m=====================================[0m\n'

if ! command -v sysctl >/dev/null 2>&1; then
    echo "sysctl is not available on this system."
    exit 1
fi

if [[ -f /etc/arch-release ]]; then
    target_file="/etc/sysctl.d/99-swappiness.conf"
else
    target_file="/etc/sysctl.d/99-toolbox-swappiness.conf"
fi

echo "Writing vm.swappiness=10 to ${target_file}..."
printf 'vm.swappiness=10\n' | sudo tee "$target_file" >/dev/null
sudo sysctl --system >/dev/null
echo "SWAP fix applied. A reboot is optional, but not required."
