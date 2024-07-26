#!/bin/bash

echo -e "\033[0;32m====================================="
echo -e "\033[1;32mThe Linux IT Guy - Linux Mint Scripts"
echo -e "\033[1;32mAfter Install - Run ALL Scripts"
echo -e "\033[0;32m=====================================\033[0m"

# Directory containing the scripts
dir="."

# Iterate over each .sh file in the directory
for script in "$dir"/*.sh
do
  # Only proceed if the file is executable
  if [ -x "$script" ]
  then
    echo "Running $script"
    # Run the script
    "$script"
  else
    echo "Skipping $script (not executable)"
  fi
done
