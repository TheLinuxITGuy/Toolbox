#!/usr/bin/env bash
set -euo pipefail

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy - Linux Mint Scripts\n'
printf '[1;32mVMware Player Fix\n'
printf '[0;32m=====================================[0m\n'

DOWNLOADS="${HOME}/Downloads"
MODULE_VERSION="workstation-17.5.0"
MODULE_URL="https://github.com/mkubecek/vmware-host-modules/archive/${MODULE_VERSION}.tar.gz"
MODULE_ARCHIVE="${DOWNLOADS}/${MODULE_VERSION}.tar.gz"
MODULE_DIR="${DOWNLOADS}/vmware-host-modules-${MODULE_VERSION}"

if [[ ! -d "$DOWNLOADS" ]]; then
    echo "Expected ${DOWNLOADS} to exist and contain the VMware Player bundle." >&2
    exit 1
fi

shopt -s nullglob
bundles=("${DOWNLOADS}"/VMware-*.bundle)
if [[ ${#bundles[@]} -ne 1 ]]; then
    echo "Place exactly one VMware-*.bundle file in ${DOWNLOADS} and rerun this script." >&2
    exit 1
fi

bundle=${bundles[0]}
chmod u+x "$bundle"
sudo "$bundle"

wget -O "$MODULE_ARCHIVE" "$MODULE_URL"
tar -xzf "$MODULE_ARCHIVE" -C "$DOWNLOADS"

if [[ ! -d "$MODULE_DIR/vmmon-only" || ! -d "$MODULE_DIR/vmnet-only" ]]; then
    echo "VMware host modules archive did not contain the expected directories." >&2
    exit 1
fi

tar -cf "${MODULE_DIR}/vmmon.tar" -C "$MODULE_DIR" vmmon-only
tar -cf "${MODULE_DIR}/vmnet.tar" -C "$MODULE_DIR" vmnet-only

sudo cp -v "${MODULE_DIR}/vmmon.tar" "${MODULE_DIR}/vmnet.tar" /usr/lib/vmware/modules/source/
sudo vmware-modconfig --console --install-all

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy - Linux Mint Scripts\n'
printf '[1;32mVMware Player Fix - Complete\n'
printf '[1;32mFire up Super->Administration->VMware Player to complete the installation.\n'
printf '[0;32m=====================================[0m\n'
