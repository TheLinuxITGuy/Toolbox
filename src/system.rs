//! Local system inspection helpers. These never take user-controlled command
//! strings and never invoke a shell.

use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::validate::{is_flatpak_id, is_package_name};

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
    format_uptime(uptime_parts(), false)
}

pub fn uptime_compact() -> String {
    format_uptime(uptime_parts(), true)
}

fn uptime_parts() -> Option<(u64, u64, u64)> {
    let contents = fs::read_to_string("/proc/uptime").ok()?;
    let first = contents.split_whitespace().next()?;
    let seconds = first.parse::<f64>().ok()?;
    let total = seconds as u64;
    Some((
        total / 86_400,
        (total % 86_400) / 3_600,
        (total % 3_600) / 60,
    ))
}

fn format_uptime(parts: Option<(u64, u64, u64)>, compact: bool) -> String {
    let Some((days, hours, minutes)) = parts else {
        return "Unknown".to_owned();
    };
    if compact {
        return match (days, hours) {
            (0, 0) => format!("{minutes}m"),
            (0, _) => format!("{hours}h"),
            (_, 0) => format!("{days}d"),
            _ => format!("{days}d {hours}h"),
        };
    }
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RemovableSource {
    Native,
    Flatpak,
}

impl RemovableSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Flatpak => "flatpak",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovableApp {
    pub label: String,
    pub source: RemovableSource,
    pub detail: String,
    pub exec: Option<String>,
    pub is_toolbox: bool,
}

impl RemovableApp {
    pub fn icon_queries(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.label.as_str())
            .chain(std::iter::once(self.detail.as_str()))
            .chain(self.exec.as_deref())
    }
}

/// Scan apps that are actually installed. Never consults apps_config.csv.
pub fn scan_removable_apps(package_manager: &str) -> Result<Vec<RemovableApp>, String> {
    let mut apps = Vec::new();
    if command_exists("flatpak") {
        apps.extend(scan_flatpak_apps()?);
    }
    apps.extend(scan_native_apps(package_manager)?);
    apps.sort_by(|left, right| {
        left.label
            .to_ascii_lowercase()
            .cmp(&right.label.to_ascii_lowercase())
            .then(left.detail.cmp(&right.detail))
    });
    Ok(apps)
}

fn scan_flatpak_apps() -> Result<Vec<RemovableApp>, String> {
    let stdout = command_stdout("flatpak", &["list", "--app", "--columns=application,name"])
        .map_err(|error| format!("flatpak list failed: {error}"))?;
    let mut apps = Vec::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (id, name) = split_flatpak_list_row(line);
        if !is_flatpak_id(id) {
            continue;
        }
        let label = if name.is_empty() {
            id.to_owned()
        } else {
            name.to_owned()
        };
        apps.push(RemovableApp {
            is_toolbox: id == "linux-it-guy-toolbox",
            label,
            source: RemovableSource::Flatpak,
            detail: id.to_owned(),
            exec: None,
        });
    }
    Ok(apps)
}

fn split_flatpak_list_row(line: &str) -> (&str, &str) {
    if let Some((id, name)) = line.split_once('\t') {
        return (id.trim(), name.trim());
    }
    if let Some((id, name)) = line.split_once("  ") {
        return (id.trim(), name.trim());
    }
    (line.trim(), "")
}

fn scan_native_apps(package_manager: &str) -> Result<Vec<RemovableApp>, String> {
    let packages = user_installed_packages(package_manager)?;
    let exe = env::current_exe().ok();
    let mut apps = Vec::new();
    for package in packages {
        if !is_package_name(&package) {
            continue;
        }
        let Some((label, exec, files)) = visible_desktop_for_package(package_manager, &package)
        else {
            continue;
        };
        let is_toolbox = package == "linux-it-guy-toolbox"
            || package == "linux-it-guy-toolbox-bin"
            || exe
                .as_ref()
                .is_some_and(|path| package_owns_executable(&files, path, exec.as_deref()));
        apps.push(RemovableApp {
            label,
            source: RemovableSource::Native,
            detail: package,
            exec,
            is_toolbox,
        });
    }
    Ok(apps)
}

fn user_installed_packages(package_manager: &str) -> Result<Vec<String>, String> {
    match package_manager {
        "apt-get" | "apt" => parse_package_lines(&command_stdout("apt-mark", &["showmanual"])?),
        "pacman" => parse_package_lines(&command_stdout("pacman", &["-Qqe"])?),
        "dnf" => dnf_user_installed_packages(),
        "unknown" => Ok(Vec::new()),
        other => Err(format!(
            "Unsupported package manager for remove scan: {other}"
        )),
    }
}

fn dnf_user_installed_packages() -> Result<Vec<String>, String> {
    if let Ok(stdout) = command_stdout(
        "dnf",
        &["repoquery", "--userinstalled", "--qf", "%{name}\n"],
    ) {
        let packages = parse_package_lines(&stdout)?;
        if !packages.is_empty() || stdout.trim().is_empty() {
            return Ok(packages);
        }
    }

    let history = command_stdout("dnf", &["history", "userinstalled"])?;
    let requested = parse_package_lines(&history)?;
    if requested.is_empty() {
        return Ok(requested);
    }

    match command_stdout("rpm", &["-qa", "--qf", "%{NAME}\n"]) {
        Ok(installed) => {
            let installed = parse_package_lines(&installed)?;
            Ok(requested
                .into_iter()
                .filter(|name| installed.iter().any(|pkg| pkg == name))
                .collect())
        }
        Err(_) => Ok(requested),
    }
}

fn parse_package_lines(stdout: &str) -> Result<Vec<String>, String> {
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| is_package_name(line))
        .map(ToOwned::to_owned)
        .collect())
}

fn visible_desktop_for_package(
    package_manager: &str,
    package: &str,
) -> Option<(String, Option<String>, Vec<PathBuf>)> {
    let files = package_files(package_manager, package).ok()?;
    let mut chosen = None;
    for path in files.iter().filter(|path| is_application_desktop(path)) {
        let Ok(contents) = fs::read_to_string(path) else {
            continue;
        };
        let parsed = parse_desktop_entry(&contents);
        if parsed.no_display {
            continue;
        }
        let label = if parsed.name.is_empty() {
            package.to_owned()
        } else {
            parsed.name
        };
        let matches_name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem == package);
        if matches_name {
            chosen = Some((label, parsed.exec, files.clone()));
            break;
        }
        if chosen.is_none() {
            chosen = Some((label, parsed.exec, files.clone()));
        }
    }
    chosen
}

fn package_files(package_manager: &str, package: &str) -> Result<Vec<PathBuf>, String> {
    let stdout = match package_manager {
        "apt-get" | "apt" => command_stdout("dpkg", &["-L", package])?,
        "pacman" => command_stdout("pacman", &["-Ql", package])?,
        "dnf" => command_stdout("rpm", &["-ql", package])?,
        _ => return Ok(Vec::new()),
    };
    Ok(stdout.lines().filter_map(extract_file_path).collect())
}

fn extract_file_path(line: &str) -> Option<PathBuf> {
    let path = if let Some(idx) = line.find('/') {
        line[idx..].trim()
    } else {
        line.trim()
    };
    if path.is_empty() || path.contains('\0') {
        return None;
    }
    Some(PathBuf::from(path))
}

pub fn is_application_desktop(path: &Path) -> bool {
    let text = path.to_string_lossy();
    (text.contains("/usr/share/applications/") || text.contains("/usr/local/share/applications/"))
        && text.ends_with(".desktop")
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesktopEntry {
    pub name: String,
    pub exec: Option<String>,
    pub no_display: bool,
}

pub fn parse_desktop_entry(contents: &str) -> DesktopEntry {
    let mut in_entry = false;
    let mut entry = DesktopEntry::default();
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            in_entry = line.eq_ignore_ascii_case("[Desktop Entry]");
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            match key {
                "Name" if entry.name.is_empty() => entry.name = value.trim().to_owned(),
                "Exec" if entry.exec.is_none() => {
                    entry.exec = Some(first_exec_token(value));
                }
                "NoDisplay" => {
                    entry.no_display = value.trim().eq_ignore_ascii_case("true");
                }
                _ => {}
            }
        }
    }
    entry
}

pub fn first_exec_token(exec: &str) -> String {
    let exec = exec.trim();
    let token = if let Some(rest) = exec.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        exec.split_whitespace().next().unwrap_or(exec)
    };
    Path::new(token)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(token)
        .to_owned()
}

fn package_owns_executable(files: &[PathBuf], exe: &Path, exec: Option<&str>) -> bool {
    if files.iter().any(|path| path == exe) {
        return true;
    }
    let Some(exe_name) = exe.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    exec == Some(exe_name)
        || files
            .iter()
            .any(|path| path.file_name().and_then(|name| name.to_str()) == Some(exe_name))
}

fn command_stdout(program: &str, args: &[&str]) -> Result<String, String> {
    if program.is_empty() || program.contains('/') || program.contains('\0') {
        return Err(format!("refusing to run {program}"));
    }
    for arg in args {
        if arg.contains('\0') {
            return Err("command argument contains NUL".to_owned());
        }
    }

    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|error| format!("{program} failed to start: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr.trim();
        if detail.is_empty() {
            return Err(format!(
                "{program} exited with {}",
                output.status.code().unwrap_or(-1)
            ));
        }
        return Err(format!("{program}: {detail}"));
    }
    String::from_utf8(output.stdout).map_err(|_| format!("{program} output is not valid UTF-8"))
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

    #[test]
    fn compact_uptime_matches_glance_mock_shape() {
        assert_eq!(format_uptime(Some((2, 4, 12)), true), "2d 4h");
        assert_eq!(format_uptime(Some((0, 3, 12)), true), "3h");
        assert_eq!(format_uptime(Some((0, 0, 9)), true), "9m");
        assert_eq!(format_uptime(Some((2, 4, 12)), false), "2d 4h 12m");
        assert_eq!(format_uptime(None, true), "Unknown");
    }

    #[test]
    fn parse_desktop_entry_reads_name_exec_and_nodisplay() {
        let visible = parse_desktop_entry(
            "[Desktop Entry]\nName=Firefox Web Browser\nExec=/usr/lib/firefox/firefox %u\n",
        );
        assert_eq!(visible.name, "Firefox Web Browser");
        assert_eq!(visible.exec.as_deref(), Some("firefox"));
        assert!(!visible.no_display);

        let hidden = parse_desktop_entry(
            "[Desktop Entry]\nName=Helper\nExec=helper\nNoDisplay=true\n[Desktop Action X]\nName=Nope\n",
        );
        assert!(hidden.no_display);
        assert_eq!(hidden.name, "Helper");
    }

    #[test]
    fn application_desktop_paths_are_restricted() {
        assert!(is_application_desktop(Path::new(
            "/usr/share/applications/firefox.desktop"
        )));
        assert!(is_application_desktop(Path::new(
            "/usr/local/share/applications/custom.desktop"
        )));
        assert!(!is_application_desktop(Path::new(
            "/usr/share/doc/firefox.desktop"
        )));
        assert!(!is_application_desktop(Path::new(
            "/usr/share/applications/firefox.png"
        )));
    }

    #[test]
    fn first_exec_token_uses_basename() {
        assert_eq!(
            first_exec_token("/usr/bin/telegram-desktop -- %u"),
            "telegram-desktop"
        );
        assert_eq!(first_exec_token("\"/opt/Foo Bar/app\" %f"), "app");
    }

    #[test]
    fn command_stdout_rejects_path_payloads() {
        assert!(command_stdout("/bin/echo", &["hi"]).is_err());
        assert!(command_stdout("sh;id", &[]).is_err());
    }

    #[test]
    fn split_flatpak_list_row_handles_tab_and_spaces() {
        assert_eq!(
            split_flatpak_list_row("org.gimp.GIMP\tGNU Image Manipulation Program"),
            ("org.gimp.GIMP", "GNU Image Manipulation Program")
        );
        assert_eq!(
            split_flatpak_list_row("com.brave.Browser  Brave"),
            ("com.brave.Browser", "Brave")
        );
    }

    #[test]
    fn scan_does_not_invoke_a_shell_string() {
        let src = include_str!("system.rs");
        let prod = src.split("#[cfg(test)]").next().unwrap();
        assert!(!prod.contains("bash -c"));
        assert!(!prod.contains("sh -c"));
        assert!(prod.contains("scan_removable_apps"));
        assert!(prod.contains("apt-mark"));
        assert!(prod.contains("pacman"));
        assert!(prod.contains("dnf"));
    }
}
