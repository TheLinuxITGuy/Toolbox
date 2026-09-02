//! Logo SVG markup compiled into the binary.
//!
//! Distro logos (`assets/distros/*.svg`) are embedded with `include_str!` and
//! rasterized from that markup. App tile logos in `assets/icons` are PNG and
//! are embedded separately with `include_bytes!` in `main`. Neither path is
//! read from disk at runtime, so logos still render if `assets/` is missing
//! next to the executable.

use eframe::egui::ColorImage;

/// Distro SVG markup keyed the same way the header chips look them up.
pub const DISTRO_SVGS: &[(&str, &str)] = &[
    ("arch", include_str!("../assets/distros/arch.svg")),
    ("debian", include_str!("../assets/distros/debian.svg")),
    ("fedora", include_str!("../assets/distros/fedora.svg")),
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

    #[test]
    fn distro_svgs_are_embedded_markup() {
        assert_eq!(DISTRO_SVGS.len(), 3);
        for (name, svg) in DISTRO_SVGS {
            assert!(
                svg.contains("<svg"),
                "{name} markup is missing an <svg> tag"
            );
            assert!(
                !svg.starts_with("assets/"),
                "{name} looks like a filesystem path rather than markup"
            );
        }
    }

    #[test]
    fn rasterize_uses_embedded_markup_not_assets_dir() {
        for (name, svg) in DISTRO_SVGS {
            let raster = rasterize_svg_markup(svg, 64)
                .unwrap_or_else(|| panic!("{name} SVG failed to rasterize"));
            assert_eq!(raster.size, [64, 64], "{name}");
            assert_eq!(raster.rgba.len(), 64 * 64 * 4, "{name}");
            assert!(
                raster.rgba.chunks(4).any(|px| px[3] != 0),
                "{name} raster was fully transparent"
            );
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
