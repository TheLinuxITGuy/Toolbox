//! Local system inspection helpers. These never take user-controlled command
//! strings and never invoke a shell.

use std::{env, ffi::OsStr, fs, path::Path, process::Command};

pub fn distro_name() -> String {
    let Ok(contents) = fs::read_to_string("/etc/os-release") else {
        return "Linux".to_owned();
    };

    let mut fallback = None;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return trim_release_value(value);
        }
        if let Some(value) = line.strip_prefix("NAME=") {
            fallback = Some(trim_release_value(value));
        }
    }
    fallback.unwrap_or_else(|| "Linux".to_owned())
}

fn trim_release_value(value: &str) -> String {
    value.trim().trim_matches('"').to_owned()
}

pub fn detect_package_manager() -> String {
    for manager in ["apt-get", "pacman", "dnf"] {
        if command_exists(manager) {
            return manager.to_owned();
        }
    }
    "unknown".to_owned()
}

pub fn command_exists(name: &str) -> bool {
    match env::var_os("PATH") {
        Some(path) => command_exists_on_path(name, &path),
        None => {
            !name.is_empty()
                && !name.contains('/')
                && !name.contains('\0')
                && Path::new(name).is_file()
        }
    }
}

pub fn command_exists_on_path(name: &str, path_value: impl AsRef<OsStr>) -> bool {
    if name.is_empty() || name.contains('/') || name.contains('\0') {
        return false;
    }

    env::split_paths(path_value.as_ref()).any(|dir| dir.join(name).is_file())
}

pub fn command_output(program: &str, args: &[&str]) -> String {
    if program.is_empty() || program.contains('/') || program.contains('\0') {
        return "Unknown".to_owned();
    }

    Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unknown".to_owned())
}

pub fn env_or_unknown(keys: &[&str]) -> String {
    for key in keys {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_owned();
            }
        }
    }
    "Unknown".to_owned()
}

pub fn uptime() -> String {
    let Ok(contents) = fs::read_to_string("/proc/uptime") else {
        return "Unknown".to_owned();
    };
    let Some(first) = contents.split_whitespace().next() else {
        return "Unknown".to_owned();
    };
    let Ok(seconds) = first.parse::<f64>() else {
        return "Unknown".to_owned();
    };

    let total = seconds as u64;
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;

    match (days, hours) {
        (0, 0) => format!("{minutes}m"),
        (0, _) => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h {minutes}m"),
    }
}

pub fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_exists_rejects_path_payloads() {
        assert!(!command_exists(""));
        assert!(!command_exists("/bin/sh"));
        assert!(!command_exists("sh\0id"));
    }

    #[test]
    fn command_exists_finds_sh() {
        assert!(command_exists("sh"));
    }

    #[test]
    fn command_exists_is_false_for_flatpak_on_empty_path() {
        assert!(!command_exists_on_path(
            "flatpak",
            "/var/empty-toolbox-no-bin"
        ));
        assert!(!command_exists_on_path("flatpak", ""));
    }

    #[test]
    fn command_output_does_not_run_shell_strings() {
        assert_eq!(command_output("sh;id", &[]), "Unknown");
        assert_eq!(command_output("/bin/echo", &["hi"]), "Unknown");
    }

    #[test]
    fn strip_ansi_removes_escape_sequences() {
        assert_eq!(
            strip_ansi("\u{1b}[0;32mhello\u{1b}[0m world"),
            "hello world"
        );
    }

    #[test]
    fn trim_release_value_strips_quotes() {
        assert_eq!(trim_release_value("\"Arch Linux\""), "Arch Linux");
    }
}
