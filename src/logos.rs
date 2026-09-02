//! Logo SVG markup compiled into the binary.
//!
//! Distro logos live in `assets/distros/*.svg` and app tile logos in
//! `assets/logos/*.svg`. Both are embedded with `include_str!` and rasterized
//! from that markup. Nothing under `assets/` is read from disk at runtime, so
//! logos still render if the checkout's assets folder is missing next to the
//! executable.

use eframe::egui::ColorImage;

/// Distro SVG markup keyed the same way the header chips look them up.
pub const DISTRO_SVGS: &[(&str, &str)] = &[
    ("arch", include_str!("../assets/distros/arch.svg")),
    ("debian", include_str!("../assets/distros/debian.svg")),
    ("fedora", include_str!("../assets/distros/fedora.svg")),
];

/// App tile SVG markup. Brave Origin uses the official outlined lion, not the
/// orange Brave Browser mark.
pub const APP_SVGS: &[(&str, &str)] = &[
    ("audacity", include_str!("../assets/logos/audacity.svg")),
    ("bottles", include_str!("../assets/logos/bottles.svg")),
    ("boxes", include_str!("../assets/logos/boxes.svg")),
    ("brave", include_str!("../assets/logos/brave.svg")),
    (
        "brave-origin",
        include_str!("../assets/logos/brave-origin.svg"),
    ),
    ("discord", include_str!("../assets/logos/discord.svg")),
    ("firefox", include_str!("../assets/logos/firefox.svg")),
    ("gimp", include_str!("../assets/logos/gimp.svg")),
    (
        "google-chrome",
        include_str!("../assets/logos/google-chrome.svg"),
    ),
    ("htop", include_str!("../assets/logos/htop.svg")),
    (
        "libreoffice",
        include_str!("../assets/logos/libreoffice.svg"),
    ),
    ("localsend", include_str!("../assets/logos/localsend.svg")),
    ("lutris", include_str!("../assets/logos/lutris.svg")),
    (
        "microsoft-edge",
        include_str!("../assets/logos/microsoft-edge.svg"),
    ),
    ("mpv", include_str!("../assets/logos/mpv.svg")),
    ("obs-studio", include_str!("../assets/logos/obs-studio.svg")),
    ("onlyoffice", include_str!("../assets/logos/onlyoffice.svg")),
    ("opera", include_str!("../assets/logos/opera.svg")),
    (
        "protonup-qt",
        include_str!("../assets/logos/protonup-qt.svg"),
    ),
    (
        "pycharm-community",
        include_str!("../assets/logos/pycharm-community.svg"),
    ),
    ("signal", include_str!("../assets/logos/signal.svg")),
    ("slack", include_str!("../assets/logos/slack.svg")),
    ("steam", include_str!("../assets/logos/steam.svg")),
    (
        "thunderbird",
        include_str!("../assets/logos/thunderbird.svg"),
    ),
    (
        "visual-studio-code",
        include_str!("../assets/logos/visual-studio-code.svg"),
    ),
    ("vivaldi", include_str!("../assets/logos/vivaldi.svg")),
    ("vlc", include_str!("../assets/logos/vlc.svg")),
    ("zen", include_str!("../assets/logos/zen.svg")),
];

pub struct Raster {
    pub size: [usize; 2],
    pub rgba: Vec<u8>,
}

fn svg_parse_options() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    // Embedded logos are self-contained. Do not resolve file: or relative
    // hrefs from disk (or the network) at runtime.
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
        }
    }

    #[test]
    fn app_svgs_are_embedded_markup() {
        assert!(APP_SVGS.len() > 20);
        for (name, svg) in APP_SVGS {
            assert_markup(name, svg);
        }
        let keys: Vec<&str> = APP_SVGS.iter().map(|(key, _)| *key).collect();
        assert!(keys.contains(&"brave"));
        assert!(keys.contains(&"brave-origin"));
    }

    #[test]
    fn rasterize_uses_embedded_markup_not_assets_dir() {
        for (name, svg) in DISTRO_SVGS.iter().chain(APP_SVGS.iter()) {
            assert_raster(name, svg);
        }
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
        for (name, embedded) in APP_SVGS {
            let path = root.join("assets/logos").join(format!("{name}.svg"));
            let on_disk = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            assert_eq!(*embedded, on_disk.as_str(), "{name}");
        }
    }

    #[test]
    fn assets_icons_dir_is_gone() {
        let icons = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        assert!(
            !icons.exists(),
            "assets/icons must be removed; app logos are compile-time SVG now"
        );
    }

    #[test]
    fn brave_origin_is_not_the_orange_brave_mark() {
        let brave = APP_SVGS
            .iter()
            .find(|(key, _)| *key == "brave")
            .map(|(_, svg)| *svg)
            .expect("brave");
        let origin = APP_SVGS
            .iter()
            .find(|(key, _)| *key == "brave-origin")
            .map(|(_, svg)| *svg)
            .expect("brave-origin");
        assert_ne!(brave, origin);
        assert!(
            origin.contains("fill=\"#111111\"") || origin.contains("fill=\"#000000\""),
            "Origin lion should be black monochrome, got: {}",
            &origin[..origin.len().min(200)]
        );
        assert!(
            !origin.to_ascii_lowercase().contains("#fb542b"),
            "Origin lion must not use the orange Brave fill"
        );
        let brave_raster = rasterize_svg_markup(brave, 32).unwrap();
        let origin_raster = rasterize_svg_markup(origin, 32).unwrap();
        assert_ne!(
            brave_raster.rgba, origin_raster.rgba,
            "Brave Origin raster must not match Brave Browser"
        );
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
