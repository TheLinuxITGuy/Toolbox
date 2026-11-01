#!/bin/bash
# Fix Broadcom BCM4331 wifi: stop the bcma driver from grabbing the card so wl can use it.
set -e

echo "== Making the fix permanent =="
sudo tee /etc/modprobe.d/broadcom-wl-fix.conf >/dev/null <<'EOF'
# bcma grabs the BCM4331 before wl can. Block it completely.
blacklist bcma
install bcma /bin/true
blacklist b43
blacklist ssb
EOF

echo "== Rebuilding boot image (takes a minute) =="
sudo limine-update

echo "== Switching drivers now =="
sudo modprobe -r wl 2>/dev/null || true
sudo rmmod bcma 2>/dev/null || true
sudo modprobe wl
sleep 3

echo "== Turning wifi on =="
sudo rfkill unblock wifi || true
nmcli radio wifi on || true
sleep 3

echo
ip -br link
echo
nmcli dev wifi list || true
echo
echo "Done. If you see a wlan/wlp line above, wifi is working."
echo "If not, reboot and it should appear."
