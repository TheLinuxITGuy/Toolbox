//! App catalog loading and command construction.
//!
//! CSV rows and admin helper names are validated before they are turned into
//! argv arrays. Commands are never built as a single shell string.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::validate::{
    validate_action, validate_category, validate_exec_name, validate_flatpak_id, validate_label,
    validate_notes, validate_package_name, validate_script_name,
};

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

    let canonical = path
        .canonicalize()
        .map_err(|error| format!("cannot resolve helper script {name}: {error}"))?;
    let base = base_dir
        .canonicalize()
        .map_err(|error| format!("cannot resolve app directory: {error}"))?;

    if !canonical.starts_with(&base) {
        return Err(format!("helper script {name} is outside the app directory"));
    }

    canonical
        .to_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("helper script {name} path is not valid UTF-8"))
}

pub fn load_apps(base_dir: &Path) -> CatalogLoad {
    let path = base_dir.join("apps_config.csv");
    if !path.is_file() {
        return CatalogLoad {
            error: Some(format!(
                "App catalog not found at {}. Add apps_config.csv next to the helper scripts.",
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

pub fn find_base_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
        && catalog_present(dir)
    {
        return dir.to_path_buf();
    }

    if let Ok(cwd) = std::env::current_dir()
        && catalog_present(&cwd)
    {
        return cwd;
    }

    PathBuf::from(".")
}

fn catalog_present(dir: &Path) -> bool {
    dir.join("apps_config.csv").is_file() && dir.join("main.sh").is_file()
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
}
