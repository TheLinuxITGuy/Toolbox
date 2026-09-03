//! Identifier and argument validation for privileged helper commands.
//!
//! These checks are defense in depth: the GUI already builds argv arrays
//! (no shell interpolation), and the helper scripts quote expansions. Rejecting
//! unexpected characters keeps package/flatpak identifiers from being treated
//! as flags or other payloads before they reach pacman/apt/dnf/flatpak.

const MAX_PACKAGE_LEN: usize = 128;
const MAX_FLATPAK_LEN: usize = 255;
const MAX_LABEL_LEN: usize = 80;
const MAX_NOTES_LEN: usize = 240;
const MAX_CATEGORY_LEN: usize = 64;

/// Helper scripts the GUI is allowed to invoke. Basename only.
pub const ALLOWED_HELPER_SCRIPTS: &[&str] = &[
    "main.sh",
    "enable-bluetooth.sh",
    "disable-bluetooth.sh",
    "install-tlp.sh",
    "install-powertop.sh",
    "update-system.sh",
    "install-nala.sh",
    "install-stacer.sh",
    "install-swapfix.sh",
    "install-fastfetch.sh",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub field: &'static str,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

fn err(field: &'static str, message: impl Into<String>) -> ValidationError {
    ValidationError {
        field,
        message: message.into(),
    }
}

/// Native package names accepted by apt/dnf/pacman.
///
/// First character must be alphanumeric so the value cannot be parsed as a
/// leading flag (`-Syu`, `--config=...`).
pub fn is_package_name(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_PACKAGE_LEN {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphanumeric() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

/// Flatpak application IDs in reverse-DNS form (at least two segments).
pub fn is_flatpak_id(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_FLATPAK_LEN || !value.contains('.') {
        return false;
    }
    value.split('.').all(|segment| {
        !segment.is_empty()
            && segment.len() <= 63
            && segment
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
            && segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
    })
}

/// Executable basename used only for icon lookup / catalog metadata.
pub fn is_exec_name(value: &str) -> bool {
    is_package_name(value)
}

pub fn is_action(value: &str) -> bool {
    matches!(value, "install" | "remove")
}

fn is_safe_display_text(value: &str, max_len: usize) -> bool {
    !value.is_empty()
        && value.len() <= max_len
        && !value.starts_with('-')
        && !value.contains("..")
        && value.chars().all(|c| c.is_ascii_graphic() || c == ' ')
        && !value.chars().any(|c| c.is_ascii_control())
}

pub fn is_label(value: &str) -> bool {
    is_safe_display_text(value, MAX_LABEL_LEN)
}

pub fn is_category(value: &str) -> bool {
    is_safe_display_text(value, MAX_CATEGORY_LEN)
}

pub fn is_notes(value: &str) -> bool {
    value.is_empty()
        || (value.len() <= MAX_NOTES_LEN
            && value
                .chars()
                .all(|c| (c.is_ascii_graphic() || c == ' ') && !c.is_ascii_control()))
}

pub fn is_safe_script_name(name: &str) -> bool {
    ALLOWED_HELPER_SCRIPTS.contains(&name)
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && !name.contains("..")
}

pub fn validate_package_name(value: &str) -> Result<(), ValidationError> {
    if is_package_name(value) {
        Ok(())
    } else {
        Err(err(
            "package",
            "must be a native package identifier (letters, digits, ._+-)",
        ))
    }
}

pub fn validate_flatpak_id(value: &str) -> Result<(), ValidationError> {
    if is_flatpak_id(value) {
        Ok(())
    } else {
        Err(err(
            "flatpak",
            "must be a reverse-DNS Flatpak application ID",
        ))
    }
}

pub fn validate_exec_name(value: &str) -> Result<(), ValidationError> {
    if is_exec_name(value) {
        Ok(())
    } else {
        Err(err(
            "exec",
            "must be a simple executable name (letters, digits, ._+-)",
        ))
    }
}

pub fn validate_action(value: &str) -> Result<(), ValidationError> {
    if is_action(value) {
        Ok(())
    } else {
        Err(err("action", "must be install or remove"))
    }
}

pub fn validate_label(value: &str) -> Result<(), ValidationError> {
    if is_label(value) {
        Ok(())
    } else {
        Err(err("label", "contains unsupported characters"))
    }
}

pub fn validate_category(value: &str) -> Result<(), ValidationError> {
    if is_category(value) {
        Ok(())
    } else {
        Err(err("category", "contains unsupported characters"))
    }
}

pub fn validate_notes(value: &str) -> Result<(), ValidationError> {
    if is_notes(value) {
        Ok(())
    } else {
        Err(err("notes", "contains unsupported characters"))
    }
}

pub fn validate_script_name(name: &str) -> Result<(), ValidationError> {
    if is_safe_script_name(name) {
        Ok(())
    } else {
        Err(err("script", "is not an allowed helper script"))
    }
}

/// Replace every occurrence of a secret in `text` so it cannot appear in logs.
pub fn redact_secret(text: &str, secret: &str) -> String {
    if secret.is_empty() {
        return text.to_owned();
    }
    text.replace(secret, "[redacted]")
}

/// Overwrite a String's contents before dropping it.
pub fn zeroize_string(value: &mut String) {
    // fill() writes NULs through the existing allocation, then clear() sets
    // len=0 without shrinking, so the secret is not left as readable UTF-8.
    let len = value.len();
    value.replace_range(.., &"\0".repeat(len));
    value.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_real_catalog_packages() {
        for name in [
            "firefox",
            "thunderbird",
            "steam",
            "audacity",
            "mpv",
            "obs-studio",
            "vlc",
            "libreoffice",
            "gparted",
            "htop",
            "brave-origin",
            "brave-origin-bin",
            "telegram-desktop",
            "retroarch",
            "dolphin-emu",
            "qbittorrent",
            "ppsspp",
        ] {
            assert!(is_package_name(name), "{name}");
        }
    }

    #[test]
    fn accepts_real_catalog_flatpak_ids() {
        for id in [
            "com.brave.Browser",
            "com.google.Chrome",
            "com.microsoft.Edge",
            "com.opera.Opera",
            "com.vivaldi.Vivaldi",
            "app.zen_browser.zen",
            "com.discordapp.Discord",
            "org.signal.Signal",
            "com.slack.Slack",
            "com.usebottles.bottles",
            "org.gnome.Boxes",
            "com.visualstudio.code",
            "com.jetbrains.PyCharm-Community",
            "net.lutris.Lutris",
            "net.davidotek.pupgui2",
            "com.valvesoftware.Steam",
            "org.gimp.GIMP",
            "com.obsproject.Studio",
            "org.onlyoffice.desktopeditors",
            "org.localsend.localsend_app",
            "org.vinegarhq.Sober",
            "com.spotify.Client",
            "com.heroicgameslauncher.hgl",
            "com.github.tchx84.Flatseal",
            "org.prismlauncher.PrismLauncher",
            "md.obsidian.Obsidian",
            "com.mattjakeman.ExtensionManager",
            "it.mijorus.gearlever",
            "com.protonvpn.www",
            "com.vysp3r.ProtonPlus",
            "com.bitwarden.desktop",
            "com.stremio.Stremio",
            "io.gitlab.librewolf-community",
            "io.missioncenter.MissionCenter",
        ] {
            assert!(is_flatpak_id(id), "{id}");
        }
    }

    #[test]
    fn rejects_package_flag_and_metacharacter_payloads() {
        for name in [
            "",
            "-Syu",
            "--noconfirm",
            "--config=/tmp/x",
            "pkg;id",
            "pkg id",
            "pkg$(id)",
            "pkg`id`",
            "pkg|id",
            "pkg&&id",
            "../pkg",
            "pkg/../other",
            "pkg\nextra",
        ] {
            assert!(!is_package_name(name), "{name:?}");
        }
    }

    #[test]
    fn rejects_invalid_flatpak_ids() {
        for id in [
            "",
            "nodot",
            "-com.evil.App",
            "com.evil;App",
            "com.evil App",
            "com..evil",
            ".com.evil",
            "com.evil.",
        ] {
            assert!(!is_flatpak_id(id), "{id:?}");
        }
    }

    #[test]
    fn action_is_install_or_remove_only() {
        assert!(is_action("install"));
        assert!(is_action("remove"));
        assert!(!is_action("update"));
        assert!(!is_action("install; rm -rf /"));
    }

    #[test]
    fn helper_script_allowlist_rejects_paths() {
        assert!(is_safe_script_name("main.sh"));
        assert!(is_safe_script_name("update-system.sh"));
        assert!(!is_safe_script_name("../main.sh"));
        assert!(!is_safe_script_name("/etc/passwd"));
        assert!(!is_safe_script_name("main.sh;id"));
        assert!(!is_safe_script_name("not-a-script.sh"));
        assert!(!is_safe_script_name("VMware Player Fix/vmware-fix.sh"));
    }

    #[test]
    fn redact_secret_removes_all_occurrences() {
        assert_eq!(
            redact_secret("auth failed for hunter2 in hunter2", "hunter2"),
            "auth failed for [redacted] in [redacted]"
        );
        assert_eq!(redact_secret("nothing to hide", ""), "nothing to hide");
        assert_eq!(redact_secret("no match", "secret"), "no match");
    }

    #[test]
    fn zeroize_string_clears_contents() {
        let mut secret = String::from("super-secret");
        zeroize_string(&mut secret);
        assert!(secret.is_empty());
    }

    #[test]
    fn labels_and_notes_reject_control_characters() {
        assert!(is_label("Brave Browser"));
        assert!(is_label("Brave Origin"));
        assert!(is_label("nala (rank mirrors) - Debian only"));
        assert!(!is_label("bad\nlabel"));
        assert!(!is_label("-sneaky"));
        assert!(is_notes("Native package"));
        assert!(is_notes(""));
        assert!(!is_notes("line1\nline2"));
    }

    #[test]
    fn bash_package_and_flatpak_validators_agree_with_rust() {
        let lib = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("toolbox-lib.sh");
        assert!(lib.is_file());

        let cases = [
            ("is_valid_package_name", "firefox", true),
            ("is_valid_package_name", "obs-studio", true),
            ("is_valid_package_name", "brave-origin", true),
            ("is_valid_package_name", "brave-origin-bin", true),
            ("is_valid_package_name", "-Syu", false),
            ("is_valid_package_name", "pkg;id", false),
            ("is_valid_flatpak_id", "com.brave.Browser", true),
            (
                "is_valid_flatpak_id",
                "com.jetbrains.PyCharm-Community",
                true,
            ),
            ("is_valid_flatpak_id", "nodot", false),
            ("is_valid_flatpak_id", "com.evil;App", false),
            ("is_valid_action", "install", true),
            ("is_valid_action", "upgrade", false),
        ];

        for (func, value, expected) in cases {
            let status = std::process::Command::new("bash")
                .arg("-c")
                .arg(format!("source \"$1\" && {func} \"$2\"",))
                .arg("validator")
                .arg(&lib)
                .arg(value)
                .status()
                .expect("run bash validator");
            assert_eq!(
                status.success(),
                expected,
                "{func}({value:?}) expected {expected}"
            );
        }
    }

    fn toolbox_lib() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("toolbox-lib.sh")
    }

    fn bash_lib(script: &str) -> std::process::Output {
        std::process::Command::new("bash")
            .arg("-c")
            .arg(format!("source \"$1\" && {script}"))
            .arg("brave-test")
            .arg(toolbox_lib())
            .output()
            .expect("run bash toolbox-lib helper")
    }

    #[test]
    fn brave_download_urls_are_pinned_and_other_urls_are_rejected() {
        let ok_script = bash_lib(
            r#"
            is_pinned_brave_download_url "$BRAVE_INSTALL_SH_URL" || exit 1
            is_pinned_brave_download_url "$BRAVE_INSTALL_SH_ASC_URL" || exit 2
            is_pinned_brave_download_url "https://evil.example/install.sh" && exit 3
            is_pinned_brave_download_url "https://dl.brave.com/other.sh" && exit 4
            dest=$(mktemp)
            if download_pinned_brave_file "https://example.com/install.sh" "$dest"; then
                rm -f "$dest"
                exit 5
            fi
            rm -f "$dest"
            [[ "$BRAVE_INSTALL_SH_URL" == "https://dl.brave.com/install.sh" ]] || exit 6
            [[ "$BRAVE_INSTALL_SH_ASC_URL" == "https://dl.brave.com/install.sh.asc" ]] || exit 7
            [[ "$BRAVE_INSTALL_SH_FINGERPRINT" == "D16166072CACDF2C9429CBF11BF41E37D039F691" ]] || exit 8
            "#,
        );
        assert!(
            ok_script.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&ok_script.stderr)
        );
    }

    #[test]
    fn brave_origin_package_candidates_match_distro() {
        let output = bash_lib(
            r#"
            brave_origin_package_candidates pacman
            echo ---
            brave_origin_package_candidates apt
            echo ---
            brave_origin_package_candidates dnf
            "#,
        );
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let chunks: Vec<&str> = stdout.split("---\n").map(str::trim).collect();
        assert_eq!(chunks[0], "brave-origin-bin\nbrave-origin");
        assert_eq!(chunks[1], "brave-origin");
        assert_eq!(chunks[2], "brave-origin");
    }

    #[test]
    fn brave_gpg_verify_fails_closed_on_tampered_script() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let key = root.join("assets/keys/brave-install.sh.asc");
        assert!(key.is_file(), "vendored Brave installer key");

        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(
                r#"
                source "$1"
                tmp=$(mktemp -d)
                printf '%s\n' '#!/bin/sh' 'echo pwned' > "$tmp/install.sh"
                printf '%s\n' 'not a signature' > "$tmp/install.sh.asc"
                if verify_brave_install_script "$tmp/install.sh" "$tmp/install.sh.asc" "$2"; then
                    rm -rf "$tmp"
                    exit 1
                fi
                rm -rf "$tmp"
                "#,
            )
            .arg("brave-verify")
            .arg(toolbox_lib())
            .arg(&key)
            .output()
            .expect("run gpg fail-closed test");
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("GPG verification of the Brave installer failed")
                || stderr.contains("Failed to import"),
            "unexpected stderr: {stderr}"
        );
    }

    #[test]
    fn brave_installer_helpers_do_not_pipe_curl_to_sh_or_eval() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let main = std::fs::read_to_string(root.join("main.sh")).unwrap();
        let lib = std::fs::read_to_string(root.join("toolbox-lib.sh")).unwrap();
        let combined = format!("{main}\n{lib}");

        assert!(
            !combined.contains("| FLAVOR=origin"),
            "must not pipe the Brave installer into a shell"
        );
        assert!(
            !combined.contains("curl -fsS https://dl.brave.com/install.sh |"),
            "must not curl|sh the Brave installer"
        );
        assert!(
            !combined.contains("eval "),
            "must not eval the Brave installer"
        );
        assert!(
            combined.contains("FLAVOR=origin CHANNEL=release"),
            "verified installer must run with FLAVOR=origin"
        );
        assert!(
            lib.contains("sudo -n"),
            "nested sudo must be non-interactive"
        );

        for url in http_urls_containing(&combined, "brave") {
            assert!(
                url == "https://dl.brave.com/install.sh"
                    || url == "https://dl.brave.com/install.sh.asc",
                "unpinned Brave download URL in helpers: {url}"
            );
        }
    }

    fn http_urls_containing<'a>(text: &'a str, needle: &str) -> Vec<&'a str> {
        text.split_whitespace()
            .filter_map(|part| {
                let start = part.find("https://")?;
                let url = part[start..].trim_end_matches(|c: char| {
                    matches!(c, '"' | '\'' | ')' | ';' | ',' | '`' | '.')
                });
                url.contains(needle).then_some(url)
            })
            .collect()
    }

    #[test]
    fn sudo_wrapper_invokes_real_sudo_with_n() {
        let output = bash_lib(
            r#"
            tmp=$(mktemp -d)
            install_noninteractive_sudo_wrapper "$tmp" || exit 1
            grep -q -- '-n' "$tmp/sudo" || exit 2
            if grep -q -- 'sudo -S' "$tmp/sudo"; then
                rm -rf "$tmp"
                exit 3
            fi
            test -x "$tmp/sudo" || exit 4
            rm -rf "$tmp"
            "#,
        );
        assert!(
            output.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn brave_gpg_verify_accepts_official_signed_installer() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let key = root.join("assets/keys/brave-install.sh.asc");
        let output = std::process::Command::new("bash")
            .arg("-c")
            .arg(
                r#"
                source "$1"
                tmp=$(mktemp -d)
                if ! download_pinned_brave_file "$BRAVE_INSTALL_SH_URL" "$tmp/install.sh"; then
                    rm -rf "$tmp"
                    exit 1
                fi
                if ! download_pinned_brave_file "$BRAVE_INSTALL_SH_ASC_URL" "$tmp/install.sh.asc"; then
                    rm -rf "$tmp"
                    exit 2
                fi
                if ! verify_brave_install_script "$tmp/install.sh" "$tmp/install.sh.asc" "$2"; then
                    rm -rf "$tmp"
                    exit 3
                fi
                # Tampering after a good download must fail closed.
                echo '# tampered' >> "$tmp/install.sh"
                if verify_brave_install_script "$tmp/install.sh" "$tmp/install.sh.asc" "$2"; then
                    rm -rf "$tmp"
                    exit 4
                fi
                rm -rf "$tmp"
                "#,
            )
            .arg("brave-verify-ok")
            .arg(toolbox_lib())
            .arg(&key)
            .output()
            .expect("run official installer verify test");
        assert!(
            output.status.success(),
            "status={} stderr={} stdout={}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
    }
}
