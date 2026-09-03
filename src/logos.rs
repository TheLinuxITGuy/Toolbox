//! Logos compiled into the binary.
//!
//! Distro chips, app tiles, and the Brave Origin tile use SVG markup via
//! `include_str!`. Icons are vendored from dashboard-icons (Apache-2.0) or
//! Simple Icons (CC0). See `assets/logos/NOTICE`. Nothing under `assets/` is
//! read from disk at runtime.

use eframe::egui::ColorImage;

/// Distro SVG markup keyed the same way the header chips look them up.
pub const DISTRO_SVGS: &[(&str, &str)] = &[
    ("arch", include_str!("../assets/distros/arch.svg")),
    ("debian", include_str!("../assets/distros/debian.svg")),
    ("fedora", include_str!("../assets/distros/fedora.svg")),
];

/// Chris's Brave Origin tile: white outlined lion on a dark rounded square.
/// Do not reuse the orange Brave Browser mark.
pub const BRAVE_ORIGIN_SVG: &str = include_str!("../assets/logos/brave-origin.svg");

/// Colorful dashboard-icons SVGs keyed by dashboardicons slug.
pub const APP_SVGS: &[(&str, &str)] = &[
    ("audacity", include_str!("../assets/logos/audacity.svg")),
    ("bitwarden", include_str!("../assets/logos/bitwarden.svg")),
    ("brave", include_str!("../assets/logos/brave.svg")),
    ("discord", include_str!("../assets/logos/discord.svg")),
    ("firefox", include_str!("../assets/logos/firefox.svg")),
    ("gimp", include_str!("../assets/logos/gimp.svg")),
    (
        "google-chrome",
        include_str!("../assets/logos/google-chrome.svg"),
    ),
    (
        "libreoffice",
        include_str!("../assets/logos/libreoffice.svg"),
    ),
    ("librewolf", include_str!("../assets/logos/librewolf.svg")),
    (
        "microsoft-edge",
        include_str!("../assets/logos/microsoft-edge.svg"),
    ),
    ("obsidian", include_str!("../assets/logos/obsidian.svg")),
    ("onlyoffice", include_str!("../assets/logos/onlyoffice.svg")),
    ("opera", include_str!("../assets/logos/opera.svg")),
    ("proton-vpn", include_str!("../assets/logos/proton-vpn.svg")),
    (
        "qbittorrent",
        include_str!("../assets/logos/qbittorrent.svg"),
    ),
    ("signal", include_str!("../assets/logos/signal.svg")),
    ("slack", include_str!("../assets/logos/slack.svg")),
    ("spotify", include_str!("../assets/logos/spotify.svg")),
    ("steam", include_str!("../assets/logos/steam.svg")),
    ("stremio", include_str!("../assets/logos/stremio.svg")),
    ("telegram", include_str!("../assets/logos/telegram.svg")),
    (
        "thunderbird",
        include_str!("../assets/logos/thunderbird.svg"),
    ),
    (
        "visual-studio-code",
        include_str!("../assets/logos/visual-studio-code.svg"),
    ),
    ("vivaldi", include_str!("../assets/logos/vivaldi.svg")),
    (
        "zen-browser",
        include_str!("../assets/logos/zen-browser.svg"),
    ),
    (
        "zen-browser-dark",
        include_str!("../assets/logos/zen-browser-dark.svg"),
    ),
];

/// Simple Icons monochrome SVGs. Tint with chrome text at paint time.
pub const MONO_SVGS: &[(&str, &str)] = &[
    ("heroic", include_str!("../assets/logos/heroic.svg")),
    ("htop", include_str!("../assets/logos/htop.svg")),
    ("lutris", include_str!("../assets/logos/lutris.svg")),
    ("mpv", include_str!("../assets/logos/mpv.svg")),
    ("obs-studio", include_str!("../assets/logos/obs-studio.svg")),
    (
        "pycharm-community",
        include_str!("../assets/logos/pycharm-community.svg"),
    ),
    ("vlc", include_str!("../assets/logos/vlc.svg")),
];

pub fn is_monochrome_icon(key: &str) -> bool {
    MONO_SVGS.iter().any(|(slug, _)| *slug == key)
}

pub struct Raster {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

fn svg_parse_options() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    options
}

/// Rasterize SVG markup that is already in memory. This never opens a path.
pub fn rasterize_svg_markup(svg: &str, size: u32) -> Option<Raster> {
    let options = svg_parse_options();
    let tree = usvg::Tree::from_str(svg, &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let tree_size = tree.size();
    let scale = (size as f32 / tree_size.width()).min(size as f32 / tree_size.height());
    let tx = (size as f32 - tree_size.width() * scale) / 2.0;
    let ty = (size as f32 - tree_size.height() * scale) / 2.0;
    let transform = tiny_skia::Transform::from_translate(tx, ty).pre_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(Raster {
        size: [size as usize, size as usize],
        rgba: pixmap.data().to_vec(),
    })
}

pub fn color_image_from_raster(raster: &Raster) -> ColorImage {
    ColorImage::from_rgba_unmultiplied(raster.size, &raster.rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn assert_markup(name: &str, svg: &str) {
        assert!(
            svg.contains("<svg"),
            "{name} markup is missing an <svg> tag"
        );
        assert!(
            !svg.starts_with("assets/"),
            "{name} looks like a filesystem path rather than markup"
        );
    }

    fn assert_raster(name: &str, svg: &str) {
        let raster = rasterize_svg_markup(svg, 64)
            .unwrap_or_else(|| panic!("{name} SVG failed to rasterize"));
        assert_eq!(raster.size, [64, 64], "{name}");
        assert_eq!(raster.rgba.len(), 64 * 64 * 4, "{name}");
        assert!(
            raster.rgba.chunks(4).any(|px| px[3] != 0),
            "{name} raster was fully transparent"
        );
    }

    #[test]
    fn distro_svgs_are_embedded_markup() {
        assert_eq!(DISTRO_SVGS.len(), 3);
        for (name, svg) in DISTRO_SVGS {
            assert_markup(name, svg);
            assert_raster(name, svg);
        }
    }

    #[test]
    fn brave_origin_svg_is_chris_markup() {
        assert_markup("brave-origin", BRAVE_ORIGIN_SVG);
        assert_raster("brave-origin", BRAVE_ORIGIN_SVG);
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let on_disk = std::fs::read_to_string(root.join("assets/logos/brave-origin.svg")).unwrap();
        assert_eq!(BRAVE_ORIGIN_SVG, on_disk.as_str());
    }

    #[test]
    fn app_svgs_are_embedded_dashboardicons_markup() {
        let keys: Vec<&str> = APP_SVGS.iter().map(|(key, _)| *key).collect();
        assert!(keys.contains(&"brave"));
        assert!(keys.contains(&"zen-browser"));
        assert!(keys.contains(&"zen-browser-dark"));
        assert!(!keys.contains(&"brave-origin"));
        for (name, svg) in APP_SVGS {
            assert_markup(name, svg);
            assert_raster(name, svg);
        }
    }

    #[test]
    fn mono_svgs_are_simple_icons_markup() {
        let keys: Vec<&str> = MONO_SVGS.iter().map(|(key, _)| *key).collect();
        for expected in [
            "vlc",
            "lutris",
            "heroic",
            "obs-studio",
            "mpv",
            "htop",
            "pycharm-community",
        ] {
            assert!(keys.contains(&expected), "{expected}");
        }
        for (name, svg) in MONO_SVGS {
            assert_markup(name, svg);
            assert_raster(name, svg);
        }
        assert!(is_monochrome_icon("vlc"));
        assert!(!is_monochrome_icon("brave"));
    }

    #[test]
    fn include_str_matches_checkout_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (name, embedded) in DISTRO_SVGS {
            let path = root.join("assets/distros").join(format!("{name}.svg"));
            let on_disk = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            assert_eq!(*embedded, on_disk.as_str(), "{name}");
        }
        for (name, embedded) in APP_SVGS.iter().chain(MONO_SVGS.iter()) {
            let path = root.join("assets/logos").join(format!("{name}.svg"));
            let on_disk = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            assert_eq!(*embedded, on_disk.as_str(), "{name}");
        }
    }

    #[test]
    fn assets_icons_dir_is_gone() {
        let icons = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        assert!(!icons.exists(), "assets/icons must stay removed");
    }

    #[test]
    fn app_rasters_are_not_shipped() {
        let logos = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/logos");
        let pngs: Vec<_> = std::fs::read_dir(&logos)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("png"))
            .collect();
        assert!(
            pngs.is_empty(),
            "app tile PNGs must not be shipped: {pngs:?}"
        );
        let prod = include_str!("logos.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("logos module");
        assert!(!prod.contains("APP_PNGS"));
        assert!(!prod.contains("include_bytes!"));
        assert!(!prod.contains(".png"));
    }

    #[test]
    fn brave_origin_is_not_the_orange_brave_mark() {
        let brave = APP_SVGS
            .iter()
            .find(|(key, _)| *key == "brave")
            .map(|(_, svg)| *svg)
            .expect("brave svg");
        assert_ne!(BRAVE_ORIGIN_SVG, brave);
        assert!(
            !BRAVE_ORIGIN_SVG.to_ascii_lowercase().contains("#fb542b"),
            "Origin SVG must not use the orange Brave fill"
        );
        let origin = rasterize_svg_markup(BRAVE_ORIGIN_SVG, 32).unwrap();
        let brave_img = rasterize_svg_markup(brave, 32).unwrap();
        assert_ne!(origin.rgba, brave_img.rgba);
    }

    #[test]
    fn letter_fallback_apps_have_no_invented_icon_files() {
        let logos = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/logos");
        for forbidden in [
            "bottles.svg",
            "boxes.svg",
            "protonup-qt.svg",
            "gparted.svg",
            "localsend.svg",
            "sober.svg",
            "flatseal.svg",
            "prism-launcher.svg",
            "retroarch.svg",
            "extension-manager.svg",
            "dolphin-emu.svg",
            "ppsspp.svg",
            "gear-lever.svg",
            "protonplus.svg",
            "mission-center.svg",
        ] {
            assert!(
                !logos.join(forbidden).exists(),
                "do not invent a dashboardicons file for {forbidden}"
            );
        }
        let keys: Vec<&str> = APP_SVGS
            .iter()
            .chain(MONO_SVGS.iter())
            .map(|(key, _)| *key)
            .collect();
        for forbidden in [
            "bottles",
            "boxes",
            "protonup-qt",
            "gparted",
            "localsend",
            "sober",
            "flatseal",
            "prism-launcher",
            "retroarch",
            "extension-manager",
            "dolphin-emu",
            "ppsspp",
            "gear-lever",
            "protonplus",
            "mission-center",
        ] {
            assert!(!keys.contains(&forbidden), "{forbidden}");
        }
    }

    #[test]
    fn notice_attributes_vendored_collections() {
        let notice = include_str!("../assets/logos/NOTICE");
        assert!(notice.contains("Apache License 2.0"));
        assert!(notice.contains("dashboard-icons"));
        assert!(notice.contains("Simple Icons"));
        assert!(notice.contains("brave-origin.svg"));
    }

    #[test]
    fn rasterize_ignores_external_file_hrefs() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8">
            <image href="/nonexistent/toolbox-logo.svg" width="8" height="8"/>
            <rect x="1" y="1" width="6" height="6" fill="#ff0000"/>
        </svg>"##;
        let raster = rasterize_svg_markup(svg, 8).expect("markup with a missing href");
        assert!(raster.rgba.chunks(4).any(|px| px[3] != 0));
    }
}
