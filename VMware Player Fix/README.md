# VMware Player Fix
This fix will work for Linux Mint and Ubuntu 24.04.
## Download
- Create a broadcom account here: https://profile.broadcom.com/web/registration
- Download the latest version of VMware Player here: https://support.broadcom.com/group/ecx/productdownloads?subfamily=VMware+Workstation+Pro

## What does this script do?
- This script will patch the **vmmon** and **vmnet** VMware modules required to run VMware Player.

## How do I run it?
- Download VMware Player to ~/Downloads
- Open a terminal from ~/Downloads
- Type: ./vmware-fix.sh
- Enter your sudo password
- Once the script finishes, fire up VMware Player from SUPER->Administrator
