//! GNOME Bibata cursor themes.
//!
//! Previews are original pointer drawings compiled into the binary. Theme
//! archives are downloaded only when Apply or the run queue installs one.

use std::path::Path;

use crate::catalog::{Task, resolve_helper_script};

pub const APPLY_CURSOR_SCRIPT: &str = "apply-cursor.sh";

/// The three left-pointer Bibata Modern themes this tab can apply.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CursorTheme {
    Classic,
    Ice,
    Amber,
}

impl CursorTheme {
    pub const ALL: [Self; 3] = [Self::Classic, Self::Ice, Self::Amber];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Classic => "Bibata Modern Classic",
            Self::Ice => "Bibata Modern Ice",
            Self::Amber => "Bibata Modern Amber",
        }
    }

    pub const fn directory(self) -> &'static str {
        match self {
            Self::Classic => "Bibata-Modern-Classic",
            Self::Ice => "Bibata-Modern-Ice",
            Self::Amber => "Bibata-Modern-Amber",
        }
    }

    /// Pinned v2.0.7 Linux archive. Not `-Right`, not a Windows zip, not latest.
    #[cfg_attr(not(test), allow(dead_code))]
    pub const fn download_url(self) -> &'static str {
        match self {
            Self::Classic => {
                "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/Bibata-Modern-Classic.tar.xz"
            }
            Self::Ice => {
                "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/Bibata-Modern-Ice.tar.xz"
            }
            Self::Amber => {
                "https://github.com/ful1e5/Bibata_Cursor/releases/download/v2.0.7/Bibata-Modern-Amber.tar.xz"
            }
        }
    }

    pub const fn preview_key(self) -> &'static str {
        match self {
            Self::Classic => "cursor-bibata-modern-classic",
            Self::Ice => "cursor-bibata-modern-ice",
            Self::Amber => "cursor-bibata-modern-amber",
        }
    }

    pub const fn preview_png(self) -> &'static [u8] {
        match self {
            Self::Classic => include_bytes!("../assets/cursors/bibata-modern-classic.png"),
            Self::Ice => include_bytes!("../assets/cursors/bibata-modern-ice.png"),
            Self::Amber => include_bytes!("../assets/cursors/bibata-modern-amber.png"),
        }
    }

    pub fn from_directory(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|theme| theme.directory() == name)
    }

    pub fn try_task(self, base_dir: &Path) -> Result<Task, String> {
        let script = resolve_helper_script(base_dir, APPLY_CURSOR_SCRIPT)?;
        Ok(Task {
            description: format!("Applying {}", self.label()),
            command: vec!["bash".to_owned(), script, self.directory().to_owned()],
        })
    }
}

pub fn validate_cursor_argv(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return Err("Cursor helper accepts exactly one theme name.".to_owned());
    }
    if CursorTheme::from_directory(&args[0]).is_none() {
        return Err("Cursor theme is not one of the Bibata Modern themes.".to_owned());
    }
    Ok(())
}

/// Wrap a compiled-in PNG the same way other UI images are rasterized.
pub fn preview_svg(png: &[u8]) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" viewBox=\"0 0 64 64\"><image width=\"64\" height=\"64\" href=\"data:image/png;base64,{}\"/></svg>",
        base64_encode(png)
    )
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut index = 0;
    while index + 3 <= data.len() {
        let chunk = ((data[index] as u32) << 16)
            | ((data[index + 1] as u32) << 8)
            | (data[index + 2] as u32);
        out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
        out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
        out.push(TABLE[((chunk >> 6) & 63) as usize] as char);
        out.push(TABLE[(chunk & 63) as usize] as char);
        index += 3;
    }
    match data.len() - index {
        1 => {
            let chunk = (data[index] as u32) << 16;
            out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
            out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let chunk = ((data[index] as u32) << 16) | ((data[index + 1] as u32) << 8);
            out.push(TABLE[((chunk >> 18) & 63) as usize] as char);
            out.push(TABLE[((chunk >> 12) & 63) as usize] as char);
            out.push(TABLE[((chunk >> 6) & 63) as usize] as char);
            out.push('=');
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempHome {
        path: PathBuf,
    }

    impl TempHome {
        fn new() -> Self {
            let seq = SEQ.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("toolbox-cursor-{}-{}", std::process::id(), seq));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn script_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(APPLY_CURSOR_SCRIPT)
    }

    fn bash(home: &Path, body: &str) -> std::process::Output {
        Command::new("bash")
            .arg("--noprofile")
            .arg("--norc")
            .arg("-c")
            .arg(body)
            .arg("bash")
            .arg(script_path())
            .env("HOME", home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .env_remove("XDG_DATA_HOME")
            .output()
            .expect("bash")
    }

    fn stdout(output: &std::process::Output) -> String {
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }

    fn stderr(output: &std::process::Output) -> String {
        String::from_utf8_lossy(&output.stderr).trim().to_owned()
    }

    #[test]
    fn exactly_three_left_pointer_themes() {
        assert_eq!(CursorTheme::ALL.len(), 3);
        let labels: Vec<_> = CursorTheme::ALL.iter().map(|theme| theme.label()).collect();
        assert_eq!(
            labels,
            [
                "Bibata Modern Classic",
                "Bibata Modern Ice",
                "Bibata Modern Amber"
            ]
        );
        for theme in CursorTheme::ALL {
            assert!(!theme.directory().contains("Right"));
            assert!(!theme.directory().contains("Original"));
            assert!(theme.download_url().ends_with(".tar.xz"));
            assert!(theme.download_url().contains("/v2.0.7/"));
            assert!(!theme.download_url().contains("latest"));
            assert!(!theme.download_url().contains("-Right"));
            assert!(!theme.download_url().contains(".zip"));
            assert_eq!(CursorTheme::from_directory(theme.directory()), Some(theme));
        }
        assert!(CursorTheme::from_directory("Bibata-Modern-Classic-Right").is_none());
        assert!(CursorTheme::from_directory("Bibata-Modern-Ice-Windows").is_none());
    }

    #[test]
    fn cursor_argv_is_one_theme_directory() {
        assert!(validate_cursor_argv(&["Bibata-Modern-Amber".into()]).is_ok());
        assert!(validate_cursor_argv(&[]).is_err());
        assert!(validate_cursor_argv(&["Bibata-Modern-Ice".into(), "extra".into()]).is_err());
        assert!(validate_cursor_argv(&["Bibata-Modern-Classic-Right".into()]).is_err());
        assert!(validate_cursor_argv(&["-Syu".into()]).is_err());
    }

    #[test]
    fn task_uses_helper_script_without_a_url_argument() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let task = CursorTheme::Ice.try_task(&base).unwrap();
        assert_eq!(task.description, "Applying Bibata Modern Ice");
        assert_eq!(task.command[0], "bash");
        assert!(task.command[1].ends_with("apply-cursor.sh"));
        assert_eq!(task.command.len(), 3);
        assert_eq!(task.command[2], "Bibata-Modern-Ice");
        assert!(!task.command.iter().any(|arg| arg.contains("http")));
    }

    #[test]
    fn pinned_urls_match_the_helper_script() {
        let script = fs::read_to_string(script_path()).unwrap();
        assert!(!script.contains("releases/latest"));
        assert!(!script.contains("sudo"));
        for theme in CursorTheme::ALL {
            assert!(
                script.contains(theme.download_url()),
                "{}",
                theme.download_url()
            );
        }
        for line in script.lines().filter(|line| line.contains("https://")) {
            assert!(line.contains("/v2.0.7/"), "{line}");
            assert!(line.contains(".tar.xz"), "{line}");
            assert!(!line.contains("-Right"), "{line}");
            assert!(!line.contains(".zip"), "{line}");
            assert!(!line.contains("latest"), "{line}");
        }
        let install = script
            .split("install_theme_from_archive()")
            .nth(1)
            .unwrap()
            .split("download_theme()")
            .next()
            .unwrap();
        assert!(install.contains("archive_is_safe"));
        assert!(install.contains("${HOME}/.local/share/icons"));
        assert!(!install.contains("/usr/share"));
        let present = script
            .split("theme_is_present()")
            .nth(1)
            .unwrap()
            .split("require_gnome_cursor_schema()")
            .next()
            .unwrap();
        assert!(present.contains("${HOME}/.local/share/icons/${theme}/index.theme"));
        assert!(present.contains("/usr/share/icons/${theme}/index.theme"));
        assert!(script.contains("The Cursor tab needs GNOME."));
        assert!(script.contains("gsettings set org.gnome.desktop.interface cursor-theme"));
        assert!(script.contains("XCURSOR_THEME="));
        assert!(script.contains("Inherits=${theme}"));
    }

    #[test]
    fn previews_are_embedded_pngs_and_rasterize_offline() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut colors = Vec::new();
        for theme in CursorTheme::ALL {
            let path = root.join("assets/cursors").join(format!(
                "{}.png",
                theme.preview_key().trim_start_matches("cursor-")
            ));
            let on_disk =
                fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(theme.preview_png(), on_disk.as_slice(), "{}", theme.label());
            let svg = preview_svg(theme.preview_png());
            assert!(svg.contains("href=\"data:image/png;base64,"));
            assert!(!svg.contains("href=\"http"));
            let raster = crate::logos::rasterize_svg_markup(&svg, 64)
                .unwrap_or_else(|| panic!("{} preview failed to rasterize", theme.label()));
            assert!(raster.rgba.chunks(4).any(|px| px[3] == 0));
            let opaque: Vec<_> = raster.rgba.chunks(4).filter(|px| px[3] > 200).collect();
            assert!(opaque.len() > 100, "{}", theme.label());
            let mut red = 0u32;
            let mut green = 0u32;
            let mut blue = 0u32;
            for px in &opaque {
                red += px[0] as u32;
                green += px[1] as u32;
                blue += px[2] as u32;
            }
            let count = opaque.len() as u32;
            colors.push((red / count, green / count, blue / count));
        }
        assert_ne!(colors[0], colors[1]);
        assert_ne!(colors[1], colors[2]);
        assert_ne!(colors[0], colors[2]);
    }

    #[test]
    fn sourcing_the_helper_does_not_apply_a_theme() {
        let home = TempHome::new();
        let output = bash(&home.path, r#"source "$1"; echo sourced"#);
        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), "sourced");
        assert!(!home.path.join(".local/share/icons").exists());
    }

    #[test]
    fn download_url_helper_rejects_right_and_windows_names() {
        let home = TempHome::new();
        let output = bash(
            &home.path,
            r#"source "$1"; cursor_download_url Bibata-Modern-Ice"#,
        );
        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), CursorTheme::Ice.download_url());

        let rejected = bash(
            &home.path,
            r#"source "$1"; cursor_download_url Bibata-Modern-Classic-Right"#,
        );
        assert!(!rejected.status.success());
        let windows = bash(
            &home.path,
            r#"source "$1"; cursor_download_url Bibata-Modern-Amber-Windows"#,
        );
        assert!(!windows.status.success());
    }

    fn write_theme(dir: &Path, with_index: bool, symlink: Option<(&str, &str)>) {
        let theme = dir.join("Bibata-Modern-Ice");
        let cursors = theme.join("cursors");
        fs::create_dir_all(&cursors).unwrap();
        if with_index {
            fs::write(
                theme.join("index.theme"),
                "[Icon Theme]\nName=Bibata-Modern-Ice\n",
            )
            .unwrap();
        }
        fs::write(cursors.join("left_ptr"), "pointer").unwrap();
        if let Some((name, target)) = symlink {
            std::os::unix::fs::symlink(target, cursors.join(name)).unwrap();
        }
    }

    fn pack(dir: &Path, archive: &Path) {
        let status = Command::new("tar")
            .args(["-cJf"])
            .arg(archive)
            .arg("-C")
            .arg(dir)
            .arg("Bibata-Modern-Ice")
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn archive_must_contain_the_theme_before_extract() {
        let home = TempHome::new();
        let work = home.path.join("work");
        fs::create_dir_all(&work).unwrap();
        write_theme(&work, true, Some(("arrow", "left_ptr")));
        let archive = home.path.join("ok.tar.xz");
        pack(&work, &archive);

        let output = bash(
            &home.path,
            &format!(
                r#"source "$1"; install_theme_from_archive Bibata-Modern-Ice "{}""#,
                archive.display()
            ),
        );
        assert!(output.status.success(), "{}", stderr(&output));
        let installed = home
            .path
            .join(".local/share/icons/Bibata-Modern-Ice/index.theme");
        assert!(installed.is_file(), "theme was not installed for the user");
        assert!(
            home.path
                .join(".local/share/icons/Bibata-Modern-Ice/cursors/arrow")
                .is_symlink()
        );

        let missing = home.path.join("missing");
        fs::create_dir_all(&missing).unwrap();
        write_theme(&missing, false, None);
        let bad = home.path.join("missing.tar.xz");
        pack(&missing, &bad);
        let rejected = bash(
            &home.path,
            &format!(
                r#"source "$1"; install_theme_from_archive Bibata-Modern-Ice "{}""#,
                bad.display()
            ),
        );
        assert!(!rejected.status.success());
        assert!(
            stderr(&rejected).contains("does not contain the Bibata-Modern-Ice theme directory")
        );
    }

    #[test]
    fn archive_rejects_escape_and_absolute_symlinks() {
        let home = TempHome::new();
        let work = home.path.join("work");
        fs::create_dir_all(&work).unwrap();
        write_theme(&work, true, Some(("evil", "/etc/passwd")));
        let archive = home.path.join("abs.tar.xz");
        pack(&work, &archive);
        let rejected = bash(
            &home.path,
            &format!(
                r#"source "$1"; archive_is_safe Bibata-Modern-Ice "{}""#,
                archive.display()
            ),
        );
        assert!(!rejected.status.success(), "{}", stdout(&rejected));
        assert!(stderr(&rejected).contains("absolute symlink"));
        assert!(
            !home
                .path
                .join(".local/share/icons/Bibata-Modern-Ice")
                .exists()
        );

        let outside = home.path.join("outside");
        fs::create_dir_all(outside.join("Bibata-Modern-Ice")).unwrap();
        fs::write(
            outside.join("Bibata-Modern-Ice/index.theme"),
            "[Icon Theme]\n",
        )
        .unwrap();
        fs::write(outside.join("readme.txt"), "nope").unwrap();
        let extra = home.path.join("extra.tar.xz");
        let status = Command::new("tar")
            .args(["-cJf"])
            .arg(&extra)
            .arg("-C")
            .arg(&outside)
            .args(["Bibata-Modern-Ice", "readme.txt"])
            .status()
            .unwrap();
        assert!(status.success());
        let extra_out = bash(
            &home.path,
            &format!(
                r#"source "$1"; archive_is_safe Bibata-Modern-Ice "{}""#,
                extra.display()
            ),
        );
        assert!(!extra_out.status.success());
        assert!(stderr(&extra_out).contains("outside"));
    }

    #[test]
    fn user_cursor_files_record_theme_and_xcursor_theme() {
        let home = TempHome::new();
        let output = bash(
            &home.path,
            r#"source "$1"; write_user_cursor_files Bibata-Modern-Amber"#,
        );
        assert!(output.status.success(), "{}", stderr(&output));
        let xdg =
            fs::read_to_string(home.path.join(".local/share/icons/default/index.theme")).unwrap();
        assert!(xdg.contains("Inherits=Bibata-Modern-Amber"));
        let legacy = fs::read_to_string(home.path.join(".icons/default/index.theme")).unwrap();
        assert!(legacy.contains("Inherits=Bibata-Modern-Amber"));
        let env = fs::read_to_string(
            home.path
                .join(".config/environment.d/99-toolbox-cursor.conf"),
        )
        .unwrap();
        assert_eq!(env, "XCURSOR_THEME=Bibata-Modern-Amber\n");
    }

    #[test]
    fn missing_gnome_schema_fails_with_a_clear_message() {
        let home = TempHome::new();
        let bin = home.path.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            bin.join("gsettings"),
            "#!/bin/bash\nif [[ \"$1\" == list-schemas ]]; then echo org.gtk.Settings; exit 0; fi\nexit 1\n",
        )
        .unwrap();
        fs::set_permissions(bin.join("gsettings"), fs::Permissions::from_mode(0o755)).unwrap();

        let output = Command::new("bash")
            .arg("--noprofile")
            .arg("--norc")
            .arg("-c")
            .arg(r#"source "$1"; require_gnome_cursor_schema"#)
            .arg("bash")
            .arg(script_path())
            .env("HOME", &home.path)
            .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
            .output()
            .unwrap();
        assert!(!output.status.success());
        let err = stderr(&output);
        assert!(err.contains("The Cursor tab needs GNOME."), "{err}");
        assert!(err.contains("org.gnome.desktop.interface"));
    }

    #[test]
    fn theme_already_under_user_icons_is_present() {
        let home = TempHome::new();
        let theme = home.path.join(".local/share/icons/Bibata-Modern-Classic");
        fs::create_dir_all(&theme).unwrap();
        fs::write(theme.join("index.theme"), "[Icon Theme]\n").unwrap();
        let output = bash(
            &home.path,
            r#"source "$1"; theme_is_present Bibata-Modern-Classic"#,
        );
        assert!(output.status.success(), "{}", stderr(&output));

        let absent = bash(
            &home.path,
            r#"source "$1"; theme_is_present Bibata-Modern-Amber"#,
        );
        assert!(!absent.status.success());
    }
}
