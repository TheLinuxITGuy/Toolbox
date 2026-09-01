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
}
