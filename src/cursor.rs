//! GNOME Bibata cursor themes.
//!
//! Previews are the v2.0.7 `left_ptr.png` bitmaps from the Bibata release,
//! compiled into the binary. Theme archives are downloaded only when Apply
//! or the run queue installs one.

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
    // Vendored files are the 256×256 left_ptr bitmaps.
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"256\" height=\"256\" viewBox=\"0 0 256 256\"><image width=\"256\" height=\"256\" href=\"data:image/png;base64,{}\"/></svg>",
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

    fn sha256(bytes: &[u8]) -> String {
        Sha256::hash(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// Compact SHA-256 so the vendored Bibata bitmaps stay pinned without a new crate.
    struct Sha256 {
        state: [u32; 8],
        bits: u64,
        block: [u8; 64],
        filled: usize,
    }

    impl Sha256 {
        fn hash(bytes: &[u8]) -> [u8; 32] {
            let mut hasher = Self {
                state: [
                    0x6a09_e667,
                    0xbb67_ae85,
                    0x3c6e_f372,
                    0xa54f_f53a,
                    0x510e_527f,
                    0x9b05_688c,
                    0x1f83_d9ab,
                    0x5be0_cd19,
                ],
                bits: 0,
                block: [0; 64],
                filled: 0,
            };
            hasher.update(bytes);
            hasher.finish()
        }

        fn update(&mut self, mut data: &[u8]) {
            self.bits = self.bits.wrapping_add((data.len() as u64).wrapping_mul(8));
            if self.filled > 0 {
                let take = (64 - self.filled).min(data.len());
                self.block[self.filled..self.filled + take].copy_from_slice(&data[..take]);
                self.filled += take;
                data = &data[take..];
                if self.filled == 64 {
                    let block = self.block;
                    self.compress(&block);
                    self.filled = 0;
                }
            }
            while data.len() >= 64 {
                self.compress(&data[..64]);
                data = &data[64..];
            }
            if !data.is_empty() {
                self.block[..data.len()].copy_from_slice(data);
                self.filled = data.len();
            }
        }

        fn finish(mut self) -> [u8; 32] {
            let bits = self.bits;
            let mut tail = [0u8; 128];
            tail[0] = 0x80;
            let pad = if self.filled < 56 {
                56 - self.filled
            } else {
                120 - self.filled
            };
            tail[pad..pad + 8].copy_from_slice(&bits.to_be_bytes());
            // `update` counts these bytes toward the length; restore the message length.
            self.update(&tail[..pad + 8]);
            self.bits = bits;
            let mut out = [0u8; 32];
            for (index, word) in self.state.iter().enumerate() {
                out[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
            }
            out
        }

        fn compress(&mut self, block: &[u8]) {
            const K: [u32; 64] = [
                0x428a_2f98,
                0x7137_4491,
                0xb5c0_fbcf,
                0xe9b5_dba5,
                0x3956_c25b,
                0x59f1_11f1,
                0x923f_82a4,
                0xab1c_5ed5,
                0xd807_aa98,
                0x1283_5b01,
                0x2431_85be,
                0x550c_7dc3,
                0x72be_5d74,
                0x80de_b1fe,
                0x9bdc_06a7,
                0xc19b_f174,
                0xe49b_69c1,
                0xefbe_4786,
                0x0fc1_9dc6,
                0x240c_a1cc,
                0x2de9_2c6f,
                0x4a74_84aa,
                0x5cb0_a9dc,
                0x76f9_88da,
                0x983e_5152,
                0xa831_c66d,
                0xb003_27c8,
                0xbf59_7fc7,
                0xc6e0_0bf3,
                0xd5a7_9147,
                0x06ca_6351,
                0x1429_2967,
                0x27b7_0a85,
                0x2e1b_2138,
                0x4d2c_6dfc,
                0x5338_0d13,
                0x650a_7354,
                0x766a_0abb,
                0x81c2_c92e,
                0x9272_2c85,
                0xa2bf_e8a1,
                0xa81a_664b,
                0xc24b_8b70,
                0xc76c_51a3,
                0xd192_e819,
                0xd699_0624,
                0xf40e_3585,
                0x106a_a070,
                0x19a4_c116,
                0x1e37_6c08,
                0x2748_774c,
                0x34b0_bcb5,
                0x391c_0cb3,
                0x4ed8_aa4a,
                0x5b9c_ca4f,
                0x682e_6ff3,
                0x748f_82ee,
                0x78a5_636f,
                0x84c8_7814,
                0x8cc7_0208,
                0x90be_fffa,
                0xa450_6ceb,
                0xbef9_a3f7,
                0xc671_78f2,
            ];
            let mut words = [0u32; 64];
            for index in 0..16 {
                words[index] =
                    u32::from_be_bytes(block[index * 4..index * 4 + 4].try_into().unwrap());
            }
            for index in 16..64 {
                let s0 = words[index - 15].rotate_right(7)
                    ^ words[index - 15].rotate_right(18)
                    ^ (words[index - 15] >> 3);
                let s1 = words[index - 2].rotate_right(17)
                    ^ words[index - 2].rotate_right(19)
                    ^ (words[index - 2] >> 10);
                words[index] = words[index - 16]
                    .wrapping_add(s0)
                    .wrapping_add(words[index - 7])
                    .wrapping_add(s1);
            }
            let mut a = self.state[0];
            let mut b = self.state[1];
            let mut c = self.state[2];
            let mut d = self.state[3];
            let mut e = self.state[4];
            let mut f = self.state[5];
            let mut g = self.state[6];
            let mut h = self.state[7];
            for index in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ (!e & g);
                let t1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[index])
                    .wrapping_add(words[index]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let t2 = s0.wrapping_add(maj);
                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(t1);
                d = c;
                c = b;
                b = a;
                a = t1.wrapping_add(t2);
            }
            let next = [
                self.state[0].wrapping_add(a),
                self.state[1].wrapping_add(b),
                self.state[2].wrapping_add(c),
                self.state[3].wrapping_add(d),
                self.state[4].wrapping_add(e),
                self.state[5].wrapping_add(f),
                self.state[6].wrapping_add(g),
                self.state[7].wrapping_add(h),
            ];
            self.state = next;
        }
    }

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
        assert_eq!(
            sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        let mut colors = Vec::new();
        for theme in CursorTheme::ALL {
            let path = root.join("assets/cursors").join(format!(
                "{}.png",
                theme.preview_key().trim_start_matches("cursor-")
            ));
            let on_disk =
                fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(theme.preview_png(), on_disk.as_slice(), "{}", theme.label());
            assert_eq!(
                sha256(theme.preview_png()),
                match theme {
                    CursorTheme::Classic => {
                        "b4bd4c1287b1b7d62bf5a992cee319b7fd8a3d3babe42fa7e9105ec98e0f05b1"
                    }
                    CursorTheme::Ice => {
                        "056e5535d634e651a1badb762b2b152fc9e21defb9eef6f907628965039354be"
                    }
                    CursorTheme::Amber => {
                        "119f61f19806aa924ccb77d003e30806304e9274e072eb98cfe85350cc5e7707"
                    }
                },
                "{} must stay the v2.0.7 left_ptr bitmap",
                theme.label()
            );
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
