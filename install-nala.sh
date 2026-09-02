#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
# shellcheck source=toolbox-lib.sh
source "${SCRIPT_DIR}/toolbox-lib.sh"

printf '[0;32m=====================================\n'
printf '[1;32mThe Linux IT Guy Toolbox\n'
printf '[1;32mConfigure nala mirrors\n'
printf '[0;32m=====================================[0m\n'

if ! command -v apt-get >/dev/null 2>&1; then
    echo "This task is intended for Debian-based systems only."
    exit 1
fi

require_package_name "nala"

if ! command -v nala >/dev/null 2>&1; then
    echo "Installing nala..."
    sudo apt-get update
    sudo apt-get install -y -- nala
fi

sudo nala fetch --auto
