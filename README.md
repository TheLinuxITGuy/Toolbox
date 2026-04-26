# The Linux IT Guy Toolbox
![GitHub Release](https://img.shields.io/github/v/release/TheLinuxITGuy/Toolbox?style=for-the-badge&labelColor=%231A365D&color=%23E9FC12)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/TheLinuxITGuy/Toolbox/total?style=for-the-badge&labelColor=%231A365D&color=%23E9FC12)

![Preview](Screenshot/Screenshot6.png)

**The Linux IT Guy Toolbox** is a Rust desktop app for installing apps, removing apps, and running practical Linux admin tasks without memorizing package names or Flatpak IDs.

**As of 4/26/26** - The app is being rewritten in Rust with an `egui/eframe` interface. The goal is a fast single-binary Linux toolbox without the old Python/PySide6 prerequisite.

## What it does

- Install native packages and Flatpak apps from one UI
- Remove native packages and Flatpak apps from one UI
- Run common admin helpers like system updates, Bluetooth toggles, and TLP setup
- Show lightweight local system information
- Work across Debian-based, Arch-based, Fedora-based, and Nix/NixOS systems

## Recent improvements

- Modern Rust UI with sidebar navigation, search, category filters, task summary, and a cleaner process log
- More reliable native package detection across `apt`, `pacman`, `dnf`, and basic `nix-env`
- Better separation between user-facing labels, package names, and Flatpak IDs
- Improved Flatpak handling with Flathub support
- Better selection controls, task summary, and general UI polish
- More robust path handling when launching the app from outside the project directory

## Requirements

### Runtime

The Rust app does not require Python or PySide6. It still expects the helper shell scripts and `apps_config.csv` to be available next to the binary when packaged.

Install/remove tasks may still require:

- `sudo`
- your distro package manager (`apt-get`, `pacman`, `dnf`, or `nix-env`)
- `flatpak` for Flatpak apps

### Build

You need a Rust toolchain new enough for `eframe 0.34.1`.

### Arch
```bash
sudo pacman -Syu git rust cargo
```

### Debian / Ubuntu / Linux Mint
```bash
sudo apt update && sudo apt install -y git cargo rustc libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

### Fedora
```bash
sudo dnf install -y git cargo rust libxcb-devel libxkbcommon-devel openssl-devel
```

### NixOS - In testing
```bash
nix-shell -p cargo rustc pkg-config xorg.libxcb libxkbcommon openssl
```
To run on NixOS without installing: 
```bash
nix run github:TheLinuxITGuy/Toolbox --extra-experimental-features 'nix-command flakes'
```

## Install

```bash
git clone https://github.com/TheLinuxITGuy/Toolbox.git
cd Toolbox
chmod +x linux-it-guy-toolbox
./linux-it-guy-toolbox
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
- The app resolves files relative to the compiled binary first, then the current working directory for development runs.
- Some package availability may vary by distro or repository configuration.
- Most install and administration tasks require sudo access.

## Known limitations

- Nix/NixOS support is basic right now: native installs use `nix-env -iA nixpkgs.<package>`, which works best when the CSV package name matches a nixpkgs attribute.
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
