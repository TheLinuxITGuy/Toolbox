#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mVMware Player Fix"
echo -e "\033[0;32m=====================================\033[0m"

# Change permissions to make the bundle executable
chmod u+x ~/Downloads/VMware-*.bundle

# Check if VMware Player bundle exists in ~/Downloads
if [ ! -f ~/Downloads/VMware-*.bundle ]; then
    echo "Run the script from ~/Downloads/"
    exit 1
fi

# Run the VMware Player installer
sudo ~/Downloads/VMware-*.bundle

# Change directory to ~/Downloads
cd ~/Downloads 

# Download the VMware host modules
wget https://github.com/mkubecek/vmware-host-modules/archive/workstation-17.5.0.tar.gz

# Extract the tarball
tar -xzf workstation-17.5.0.tar.gz

# Change directory to the extracted folder
cd vmware-host-modules-workstation-17.5.0/

# Create tarballs for vmmon and vmnet
tar -cf vmmon.tar vmmon-only/
tar -cf vmnet.tar vmnet-only/

# Copy the tarballs to the VMware modules source directory
sudo cp -v vmmon.tar vmnet.tar  /usr/lib/vmware/modules/source/

# Run VMware modconfig to install all modules
sudo vmware-modconfig --console --install-all

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mVMware Player Fix - Complete"
echo -e "\033[1;32mFire up Super->Administration->VMware Player to complete the installation."
echo -e "\033[0;32m=====================================\033[0m"
