#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mAfter Install - Run ALL Scripts"
echo -e "\033[0;32m=====================================\033[0m"

# Directory containing the scripts
dir="."

# Array of script names
scripts=("install-bravebrowser.sh" "install-chromebrowser.sh" "install-code.sh" "install-edge.sh" "install-gimp.sh" "install-lutris.sh" "install-steam&protonupqt.sh" "remove-apps.sh")

# Iterate over each script in the array
for script in "${scripts[@]}"
do
  # Full path to the script
  full_path="$dir/$script"

  # Only proceed if the file is executable
  if [ -x "$full_path" ]
  then
    echo "Running $script"
    # Run the script and wait for it to finish
    "$full_path"
  else
    echo "Skipping $script (not executable or does not exist)"
  fi
done

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mAfter Install - Scripts Completed"
echo -e "\033[0;32m=====================================\033[0m"
