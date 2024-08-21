# VMware Player Fix
This fix will work for Linux Mint 22 and Ubuntu 24.04. This works as of 7/26/24.
## Download
- Create a broadcom account here: https://profile.broadcom.com/web/registration
- Download the latest version of VMware Player here: https://support.broadcom.com/group/ecx/productdownloads?subfamily=VMware+Workstation+Pro

## What does this script do?
- This script will patch the **vmmon** and **vmnet** VMware modules required to run VMware Player.

## How do I run it?
1. Download **VMware Player** and **vmware-fix.sh** to ~/Downloads
2. Open a **Terminal** from ~/Downloads:
    1. Type: `chmod u+x vmware-fix.sh`
    2. Type: `./vmware-fix.sh`
4. Once the script is complete, fire up **VMware Player** (SUPER->Administrator->VMware Player)
