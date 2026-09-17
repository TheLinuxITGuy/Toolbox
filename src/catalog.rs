//! App catalog loading and command construction.
//!
//! CSV rows and admin helper names are validated before they are turned into
//! argv arrays. Commands are never built as a single shell string.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::validate::{
    validate_action, validate_category, validate_exec_name, validate_flatpak_id, validate_label,
    validate_notes, validate_package_name, validate_script_name,
};

/// Absolute placeholder used when the catalog was not found.
/// Relative fallbacks such as `.` would resolve helper names against CWD.
pub const MISSING_BASE_DIR: &str = "/var/empty-linux-it-guy-toolbox";

/// How the catalog directory was located.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaseDirSource {
    Executable,
    Ancestor,
    WorkingDirectory,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CsvAppEntry {
    #[serde(rename = "Category")]
    pub category: String,
    #[serde(rename = "Label")]
    pub label: String,
    #[serde(rename = "Package Name")]
    pub package_name: String,
    #[serde(rename = "Flatpak ID")]
    pub flatpak_id: String,
    #[serde(rename = "Exec Name")]
    pub exec_name: String,
    #[serde(rename = "Notes")]
    pub notes: String,
}

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub category: String,
    pub label: String,
    pub package_name: String,
    pub flatpak_id: String,
    pub exec_name: String,
    pub notes: String,
}

impl AppEntry {
    pub fn from_csv(entry: CsvAppEntry) -> Result<Self, String> {
        let category = entry.category.trim();
        let label = entry.label.trim();
        let package_name = entry.package_name.trim();
        let flatpak_id = entry.flatpak_id.trim();
        let exec_name = entry.exec_name.trim();
        let notes = entry.notes.trim();

        validate_category(category).map_err(|error| error.to_string())?;
        validate_label(label).map_err(|error| error.to_string())?;
        validate_notes(notes).map_err(|error| error.to_string())?;

        if package_name.is_empty() && flatpak_id.is_empty() {
            return Err("at least one of package name or Flatpak ID is required".to_owned());
        }
        if !package_name.is_empty() {
            validate_package_name(package_name).map_err(|error| error.to_string())?;
        }
        if !flatpak_id.is_empty() {
            validate_flatpak_id(flatpak_id).map_err(|error| error.to_string())?;
        }
        if !exec_name.is_empty() {
            validate_exec_name(exec_name).map_err(|error| error.to_string())?;
        }

        Ok(Self {
            category: category.to_owned(),
            label: label.to_owned(),
            package_name: package_name.to_owned(),
            flatpak_id: flatpak_id.to_owned(),
            exec_name: exec_name.to_owned(),
            notes: notes.to_owned(),
        })
    }

    pub fn source_label(&self) -> &'static str {
        if self.flatpak_id.is_empty() {
            "native"
        } else if self.package_name.is_empty() {
            "flatpak"
        } else {
            "native + flatpak"
        }
    }

    pub fn try_command(&self, base_dir: &Path, action: &str) -> Result<Vec<String>, String> {
        validate_action(action).map_err(|error| error.to_string())?;
        let script = resolve_helper_script(base_dir, "main.sh")?;

        let mut command = vec!["bash".to_owned(), script];
        command.extend(["--label".to_owned(), self.label.clone()]);
        if !self.package_name.is_empty() {
            command.extend(["--package".to_owned(), self.package_name.clone()]);
        }
        if !self.flatpak_id.is_empty() {
            command.extend(["--flatpak".to_owned(), self.flatpak_id.clone()]);
        }
        if !self.exec_name.is_empty() {
            command.extend(["--exec".to_owned(), self.exec_name.clone()]);
        }
        command.push(action.to_owned());
        Ok(command)
    }
}

#[derive(Clone, Debug)]
pub struct AdminTask {
    #[allow(dead_code)]
    pub category: String,
    pub label: String,
    pub script: String,
}

impl AdminTask {
    pub fn try_command(&self, base_dir: &Path) -> Result<Vec<String>, String> {
        let script = resolve_helper_script(base_dir, &self.script)?;
        Ok(vec!["bash".to_owned(), script])
    }
}

#[derive(Clone, Debug)]
pub struct Task {
    pub description: String,
    pub command: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CatalogLoad {
    pub apps: Vec<AppEntry>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

pub fn resolve_helper_script(base_dir: &Path, name: &str) -> Result<String, String> {
    validate_script_name(name).map_err(|error| error.to_string())?;

    let path = base_dir.join(name);
    if !path.is_file() {
        return Err(format!("helper script {name} was not found"));
    }

    helper_path_is_under_base(&path, base_dir)?
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("helper script {name} path is not valid UTF-8"))
}

/// Re-check at spawn time that `script` canonicalizes inside `base_dir`.
///
/// Basename allowlisting alone is not enough: `/tmp/main.sh` must not run.
pub fn helper_path_is_under_base(script: &Path, base_dir: &Path) -> Result<PathBuf, String> {
    let base = base_dir
        .canonicalize()
        .map_err(|error| format!("cannot resolve app directory: {error}"))?;
    if base.parent().is_none() {
        return Err("app directory must not be the filesystem root".to_owned());
    }

    let canonical = script
        .canonicalize()
        .map_err(|error| format!("cannot resolve helper script {}: {error}", script.display()))?;

    if !canonical.starts_with(&base) {
        return Err(format!(
            "helper script {} is outside the app directory",
            script.display()
        ));
    }

    Ok(canonical)
}

pub fn load_apps(base_dir: &Path) -> CatalogLoad {
    let path = base_dir.join("apps_config.csv");
    if !path.is_file() {
        return CatalogLoad {
            error: Some(format!(
                "App catalog not found at {}. Expected apps_config.csv beside the helper scripts (main.sh).",
                path.display()
            )),
            ..CatalogLoad::default()
        };
    }

    let mut reader = match csv::Reader::from_path(&path) {
        Ok(reader) => reader,
        Err(error) => {
            return CatalogLoad {
                error: Some(format!("Failed to open app catalog: {error}")),
                ..CatalogLoad::default()
            };
        }
    };

    let mut load = CatalogLoad::default();
    for (index, result) in reader.deserialize::<CsvAppEntry>().enumerate() {
        let row = index + 2;
        match result {
            Ok(entry) => match AppEntry::from_csv(entry) {
                Ok(app) => load.apps.push(app),
                Err(error) => load
                    .warnings
                    .push(format!("Skipping catalog row {row}: {error}")),
            },
            Err(error) => load
                .warnings
                .push(format!("Skipping catalog row {row}: {error}")),
        }
    }

    if load.apps.is_empty() && load.error.is_none() {
        load.error = Some("App catalog did not contain any valid rows.".to_owned());
    }

    load
}

pub fn admin_tasks() -> Vec<AdminTask> {
    [
        (
            "Power Management",
            "Enable Bluetooth",
            "enable-bluetooth.sh",
        ),
        (
            "Power Management",
            "Disable Bluetooth",
            "disable-bluetooth.sh",
        ),
        ("Power Management", "TLP (Laptops)", "install-tlp.sh"),
        ("Power Management", "Powertop", "install-powertop.sh"),
        ("System", "Update System", "update-system.sh"),
        (
            "System",
            "nala (rank mirrors) - Debian only",
            "install-nala.sh",
        ),
        ("System", "Stacer", "install-stacer.sh"),
        ("System", "SWAP Fix", "install-swapfix.sh"),
        ("System", "Fastfetch", "install-fastfetch.sh"),
    ]
    .into_iter()
    .map(|(category, label, script)| AdminTask {
        category: category.to_owned(),
        label: label.to_owned(),
        script: script.to_owned(),
    })
    .collect()
}

pub fn display_command(command: &[String]) -> String {
    command
        .iter()
        .map(|part| {
            if part.chars().any(|c| c.is_ascii_whitespace()) {
                format!("\"{part}\"")
            } else {
                part.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

const CATALOG_FILE: &str = "apps_config.csv";
const HELPER_MARKER: &str = "main.sh";

/// Result of locating the directory that holds the catalog and helper scripts.
#[derive(Clone, Debug)]
pub struct BaseDirDiscovery {
    pub base_dir: PathBuf,
    pub looked: Vec<PathBuf>,
    pub found: bool,
    pub source: Option<BaseDirSource>,
}

impl Default for BaseDirDiscovery {
    fn default() -> Self {
        Self::missing()
    }
}

impl BaseDirDiscovery {
    fn missing() -> Self {
        Self {
            base_dir: PathBuf::from(MISSING_BASE_DIR),
            looked: Vec::new(),
            found: false,
            source: None,
        }
    }

    pub fn not_found_message(&self) -> String {
        if self.looked.is_empty() {
            return "App catalog not found. Looked for apps_config.csv beside the helper scripts next to the executable, in parent folders, and in the current working directory.".to_owned();
        }

        let places = self
            .looked
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "App catalog not found. Looked for apps_config.csv beside the helper scripts in: {places}"
        )
    }
}

pub fn find_base_dir() -> BaseDirDiscovery {
    discover_base_dir(
        std::env::current_exe().ok().as_deref(),
        std::env::current_dir().ok().as_deref(),
    )
}

/// Lookup order: directory of the executable, walk-up toward a repo root that
/// contains the catalog and helper scripts, then the current working directory
/// as a last resort (untrusted; the UI warns when this source is used).
pub fn discover_base_dir(exe: Option<&Path>, cwd: Option<&Path>) -> BaseDirDiscovery {
    let mut discovery = BaseDirDiscovery::missing();
    let mut seen = HashSet::new();

    let mut consider = |dir: &Path, source: BaseDirSource| -> bool {
        let key = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        if !seen.insert(key.clone()) {
            return false;
        }
        discovery.looked.push(dir.to_path_buf());
        if catalog_present(dir) {
            discovery.base_dir = key;
            discovery.found = true;
            discovery.source = Some(source);
            return true;
        }
        false
    };

    if let Some(exe) = exe {
        // 1. Next to the executable (e.g. a release layout that ships the CSV).
        if let Some(exe_dir) = exe.parent() {
            if consider(exe_dir, BaseDirSource::Executable) {
                return discovery;
            }

            // 2. Walk up from the binary toward a checkout root.
            for ancestor in exe_dir.ancestors().skip(1) {
                if consider(ancestor, BaseDirSource::Ancestor) {
                    return discovery;
                }
                if ancestor.parent().is_none() {
                    break;
                }
            }
        }
    }

    // 3. Directory that already contains helper scripts (main.sh), if the
    // catalog sits beside them but was not on the exe walk.
    for candidate in helper_script_dirs(exe, None) {
        if consider(&candidate, BaseDirSource::Ancestor) {
            return discovery;
        }
    }

    // 4. Current working directory. Last resort: scripts here are not tied
    // to the executable, so the GUI warns when this source is used.
    if let Some(cwd) = cwd {
        for candidate in helper_script_dirs(None, Some(cwd)) {
            if consider(&candidate, BaseDirSource::WorkingDirectory) {
                return discovery;
            }
        }
        if consider(cwd, BaseDirSource::WorkingDirectory) {
            return discovery;
        }
    }

    discovery
}

fn helper_script_dirs(exe: Option<&Path>, cwd: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut push_if_helpers = |dir: &Path| {
        if dir.join(HELPER_MARKER).is_file() {
            dirs.push(dir.to_path_buf());
        }
    };

    if let Some(exe) = exe
        && let Some(exe_dir) = exe.parent()
    {
        push_if_helpers(exe_dir);
        for ancestor in exe_dir.ancestors().skip(1) {
            push_if_helpers(ancestor);
            if ancestor.parent().is_none() {
                break;
            }
        }
    }

    if let Some(cwd) = cwd {
        push_if_helpers(cwd);
    }

    dirs
}

fn catalog_present(dir: &Path) -> bool {
    dir.join(CATALOG_FILE).is_file() && dir.join(HELPER_MARKER).is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempToolbox {
        path: PathBuf,
    }

    impl Drop for TempToolbox {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn sample_entry() -> AppEntry {
        AppEntry {
            category: "Browsers".to_owned(),
            label: "Firefox".to_owned(),
            package_name: "firefox".to_owned(),
            flatpak_id: String::new(),
            exec_name: "firefox".to_owned(),
            notes: String::new(),
        }
    }

    fn temp_toolbox() -> TempToolbox {
        let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "linux-it-guy-toolbox-test-{}-{}",
            std::process::id(),
            seq
        ));
        fs::create_dir_all(&path).expect("create temp toolbox dir");
        fs::write(
            path.join("apps_config.csv"),
            "Category,Label,Package Name,Flatpak ID,Exec Name,Notes\nBrowsers,Firefox,firefox,,firefox,\n",
        )
        .unwrap();
        fs::write(path.join("main.sh"), "#!/bin/bash\n").unwrap();
        TempToolbox { path }
    }

    #[test]
    fn command_uses_argv_array_not_shell_string() {
        let toolbox = temp_toolbox();
        let command = sample_entry()
            .try_command(&toolbox.path, "install")
            .unwrap();
        assert_eq!(command[0], "bash");
        assert!(command[1].ends_with("main.sh"));
        assert_eq!(
            command[2..],
            [
                "--label",
                "Firefox",
                "--package",
                "firefox",
                "--exec",
                "firefox",
                "install"
            ]
        );
        assert!(!command.iter().any(|part| part.contains(';')));
    }

    #[test]
    fn command_omits_empty_optional_targets() {
        let toolbox = temp_toolbox();
        let mut entry = sample_entry();
        entry.package_name.clear();
        entry.flatpak_id = "org.mozilla.firefox".to_owned();
        let command = entry.try_command(&toolbox.path, "remove").unwrap();
        assert!(!command.contains(&"--package".to_owned()));
        assert!(command.contains(&"--flatpak".to_owned()));
        assert_eq!(command.last().map(String::as_str), Some("remove"));
    }

    #[test]
    fn command_rejects_unknown_action() {
        let toolbox = temp_toolbox();
        assert!(
            sample_entry()
                .try_command(&toolbox.path, "upgrade")
                .is_err()
        );
    }

    #[test]
    fn csv_row_rejects_invalid_package() {
        let result = AppEntry::from_csv(CsvAppEntry {
            category: "Browsers".into(),
            label: "Evil".into(),
            package_name: "-Syu".into(),
            flatpak_id: String::new(),
            exec_name: String::new(),
            notes: String::new(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn load_apps_skips_invalid_rows_and_keeps_valid_ones() {
        let toolbox = temp_toolbox();
        let path = &toolbox.path;
        let mut file = fs::File::create(path.join("apps_config.csv")).unwrap();
        writeln!(
            file,
            "Category,Label,Package Name,Flatpak ID,Exec Name,Notes"
        )
        .unwrap();
        writeln!(file, "Browsers,Firefox,firefox,,firefox,").unwrap();
        writeln!(file, "Browsers,Evil,-Syu,,evil,").unwrap();
        writeln!(
            file,
            "Browsers,Brave,,com.brave.Browser,brave,Privacy-focused browser"
        )
        .unwrap();
        drop(file);
        fs::write(path.join("main.sh"), "#!/bin/bash\n").unwrap();

        let load = load_apps(path);
        assert_eq!(load.apps.len(), 2);
        assert_eq!(load.apps[0].label, "Firefox");
        assert_eq!(load.apps[1].label, "Brave");
        assert_eq!(load.warnings.len(), 1);
        assert!(load.error.is_none());
    }

    #[test]
    fn resolve_helper_script_rejects_unknown_and_escaped_names() {
        let toolbox = temp_toolbox();
        assert!(resolve_helper_script(&toolbox.path, "main.sh").is_ok());
        assert!(resolve_helper_script(&toolbox.path, "../main.sh").is_err());
        assert!(resolve_helper_script(&toolbox.path, "not-real.sh").is_err());
        assert!(resolve_helper_script(&toolbox.path, "enable-bluetooth.sh").is_err());
    }

    #[test]
    fn admin_command_is_bash_plus_allowlisted_script() {
        let toolbox = temp_toolbox();
        let path = toolbox.path.clone();
        fs::write(path.join("update-system.sh"), "#!/bin/bash\n").unwrap();
        let task = AdminTask {
            category: "System".into(),
            label: "Update System".into(),
            script: "update-system.sh".into(),
        };
        let command = task.try_command(&path).unwrap();
        assert_eq!(command[0], "bash");
        assert!(command[1].ends_with("update-system.sh"));
        assert_eq!(command.len(), 2);
    }

    #[test]
    fn display_command_quotes_whitespace() {
        assert_eq!(
            display_command(&["bash".into(), "/tmp/My Scripts/main.sh".into()]),
            "bash \"/tmp/My Scripts/main.sh\""
        );
    }

    #[test]
    fn shipped_catalog_loads_without_warnings() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let load = load_apps(&root);
        assert!(load.error.is_none(), "{:?}", load.error);
        assert!(
            load.warnings.is_empty(),
            "unexpected catalog warnings: {:?}",
            load.warnings
        );
        assert!(!load.apps.is_empty());
    }

    #[test]
    fn shipped_catalog_has_brave_origin_native_and_keeps_flatpak_brave() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let load = load_apps(&root);
        assert!(load.error.is_none(), "{:?}", load.error);
        assert!(load.warnings.is_empty(), "{:?}", load.warnings);

        let origin = load
            .apps
            .iter()
            .find(|app| app.label == "Brave Origin")
            .expect("Brave Origin catalog row");
        assert_eq!(origin.category, "Browsers");
        assert_eq!(origin.package_name, "brave-origin");
        assert!(origin.flatpak_id.is_empty());
        assert_eq!(origin.exec_name, "brave-origin");
        assert_eq!(origin.source_label(), "native");

        let command = origin.try_command(&root, "install").unwrap();
        assert_eq!(command[0], "bash");
        assert!(command[1].ends_with("main.sh"));
        assert!(command.contains(&"--package".to_owned()));
        assert!(command.contains(&"brave-origin".to_owned()));
        assert!(command.contains(&"--exec".to_owned()));
        assert!(!command.contains(&"--flatpak".to_owned()));
        assert_eq!(command.last().map(String::as_str), Some("install"));

        let remove = origin.try_command(&root, "remove").unwrap();
        assert_eq!(remove.last().map(String::as_str), Some("remove"));
        assert!(remove.contains(&"--package".to_owned()));
        assert!(!remove.contains(&"--flatpak".to_owned()));

        let flatpak_brave = load
            .apps
            .iter()
            .find(|app| app.label == "Brave Browser")
            .expect("Brave Browser Flatpak catalog row");
        assert!(flatpak_brave.package_name.is_empty());
        assert_eq!(flatpak_brave.flatpak_id, "com.brave.Browser");
        assert_eq!(flatpak_brave.exec_name, "brave");
    }

    #[test]
    fn shipped_catalog_keeps_original_rows_and_adds_flathub_popular() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let load = load_apps(&root);
        assert!(load.error.is_none(), "{:?}", load.error);
        assert!(load.warnings.is_empty(), "{:?}", load.warnings);
        assert_eq!(load.apps.len(), 50);

        let original = [
            "Brave Browser",
            "Brave Origin",
            "Google Chrome",
            "Firefox",
            "Microsoft Edge",
            "Opera",
            "Vivaldi",
            "Zen",
            "Discord",
            "Signal",
            "Slack",
            "Thunderbird",
            "Bottles",
            "Boxes",
            "Visual Studio Code",
            "PyCharm Community",
            "Lutris",
            "ProtonUp-Qt",
            "Steam",
            "Steam",
            "Audacity",
            "GIMP",
            "mpv",
            "OBS Studio",
            "OBS Studio",
            "VLC",
            "LibreOffice",
            "OnlyOffice",
            "GParted",
            "htop",
            "LocalSend",
        ];
        for (index, label) in original.iter().enumerate() {
            assert_eq!(load.apps[index].label, *label, "original row {index}");
        }

        let additions = [
            (
                "Sober",
                "Gaming",
                "",
                "org.vinegarhq.Sober",
                "sober",
                "flatpak",
            ),
            (
                "Spotify",
                "Multimedia",
                "",
                "com.spotify.Client",
                "spotify",
                "flatpak",
            ),
            (
                "Heroic",
                "Gaming",
                "",
                "com.heroicgameslauncher.hgl",
                "heroic",
                "flatpak",
            ),
            (
                "Flatseal",
                "Utilities",
                "",
                "com.github.tchx84.Flatseal",
                "flatseal",
                "flatpak",
            ),
            (
                "Telegram",
                "Communication",
                "telegram-desktop",
                "",
                "telegram-desktop",
                "native",
            ),
            (
                "Prism Launcher",
                "Gaming",
                "",
                "org.prismlauncher.PrismLauncher",
                "prismlauncher",
                "flatpak",
            ),
            (
                "Obsidian",
                "Office Tools",
                "",
                "md.obsidian.Obsidian",
                "obsidian",
                "flatpak",
            ),
            (
                "RetroArch",
                "Gaming",
                "retroarch",
                "",
                "retroarch",
                "native",
            ),
            (
                "Extension Manager",
                "Utilities",
                "",
                "com.mattjakeman.ExtensionManager",
                "extension-manager",
                "flatpak",
            ),
            (
                "Dolphin Emulator",
                "Gaming",
                "dolphin-emu",
                "",
                "dolphin-emu",
                "native",
            ),
            (
                "qBittorrent",
                "Utilities",
                "qbittorrent",
                "",
                "qbittorrent",
                "native",
            ),
            ("PPSSPP", "Gaming", "ppsspp", "", "ppsspp", "native"),
            (
                "Gear Lever",
                "Utilities",
                "",
                "it.mijorus.gearlever",
                "gearlever",
                "flatpak",
            ),
            (
                "Proton VPN",
                "Utilities",
                "",
                "com.protonvpn.www",
                "protonvpn-app",
                "flatpak",
            ),
            (
                "ProtonPlus",
                "Gaming",
                "",
                "com.vysp3r.ProtonPlus",
                "protonplus",
                "flatpak",
            ),
            (
                "Bitwarden",
                "Utilities",
                "",
                "com.bitwarden.desktop",
                "bitwarden",
                "flatpak",
            ),
            (
                "Stremio",
                "Multimedia",
                "",
                "com.stremio.Stremio",
                "stremio",
                "flatpak",
            ),
            (
                "LibreWolf",
                "Browsers",
                "",
                "io.gitlab.librewolf-community",
                "librewolf",
                "flatpak",
            ),
            (
                "Mission Center",
                "Utilities",
                "",
                "io.missioncenter.MissionCenter",
                "missioncenter",
                "flatpak",
            ),
        ];
        for (offset, (label, category, package, flatpak, exec, source)) in
            additions.iter().enumerate()
        {
            let app = &load.apps[31 + offset];
            assert_eq!(app.label, *label);
            assert_eq!(app.category, *category);
            assert_eq!(app.package_name, *package);
            assert_eq!(app.flatpak_id, *flatpak);
            assert_eq!(app.exec_name, *exec);
            assert_eq!(app.source_label(), *source);
            AppEntry::from_csv(CsvAppEntry {
                category: (*category).to_owned(),
                label: (*label).to_owned(),
                package_name: (*package).to_owned(),
                flatpak_id: (*flatpak).to_owned(),
                exec_name: (*exec).to_owned(),
                notes: String::new(),
            })
            .unwrap_or_else(|error| panic!("{label}: {error}"));
        }

        assert_eq!(
            load.apps
                .iter()
                .filter(|app| app.label == "Firefox")
                .count(),
            1
        );
        assert_eq!(
            load.apps.iter().filter(|app| app.label == "Steam").count(),
            2
        );
    }

    fn empty_dir(label: &str) -> PathBuf {
        let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "linux-it-guy-toolbox-lookup-{}-{}-{}",
            std::process::id(),
            seq,
            label
        ));
        fs::create_dir_all(&path).expect("create empty lookup dir");
        path
    }

    fn write_catalog_files(dir: &Path) {
        fs::write(
            dir.join("apps_config.csv"),
            "Category,Label,Package Name,Flatpak ID,Exec Name,Notes\nBrowsers,Firefox,firefox,,firefox,\n",
        )
        .unwrap();
        fs::write(dir.join("main.sh"), "#!/bin/bash\n").unwrap();
    }

    #[test]
    fn discover_prefers_directory_beside_the_executable() {
        let toolbox = temp_toolbox();
        let exe = toolbox.path.join("linux-it-guy-toolbox");
        fs::write(&exe, []).unwrap();
        let cwd = empty_dir("cwd-ignored");
        write_catalog_files(&cwd);

        let discovery = discover_base_dir(Some(&exe), Some(&cwd));
        assert!(discovery.found);
        assert_eq!(discovery.source, Some(BaseDirSource::Executable));
        assert_eq!(
            discovery.base_dir.canonicalize().unwrap(),
            toolbox.path.canonicalize().unwrap()
        );
        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn discover_walks_up_from_target_release_to_repo_root() {
        let toolbox = temp_toolbox();
        let release_dir = toolbox.path.join("target").join("release");
        fs::create_dir_all(&release_dir).unwrap();
        let exe = release_dir.join("linux-it-guy-toolbox");
        fs::write(&exe, []).unwrap();
        let cwd = empty_dir("other-cwd");

        let discovery = discover_base_dir(Some(&exe), Some(&cwd));
        assert!(discovery.found);
        assert_eq!(discovery.source, Some(BaseDirSource::Ancestor));
        assert_eq!(
            discovery.base_dir.canonicalize().unwrap(),
            toolbox.path.canonicalize().unwrap()
        );
        assert!(discovery.looked.iter().any(|path| path == &release_dir));
        assert!(discovery.looked.iter().any(|path| path == &toolbox.path));
        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn discover_uses_directory_beside_helper_scripts() {
        let root = empty_dir("helpers-root");
        write_catalog_files(&root);
        let nested = root.join("nested").join("bin");
        fs::create_dir_all(&nested).unwrap();
        let exe = nested.join("toolbox");
        fs::write(&exe, []).unwrap();
        let cwd = empty_dir("helpers-cwd");

        let discovery = discover_base_dir(Some(&exe), Some(&cwd));
        assert!(discovery.found);
        assert_eq!(
            discovery.base_dir.canonicalize().unwrap(),
            root.canonicalize().unwrap()
        );
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn discover_falls_back_to_current_working_directory() {
        let cwd = empty_dir("cwd-catalog");
        write_catalog_files(&cwd);
        let exe_dir = empty_dir("exe-without-catalog");
        let exe = exe_dir.join("linux-it-guy-toolbox");
        fs::write(&exe, []).unwrap();

        let discovery = discover_base_dir(Some(&exe), Some(&cwd));
        assert!(discovery.found);
        assert_eq!(discovery.source, Some(BaseDirSource::WorkingDirectory));
        assert_eq!(
            discovery.base_dir.canonicalize().unwrap(),
            cwd.canonicalize().unwrap()
        );
        let _ = fs::remove_dir_all(cwd);
        let _ = fs::remove_dir_all(exe_dir);
    }

    #[test]
    fn discover_when_missing_does_not_fall_back_to_dot() {
        let exe_dir = empty_dir("missing-exe-nodot");
        let exe = exe_dir.join("linux-it-guy-toolbox");
        fs::write(&exe, []).unwrap();
        let cwd = empty_dir("missing-cwd-nodot");

        let discovery = discover_base_dir(Some(&exe), Some(&cwd));
        assert!(!discovery.found);
        assert!(discovery.source.is_none());
        assert_eq!(discovery.base_dir, PathBuf::from(MISSING_BASE_DIR));
        let _ = fs::remove_dir_all(exe_dir);
        let _ = fs::remove_dir_all(cwd);
    }

    #[test]
    fn helper_path_is_under_base_rejects_same_basename_outside_root() {
        let toolbox = temp_toolbox();
        let outsider = empty_dir("evil-main");
        fs::write(outsider.join("main.sh"), "#!/bin/bash\necho pwned\n").unwrap();

        assert!(helper_path_is_under_base(&toolbox.path.join("main.sh"), &toolbox.path).is_ok());
        assert!(
            helper_path_is_under_base(&outsider.join("main.sh"), &toolbox.path).is_err(),
            "basename main.sh outside the toolbox root must be rejected"
        );
        let _ = fs::remove_dir_all(outsider);
    }

    #[test]
    fn discover_reports_every_place_it_looked_when_missing() {
        let exe_dir = empty_dir("missing-exe");
        let nested = exe_dir.join("target").join("release");
        fs::create_dir_all(&nested).unwrap();
        let exe = nested.join("linux-it-guy-toolbox");
        fs::write(&exe, []).unwrap();
        let cwd = empty_dir("missing-cwd");

        let discovery = discover_base_dir(Some(&exe), Some(&cwd));
        assert!(!discovery.found);
        assert!(discovery.looked.iter().any(|path| path == &nested));
        assert!(discovery.looked.iter().any(|path| path == &cwd));
        let message = discovery.not_found_message();
        assert!(message.contains("Looked for apps_config.csv beside the helper scripts"));
        assert!(!message.contains("./apps_config.csv"));
        let _ = fs::remove_dir_all(exe_dir);
        let _ = fs::remove_dir_all(cwd);
    }
}
