# Function to disable ZRAM
function disable_zram {
    clear_screen
    echo "Disabling ZRAM and removing unnecessary packages..."
    apt remove --purge zram-tools -qq
    apt autoremove -y -qq
    apt clean -qq
    if [[ $? -eq 0 ]]; then
        echo "ZRAM has been disabled and unnecessary packages removed successfully."
    else
        echo "There was an error disabling ZRAM and removing unnecessary packages."
    fi
}