#!/bin/bash

# Function to check if the script is run as root
function check_root {
    if [[ "$EUID" -ne 0 ]]; then
        echo "This script must be run as root. Please use 'sudo'."
        exit 1
    fi
}

# Function to clear the screen (you can customize this if needed)
function clear_screen {
    clear
}


# Function to clean the system
function clean_system {
    clear_screen
    echo "Cleaning orphaned and unnecessary packages..."
    apt autoremove -y -qq
    apt clean -qq
    if [[ $? -eq 0 ]]; then
        echo "System cleaned successfully."
    else
        echo "There was an error cleaning the system."
    fi
}

# Function to clean browser caches (Firefox and Chrome)
function clean_browsers {
    clear_screen
    echo "Cleaning browser caches..."
    if [ -d ~/.mozilla ]; then
        rm -rf ~/.mozilla/firefox/*.default-release/cache2/* || error_exit "Failed to clean Firefox cache."
    fi
    if [ -d ~/.cache/google-chrome ]; then
        rm -rf ~/.cache/google-chrome/* || error_exit "Failed to clean Chrome cache."
    fi
    echo "Browser caches cleaned."
}

# Function to clean system caches (systemd, font cache, etc.)
function clean_system_caches {
    clear_screen
    echo "Cleaning system caches..."
    rm -rf /var/cache/* /var/lib/systemd/coredump/* || error_exit "Failed to clean system caches."
    fc-cache -r || error_exit "Failed to rebuild font cache."
    echo "System caches cleaned."
}

# Function to optimize the system (repair broken packages, optimize disk space)
function optimize_system {
    clear_screen
    echo "Repairing broken packages..."
    apt --fix-broken install -y -qq || error_exit "Failed to repair broken packages."
    echo "Optimizing disk space..."
    e4defrag / || error_exit "Failed to optimize disk space."
    echo "System optimization complete."
}

# Function to clean Snap and Flatpak
function clean_snap_flatpak {
    clear_screen
    echo "Cleaning Snap and Flatpak caches and packages..."
    snap list --all | awk '/disabled/{print $1, $3}' | while read snapname revision; do
        snap remove "$snapname" --revision="$revision" || error_exit "Failed to remove Snap package $snapname."
    done
    flatpak uninstall --unused -y || error_exit "Failed to clean Flatpak packages."
    echo "Snap and Flatpak cleanup complete."
}

# Call the functions
clean_system
clean_browsers
clean_system_caches
optimize_system
clean_snap_flatpak

echo "All actions completed."