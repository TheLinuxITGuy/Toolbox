# Function to enable ZRAM
function enable_zram {
    clear_screen
    echo "Enabling ZRAM for memory optimization..."
    apt install zram-tools -qq
    echo -e "ALGO=zstd\nPERCENT=60" | tee /etc/default/zramswap
    service zramswap reload -qq
    if [[ $? -eq 0 ]]; then
        echo "ZRAM has been successfully enabled."
    else
        echo "There was an error enabling ZRAM."
    fi
}