# Linux Mint Scripts
Various scripts for Linux Mint.



![](<Screenshot/Screenshot%20from%202024-07-26%2018-17-09.png>)

## How do I use it?
- Download the latest release into your ~/Downloads folder
- Open a terminal
- Type: cd ~/Downloads
- Type: cd LinuxMintScripts
- Type: chmod u+x ./*.*
- Type: ./Main.py
- Select the Applications you would like to Install/Remove
- Click Run

## What does each script do?
- **Main.py:** GUI that allows you to Install/Remove applications all at once
-  **AfterInstall.sh:** Runs ALL the scripts **Use Main.py instead**
-  **install-bravebrowser.sh:** Adds the Brave repo's GPG key, Adds the Brave repo, Updates the package list, Downloads and Installs Brave
-  **install-chromebrowser.sh:** Updates the package list, Downloads the .deb package to your ~/Downloads folder, Installs Chrome along with any missing dependencies
-  **install-code:** Updates the package list, Adds Flahub report if needed, Downloads and Installs Visual Studio Code
-  **install-discord:** Updates the package list, Adds Flahub report if needed, Downloads and Installs Discord
-  **install-edge:** Updates the package list, Adds Flahub report if needed, Downloads and Installs Microsoft Edge
-  **install-gimp:** Updates the package list, Adds Flahub report if needed, Downloads and Installs GIMP
-  **install-lutris.sh:** Updates the package list, Adds Flathub repo if needed, Downloads and Installs Lutris
-  **install-steam&protonupqt.sh:** Updates the package list, Downloads and Installs Steam along with any missing dependencies, Downloads and Installs ProtonUp-Qt
-  **remove-firefox.sh:** Removes Firefox
-  **remove-libreoffice.sh:** Removes Libre Office
-  **remove-thunderbird.sh:** Removes Thunderbird Mail
