# 🧰 The Linux IT Guy Toolbox

![GitHub Release](https://img.shields.io/github/v/release/TheLinuxITGuy/Toolbox?style=for-the-badge&labelColor=%230B0E13&color=%234285F4)
![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/TheLinuxITGuy/Toolbox/total?style=for-the-badge&labelColor=%230B0E13&color=%234285F4)

A fast Rust desktop app for installing or removing apps, changing your mouse cursor, and running Linux admin tasks with one click.

![Apps](Screenshot/readme-apps-dark.png)

> [!NOTE]
> #### Supported Distros
> ![Static Badge](https://img.shields.io/badge/Arch-%230B0E13?style=for-the-badge&logo=arch%20linux&logoColor=%234285F4)
> ![Static Badge](https://img.shields.io/badge/Debian-%230B0E13?style=for-the-badge&logo=debian&logoColor=%234285F4)
> ![Static Badge](https://img.shields.io/badge/Fedora-%230B0E13?style=for-the-badge&logo=fedora&logoColor=%234285F4)

## ✨ What It Does

- **Home** with three quick tasks (install apps, set up a new computer, make your battery last), your computer at a glance, and recent activity
- **Apps** to install or remove native packages and Flatpaks, with a floating queue pill when items are staged
- **Recipes** for one-click admin playbooks (updates, Bluetooth, TLP, Powertop, and more)
- **Cursor** to apply a Bibata theme or the KDE Plasma cursor on GNOME or Omarchy (Hyprland) immediately, or stage it with app installs and removes
- **System** page with live processor, memory, and storage details
- **Blue** by default, with Green, Orange, Teal, and a white Light skin picked from the swatches in the top bar
- Works across multiple distros

## 🚀 Quick Start

```bash
git clone https://github.com/TheLinuxITGuy/Toolbox.git
cd Toolbox
cargo build --release
./target/release/linux-it-guy-toolbox
```

Releases can be found on the [GitHub Releases](https://github.com/TheLinuxITGuy/Toolbox/releases) page.

If your distribution's `rustc`/`cargo` is older than 1.92:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
```

## 📖 Usage

1. Use the tabs along the top: **Home**, **Apps**, **Recipes**, **Cursor**, or **System**
2. On **Apps**, switch **Install** or **Uninstall**, then stage what you need
3. On **Recipes**, stage the playbooks you want to run
4. On **Cursor**, **Use ... now** installs a Bibata theme or the KDE Plasma cursor if needed and sets it immediately. **Add to queue** adds that theme to the queue so it runs in one shot with installs and removes
5. When the queue pill appears, click **Review & run**, enter your password when a queued task needs it, and follow progress in the drawer on the right (then Close when it says All done)

## 🔧 Requirements

**If `cargo build --release` fails, your distribution might be missing git or GUI build packages (xcb, xkbcommon, OpenSSL, and similar).**

### Arch
```bash
sudo pacman -Syu git rust cargo
```

### Linux Mint / Debian
```bash
sudo apt install -y git cargo rustc libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

### Fedora
```bash
sudo dnf install -y git cargo rust libxcb-devel libxkbcommon-devel openssl-devel
```

### Silverblue / bazzite / ublue
```bash
rpm-ostree install -y git cargo rust libxcb-devel libxkbcommon-devel openssl-devel
```

## 📋 App Catalog

Apps are defined in `apps_config.csv` with columns:
- `Category`, `Label`, `Package Name`, `Flatpak ID`, `Exec Name`, `Notes`

Users can add applications by editing the `apps_config.csv` file.

Package names, Flatpak IDs, and labels are validated before any helper script runs. Only run Toolbox from a directory you trust: `apps_config.csv` and the helper scripts beside the app (or, as a last resort, in the current working directory) are executed as your user and may call `sudo`. Brave Origin downloads a pinned installer from `dl.brave.com` and GPG-verifies it before running; a compromised vendor key would still be a full-system trust failure.

>[!NOTE]
> - Some apps are better as Flatpaks on certain distros
> - Some admin actions are distro-specific
> - Optional packages may not be in all default repositories

## 🎬 Video

[![Video](https://img.youtube.com/vi/PJytFBO3seM/maxresdefault.jpg)](https://youtu.be/PJytFBO3seM)

## 💝 Sponsor

https://github.com/sponsors/TheLinuxITGuy
