# The Linux IT Guy Toolbox
![GitHub Release](https://img.shields.io/github/v/release/TheLinuxITGuy/Toolbox?style=for-the-badge&labelColor=%231A365D&color=%23E9FC12)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/TheLinuxITGuy/Toolbox/total?style=for-the-badge&labelColor=%231A365D&color=%23E9FC12)

![Preview](Screenshot/Screenshot5.png)

**The Linux IT Guy Toolbox** is a Python GUI for installing apps, removing apps, and running practical Linux admin tasks without memorizing package names or Flatpak IDs.

**As of 4/26/26** - I'm doing a deepdive into NixOS video and realized the Toolbox doesn't currently have support for NixOS. I also realized the pyside6 requirement is kinda annoying so I'm looking into what it would take to rewrite the app in Rust. **One binary, no prereqs, simple, and super fast.**

## What it does

- Install native packages and Flatpak apps from one UI
- Remove native packages and Flatpak apps from one UI
- Run common admin helpers like system updates, Bluetooth toggles, and TLP setup
- Show lightweight local system information
- Work across Debian-based, Arch-based, and Fedora-based distributions

## Recent improvements

- More reliable native package detection across `apt`, `pacman`, and `dnf`
- Better separation between user-facing labels, package names, and Flatpak IDs
- Improved Flatpak handling with Flathub support
- GNOME dark mode detection and a matching dark UI palette
- Better selection controls, task summary, and general UI polish
- More robust path handling when launching the app from outside the project directory

## Requirements

### Arch
```bash
sudo pacman -Syu git pyside6
```

### Debian / Ubuntu / Linux Mint
```bash
sudo apt update && sudo apt install -y git python3-pyside6
```

### Fedora
```bash
sudo dnf install -y git python3-pyside6
```

> If PySide6 is missing, the app will try to bootstrap it automatically.

## Install

```bash
git clone https://github.com/TheLinuxITGuy/Toolbox.git
cd Toolbox
python3 Main.py
```

## Usage

1. Open the **Install** tab and choose native packages or Flatpaks.
2. Open the **Remove** tab to uninstall apps.
3. Open **Administration** for system tasks.
4. Click **Run Selected Tasks** and enter your sudo password when prompted.
5. Review progress in the built-in process log.

## App catalog

The application catalog lives in `apps_config.csv`.

Current columns:
- `Category`
- `Label`
- `Package Name`
- `Flatpak ID`
- `Exec Name`
- `Notes`

This structure keeps user-facing labels separate from package-manager package names and Flatpak IDs, which makes the catalog easier to maintain as the project grows.

## Notes

- Flatpak installs use the Flathub remote.
- Native package install detection uses package-manager queries instead of guessing from executable names.
- The GUI resolves files relative to `Main.py`, so it does not depend on your current working directory.
- Some package availability may vary by distro or repository configuration.
- Most install and administration tasks require sudo access.

## Known limitations

- Some applications are available as native packages on one distro but are better handled as Flatpaks on another.
- A few administration actions are distro-specific by design.
- Optional packages may not exist in every default repository.

## Video
[![Video](https://img.youtube.com/vi/PJytFBO3seM/maxresdefault.jpg)](https://youtu.be/PJytFBO3seM)

## Supported distros
![Static Badge](https://img.shields.io/badge/Arch-%231A365D?style=for-the-badge&logo=arch%20linux&logoColor=%23E9FC12)
![Static Badge](https://img.shields.io/badge/Debian-%231A365D?style=for-the-badge&logo=debian&logoColor=%23E9FC12)
![Static Badge](https://img.shields.io/badge/Fedora-%231A365D?style=for-the-badge&logo=fedora&logoColor=%23E9FC12)

## Sponsor
https://www.paypal.com/donate/?hosted_button_id=WPTX2BMBARSG2
