//! Logos compiled into the binary.
//!
//! Distro chips and the Brave Origin tile use SVG markup via `include_str!`.
//! Other app tiles use Chris's original PNGs via `include_bytes!`. Nothing
//! under `assets/` is read from disk at runtime.

use eframe::egui::ColorImage;

/// Distro SVG markup keyed the same way the header chips look them up.
pub const DISTRO_SVGS: &[(&str, &str)] = &[
    ("arch", include_str!("../assets/distros/arch.svg")),
    ("debian", include_str!("../assets/distros/debian.svg")),
    ("fedora", include_str!("../assets/distros/fedora.svg")),
];

/// Chris's Brave Origin tile: white outlined lion on a dark rounded square.
/// Do not reuse the orange Brave Browser PNG.
pub const BRAVE_ORIGIN_SVG: &str = include_str!("../assets/logos/brave-origin.svg");

/// Original app tile PNGs. Brave Browser is `brave`; Origin is not in this list.
pub const APP_PNGS: &[(&str, &[u8])] = &[
    ("audacity", include_bytes!("../assets/logos/audacity.png")),
    ("bottles", include_bytes!("../assets/logos/bottles.png")),
    ("boxes", include_bytes!("../assets/logos/boxes.png")),
    ("brave", include_bytes!("../assets/logos/brave.png")),
    ("discord", include_bytes!("../assets/logos/discord.png")),
    ("firefox", include_bytes!("../assets/logos/firefox.png")),
    ("gimp", include_bytes!("../assets/logos/gimp.png")),
    (
        "google-chrome",
        include_bytes!("../assets/logos/google-chrome.png"),
    ),
    ("localsend", include_bytes!("../assets/logos/localsend.png")),
    ("lutris", include_bytes!("../assets/logos/lutris.png")),
    (
        "microsoft-edge",
        include_bytes!("../assets/logos/microsoft-edge.png"),
    ),
    (
        "obs-studio",
        include_bytes!("../assets/logos/obs-studio.png"),
    ),
    (
        "onlyoffice",
        include_bytes!("../assets/logos/onlyoffice.png"),
    ),
    ("opera", include_bytes!("../assets/logos/opera.png")),
    (
        "protonup-qt",
        include_bytes!("../assets/logos/protonup-qt.png"),
    ),
    (
        "pycharm-community",
        include_bytes!("../assets/logos/pycharm-community.png"),
    ),
    ("signal", include_bytes!("../assets/logos/signal.png")),
    ("slack", include_bytes!("../assets/logos/slack.png")),
    ("steam", include_bytes!("../assets/logos/steam.png")),
    (
        "thunderbird",
        include_bytes!("../assets/logos/thunderbird.png"),
    ),
    (
        "visual-studio-code",
        include_bytes!("../assets/logos/visual-studio-code.png"),
    ),
    ("vivaldi", include_bytes!("../assets/logos/vivaldi.png")),
    ("vlc", include_bytes!("../assets/logos/vlc.png")),
    ("zen", include_bytes!("../assets/logos/zen.png")),
];

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

pub fn decode_png(bytes: &[u8]) -> Option<ColorImage> {
    let image = image::load_from_memory(bytes).ok()?;
    let rgba = image.to_rgba8();
    Some(ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
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
    fn app_pngs_decode_without_assets_dir() {
        assert_eq!(APP_PNGS.len(), 24);
        let keys: Vec<&str> = APP_PNGS.iter().map(|(key, _)| *key).collect();
        assert!(keys.contains(&"brave"));
        assert!(!keys.contains(&"brave-origin"));
        assert!(!keys.contains(&"htop"));
        assert!(!keys.contains(&"libreoffice"));
        assert!(!keys.contains(&"mpv"));
        for (name, bytes) in APP_PNGS {
            let image = decode_png(bytes).unwrap_or_else(|| panic!("{name} PNG failed to decode"));
            assert!(image.width() > 0 && image.height() > 0, "{name}");
        }
    }

    #[test]
    fn include_str_and_bytes_match_checkout_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for (name, embedded) in DISTRO_SVGS {
            let path = root.join("assets/distros").join(format!("{name}.svg"));
            let on_disk = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            assert_eq!(*embedded, on_disk.as_str(), "{name}");
        }
        for (name, bytes) in APP_PNGS {
            let path = root.join("assets/logos").join(format!("{name}.png"));
            let on_disk = std::fs::read(&path)
                .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
            assert_eq!(*bytes, on_disk.as_slice(), "{name}");
        }
    }

    #[test]
    fn assets_icons_dir_is_gone() {
        let icons = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/icons");
        assert!(!icons.exists(), "assets/icons must stay removed");
    }

    #[test]
    fn brave_origin_is_not_the_orange_brave_png() {
        let brave = APP_PNGS
            .iter()
            .find(|(key, _)| *key == "brave")
            .map(|(_, bytes)| *bytes)
            .expect("brave png");
        assert!(!BRAVE_ORIGIN_SVG.as_bytes().eq(brave));
        assert!(
            !BRAVE_ORIGIN_SVG.to_ascii_lowercase().contains("#fb542b"),
            "Origin SVG must not use the orange Brave fill"
        );
        let origin = rasterize_svg_markup(BRAVE_ORIGIN_SVG, 32).unwrap();
        let brave_img = decode_png(brave).unwrap();
        assert_ne!(
            origin.rgba.len(),
            0,
            "Origin SVG raster must produce pixels"
        );
        assert!(brave_img.width() > 0);
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
