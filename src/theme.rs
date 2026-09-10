//! Lumen themes for Toolbox chrome.
//!
//! Dark (default): void canvas, teal accent, no navy/yellow brand colors.
//! Light is toggle-only and is never the startup default.

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{Color32, Context, Painter, Pos2, Rect, Shape, Stroke, Vec2, Visuals, vec2};

pub const VOID_DARK: Color32 = Color32::from_rgb(0x07, 0x08, 0x0A);
pub const SURFACE_DARK: Color32 = Color32::from_rgb(0x11, 0x13, 0x18);
pub const ELEVATED_DARK: Color32 = Color32::from_rgb(0x1A, 0x1D, 0x26);
pub const TEXT_DARK: Color32 = Color32::from_rgb(0xF4, 0xF5, 0xF7);
pub const MUTED_DARK: Color32 = Color32::from_rgb(0x8B, 0x92, 0xA5);
pub const ACCENT_DARK: Color32 = Color32::from_rgb(0x5E, 0xEA, 0xD4);
pub const ACCENT_ON_DARK: Color32 = Color32::from_rgb(0x07, 0x08, 0x0A);
pub const DANGER: Color32 = Color32::from_rgb(0xFF, 0x6B, 0x6B);

pub const VOID_LIGHT: Color32 = Color32::from_rgb(0xF0, 0xF2, 0xF5);
pub const SURFACE_LIGHT: Color32 = Color32::WHITE;
pub const ELEVATED_LIGHT: Color32 = Color32::from_rgb(0xF7, 0xF8, 0xFA);
pub const TEXT_LIGHT: Color32 = Color32::from_rgb(0x0B, 0x12, 0x20);
pub const MUTED_LIGHT: Color32 = Color32::from_rgb(0x64, 0x74, 0x8B);
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0x0D, 0x94, 0x88);
pub const ACCENT_BRIGHT: Color32 = Color32::from_rgb(0x2D, 0xD4, 0xBF);
pub const ACCENT_ON_LIGHT: Color32 = Color32::from_rgb(0x04, 0x2F, 0x2E);
pub const BORDER_LIGHT_BASE: Color32 = Color32::from_rgb(0x0F, 0x17, 0x2A);

const CONFIG_DIR_NAME: &str = "linux-it-guy-toolbox";
const THEME_FILE_NAME: &str = "theme";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "dark" => Some(Self::Dark),
            "light" => Some(Self::Light),
            _ => None,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::Dark,
        }
    }

    pub fn palette(self) -> Palette {
        match self {
            Self::Dark => Palette::dark(),
            Self::Light => Palette::light(),
        }
    }

    pub fn toggle_tooltip(self) -> &'static str {
        match self {
            Self::Dark => "Switch to light theme",
            Self::Light => "Switch to dark theme",
        }
    }
}

/// Derived chrome colors for one Lumen theme.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub mode: ThemeMode,
    pub void: Color32,
    pub surface: Color32,
    pub elevated: Color32,
    pub border: Color32,
    pub border_strong: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub accent_dim: Color32,
    pub accent_bright: Color32,
    pub accent_on: Color32,
    pub danger: Color32,
    pub tile: Color32,
    pub tile_selected: Color32,
    pub cta_fill: Color32,
    pub cta_text: Color32,
    pub widget_bg: Color32,
    pub widget_hover: Color32,
    pub input_fill: Color32,
    pub input_border: Color32,
    pub nav_hover: Color32,
    pub filter_selected_fill: Color32,
    pub filter_selected_text: Color32,
    pub icon_well: Color32,
    pub modal_fill: Color32,
    pub modal_text: Color32,
    /// Kept for contrast tests. Cancel is drawn as an outline, not a fill.
    #[allow(dead_code)]
    pub cancel_fill: Color32,
    pub cancel_text: Color32,
    pub log_inner: Color32,
    pub log_text: Color32,
    pub surface_hover: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        let accent_dim = Color32::from_rgba_unmultiplied(0x5E, 0xEA, 0xD4, 36);
        let border = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        let border_strong = Color32::from_rgba_unmultiplied(255, 255, 255, 31);

        Self {
            mode: ThemeMode::Dark,
            void: VOID_DARK,
            surface: SURFACE_DARK,
            elevated: ELEVATED_DARK,
            border,
            border_strong,
            text: TEXT_DARK,
            muted: MUTED_DARK,
            accent: ACCENT_DARK,
            accent_dim,
            accent_bright: ACCENT_DARK,
            accent_on: ACCENT_ON_DARK,
            danger: DANGER,
            tile: SURFACE_DARK,
            tile_selected: mix(SURFACE_DARK, ACCENT_DARK, 0.14),
            cta_fill: ACCENT_DARK,
            cta_text: ACCENT_ON_DARK,
            widget_bg: ELEVATED_DARK,
            widget_hover: mix(ELEVATED_DARK, ACCENT_DARK, 0.10),
            input_fill: SURFACE_DARK,
            input_border: border,
            nav_hover: mix(VOID_DARK, ELEVATED_DARK, 0.55),
            filter_selected_fill: ACCENT_DARK,
            filter_selected_text: ACCENT_ON_DARK,
            icon_well: ELEVATED_DARK,
            modal_fill: ELEVATED_DARK,
            modal_text: TEXT_DARK,
            cancel_fill: SURFACE_DARK,
            cancel_text: TEXT_DARK,
            log_inner: VOID_DARK,
            log_text: TEXT_DARK,
            surface_hover: mix(SURFACE_DARK, ELEVATED_DARK, 0.65),
        }
    }

    pub fn light() -> Self {
        let border = Color32::from_rgba_unmultiplied(
            BORDER_LIGHT_BASE.r(),
            BORDER_LIGHT_BASE.g(),
            BORDER_LIGHT_BASE.b(),
            20,
        );
        let border_strong = Color32::from_rgba_unmultiplied(
            BORDER_LIGHT_BASE.r(),
            BORDER_LIGHT_BASE.g(),
            BORDER_LIGHT_BASE.b(),
            36,
        );
        let accent_dim = Color32::from_rgba_unmultiplied(0x0D, 0x94, 0x88, 28);

        Self {
            mode: ThemeMode::Light,
            void: VOID_LIGHT,
            surface: SURFACE_LIGHT,
            elevated: ELEVATED_LIGHT,
            border,
            border_strong,
            text: TEXT_LIGHT,
            muted: MUTED_LIGHT,
            accent: ACCENT_LIGHT,
            accent_dim,
            accent_bright: ACCENT_BRIGHT,
            accent_on: ACCENT_ON_LIGHT,
            danger: DANGER,
            tile: SURFACE_LIGHT,
            tile_selected: mix(SURFACE_LIGHT, ACCENT_LIGHT, 0.10),
            cta_fill: ACCENT_BRIGHT,
            cta_text: ACCENT_ON_LIGHT,
            widget_bg: ELEVATED_LIGHT,
            widget_hover: mix(ELEVATED_LIGHT, ACCENT_LIGHT, 0.12),
            input_fill: SURFACE_LIGHT,
            input_border: border,
            nav_hover: mix(VOID_LIGHT, TEXT_LIGHT, 0.06),
            filter_selected_fill: ACCENT_BRIGHT,
            filter_selected_text: ACCENT_ON_LIGHT,
            icon_well: ELEVATED_LIGHT,
            modal_fill: SURFACE_LIGHT,
            modal_text: TEXT_LIGHT,
            cancel_fill: ELEVATED_LIGHT,
            cancel_text: TEXT_LIGHT,
            log_inner: ELEVATED_LIGHT,
            log_text: TEXT_LIGHT,
            surface_hover: mix(SURFACE_LIGHT, TEXT_LIGHT, 0.06),
        }
    }

    /// Combined sun + right-facing crescent. Same glyph in both themes.
    pub fn paint_toggle_icon(&self, painter: &Painter, rect: Rect, punch: Color32) {
        paint_theme_toggle(painter, rect, self.text, punch);
    }
}

/// ViewBox 0 0 24 24 geometry for the combined theme glyph.
const TOGGLE_VIEWBOX: f32 = 24.0;
const SUN_DISK: (f32, f32, f32) = (12.0, 12.0, 7.8);
const CRESCENT_A: (f32, f32, f32) = (12.0, 12.4, 5.4);
const CRESCENT_B: (f32, f32, f32) = (14.0, 10.4, 4.5);
const RAY_THICKNESS: f32 = 2.0;
const RAY_LENGTH: f32 = 4.2;
const RAY_ROUNDING: f32 = 0.15;

#[cfg(test)]
fn sun_ray_angles() -> [f32; 8] {
    core::array::from_fn(|i| (i as f32) * std::f32::consts::TAU / 8.0)
}

/// Axis-aligned ray rects in viewBox space (the other four are these rotated 45°).
fn cardinal_ray_rects() -> [(f32, f32, f32, f32); 4] {
    [
        (11.0, 0.0, RAY_THICKNESS, RAY_LENGTH),
        (11.0, TOGGLE_VIEWBOX - RAY_LENGTH, RAY_THICKNESS, RAY_LENGTH),
        (0.0, 11.0, RAY_LENGTH, RAY_THICKNESS),
        (TOGGLE_VIEWBOX - RAY_LENGTH, 11.0, RAY_LENGTH, RAY_THICKNESS),
    ]
}

fn map_viewbox(x: f32, y: f32, rect: Rect) -> Pos2 {
    let size = rect.width().min(rect.height());
    let scale = size / TOGGLE_VIEWBOX;
    let origin = rect.center() - vec2(12.0 * scale, 12.0 * scale);
    origin + vec2(x * scale, y * scale)
}

fn viewbox_scale(rect: Rect) -> f32 {
    rect.width().min(rect.height()) / TOGGLE_VIEWBOX
}

fn rotate_around(point: Pos2, origin: Pos2, angle: f32) -> Pos2 {
    let delta = point - origin;
    let (sin, cos) = angle.sin_cos();
    origin + vec2(delta.x * cos - delta.y * sin, delta.x * sin + delta.y * cos)
}

fn fill_viewbox_rect(
    painter: &Painter,
    rect: Rect,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color32,
) {
    let min = map_viewbox(x, y, rect);
    let max = map_viewbox(x + w, y + h, rect);
    let rounding = RAY_ROUNDING * viewbox_scale(rect);
    painter.rect_filled(Rect::from_min_max(min, max), rounding, color);
}

fn fill_rotated_viewbox_rect(
    painter: &Painter,
    rect: Rect,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color32,
) {
    let origin = map_viewbox(12.0, 12.0, rect);
    let corners = [
        map_viewbox(x, y, rect),
        map_viewbox(x + w, y, rect),
        map_viewbox(x + w, y + h, rect),
        map_viewbox(x, y + h, rect),
    ]
    .map(|point| rotate_around(point, origin, 45.0_f32.to_radians()));
    painter.add(Shape::convex_polygon(corners.to_vec(), color, Stroke::NONE));
}

/// Always draws the same filled sun (8 rectangular rays + right-facing crescent cutout).
/// `punch` is the rail/hover fill used to cut the crescent out of the disk.
pub fn paint_theme_toggle(painter: &Painter, rect: Rect, chrome_text: Color32, punch: Color32) {
    for (x, y, w, h) in cardinal_ray_rects() {
        fill_viewbox_rect(painter, rect, x, y, w, h, chrome_text);
        fill_rotated_viewbox_rect(painter, rect, x, y, w, h, chrome_text);
    }

    let scale = viewbox_scale(rect);
    let disk = map_viewbox(SUN_DISK.0, SUN_DISK.1, rect);
    painter.circle_filled(disk, SUN_DISK.2 * scale, chrome_text);

    let hole_a = map_viewbox(CRESCENT_A.0, CRESCENT_A.1, rect);
    painter.circle_filled(hole_a, CRESCENT_A.2 * scale, punch);
    let hole_b = map_viewbox(CRESCENT_B.0, CRESCENT_B.1, rect);
    painter.circle_filled(hole_b, CRESCENT_B.2 * scale, chrome_text);
}

pub fn apply_theme(ctx: &Context, mode: ThemeMode) {
    let palette = mode.palette();
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(13.0, 7.0);
    style.spacing.interact_size = Vec2::new(36.0, 32.0);
    style.visuals = match mode {
        ThemeMode::Dark => Visuals::dark(),
        ThemeMode::Light => Visuals::light(),
    };
    style.visuals.window_fill = palette.modal_fill;
    style.visuals.panel_fill = palette.void;
    style.visuals.extreme_bg_color = palette.input_fill;
    style.visuals.faint_bg_color = palette.surface;
    style.visuals.override_text_color = Some(palette.text);
    style.visuals.window_stroke = Stroke::new(1.0_f32, palette.border_strong);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.text);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.widgets.inactive.bg_fill = palette.widget_bg;
    style.visuals.widgets.inactive.weak_bg_fill = palette.widget_bg;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.text);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.widgets.hovered.bg_fill = palette.widget_hover;
    style.visuals.widgets.hovered.weak_bg_fill = palette.widget_hover;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.text);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.widgets.active.bg_fill = palette.widget_hover;
    style.visuals.widgets.active.weak_bg_fill = palette.widget_hover;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, palette.text);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.selection.bg_fill = palette.widget_hover;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, palette.text);
    style.visuals.hyperlink_color = palette.accent;
    ctx.set_global_style(style);
}

/// Config directory: `$XDG_CONFIG_HOME/linux-it-guy-toolbox` or `~/.config/...`.
/// Relative `XDG_CONFIG_HOME` / `HOME` values are ignored.
pub fn config_dir() -> Option<PathBuf> {
    config_dir_from_env(
        std::env::var_os("XDG_CONFIG_HOME")
            .as_deref()
            .map(Path::new),
        std::env::var_os("HOME").as_deref().map(Path::new),
    )
}

pub fn config_dir_from_env(xdg_config_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    if let Some(xdg) = xdg_config_home {
        if xdg.is_absolute() {
            return Some(xdg.join(CONFIG_DIR_NAME));
        }
        return None;
    }
    let home = home?;
    home.is_absolute()
        .then(|| home.join(".config").join(CONFIG_DIR_NAME))
}

pub fn theme_file_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join(THEME_FILE_NAME))
}

pub fn load_theme_mode() -> ThemeMode {
    theme_file_path()
        .and_then(|path| load_theme_mode_from(&path))
        .unwrap_or_default()
}

pub fn load_theme_mode_from(path: &Path) -> Option<ThemeMode> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| ThemeMode::parse(&contents))
}

pub fn save_theme_mode(mode: ThemeMode) -> std::io::Result<PathBuf> {
    let dir = config_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "no usable config directory")
    })?;
    fs::create_dir_all(&dir)?;
    let path = dir.join(THEME_FILE_NAME);
    save_theme_mode_to(&path, mode)?;
    Ok(path)
}

pub fn save_theme_mode_to(path: &Path, mode: ThemeMode) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", mode.as_str()))
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let inv = 1.0 - t;
    Color32::from_rgb(
        (f32::from(a.r()) * inv + f32::from(b.r()) * t).round() as u8,
        (f32::from(a.g()) * inv + f32::from(b.g()) * t).round() as u8,
        (f32::from(a.b()) * inv + f32::from(b.b()) * t).round() as u8,
    )
}

#[cfg(test)]
fn relative_luminance(color: Color32) -> f32 {
    fn lin(channel: u8) -> f32 {
        let value = f32::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * lin(color.r()) + 0.7152 * lin(color.g()) + 0.0722 * lin(color.b())
}

#[cfg(test)]
fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (left, right) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if left > right {
        (left, right)
    } else {
        (right, left)
    };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
fn hex(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn production_theme() -> &'static str {
        include_str!("theme.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("theme module")
    }

    #[test]
    fn lumen_hex_values_are_exact() {
        assert_eq!(hex(VOID_DARK), "#07080A");
        assert_eq!(hex(SURFACE_DARK), "#111318");
        assert_eq!(hex(ELEVATED_DARK), "#1A1D26");
        assert_eq!(hex(TEXT_DARK), "#F4F5F7");
        assert_eq!(hex(MUTED_DARK), "#8B92A5");
        assert_eq!(hex(ACCENT_DARK), "#5EEAD4");
        assert_eq!(hex(ACCENT_ON_DARK), "#07080A");
        assert_eq!(hex(DANGER), "#FF6B6B");
        assert_eq!(hex(VOID_LIGHT), "#F0F2F5");
        assert_eq!(hex(SURFACE_LIGHT), "#FFFFFF");
        assert_eq!(hex(ELEVATED_LIGHT), "#F7F8FA");
        assert_eq!(hex(TEXT_LIGHT), "#0B1220");
        assert_eq!(hex(MUTED_LIGHT), "#64748B");
        assert_eq!(hex(ACCENT_LIGHT), "#0D9488");
        assert_eq!(hex(ACCENT_BRIGHT), "#2DD4BF");
        assert_eq!(hex(ACCENT_ON_LIGHT), "#042F2E");
        assert_eq!(hex(BORDER_LIGHT_BASE), "#0F172A");
    }

    #[test]
    fn retired_brand_hexes_are_gone_from_chrome() {
        let theme = production_theme();
        for retired in [
            "#E9FC12", "E9FC12", "#1A365D", "1A365D", "#CDEDFE", "CDEDFE",
        ] {
            assert!(
                !theme.contains(retired),
                "retired brand token {retired} must not appear in Lumen chrome"
            );
        }
        assert!(!theme.contains("NAVY"));
        assert!(!theme.contains("PALE_SKY"));
        assert!(!theme.contains("neon yellow"));
    }

    #[test]
    fn default_theme_is_lumen_dark() {
        assert_eq!(ThemeMode::default(), ThemeMode::Dark);
        let palette = Palette::dark();
        assert_eq!(palette.void, VOID_DARK);
        assert_eq!(palette.surface, SURFACE_DARK);
        assert_eq!(palette.elevated, ELEVATED_DARK);
        assert_eq!(palette.text, TEXT_DARK);
        assert_eq!(palette.muted, MUTED_DARK);
        assert_eq!(palette.accent, ACCENT_DARK);
        assert_eq!(palette.accent_on, ACCENT_ON_DARK);
        assert_eq!(palette.cta_fill, ACCENT_DARK);
        assert_eq!(palette.cta_text, ACCENT_ON_DARK);
        assert_eq!(palette.filter_selected_fill, ACCENT_DARK);
        assert_eq!(palette.filter_selected_text, ACCENT_ON_DARK);
        assert_eq!(palette.tile, SURFACE_DARK);
        assert_eq!(palette.tile_selected, mix(SURFACE_DARK, ACCENT_DARK, 0.14));
        assert_eq!(palette.icon_well, ELEVATED_DARK);
        assert_eq!(palette.danger, DANGER);
        assert_eq!(palette.border.a(), 18);
        assert_eq!(palette.border_strong.a(), 31);
        assert_eq!(palette.accent_dim.a(), 36);
    }

    #[test]
    fn light_theme_uses_lumen_light_tokens() {
        let palette = Palette::light();
        assert_eq!(palette.void, VOID_LIGHT);
        assert_eq!(palette.surface, SURFACE_LIGHT);
        assert_eq!(palette.elevated, ELEVATED_LIGHT);
        assert_eq!(palette.text, TEXT_LIGHT);
        assert_eq!(palette.muted, MUTED_LIGHT);
        assert_eq!(palette.accent, ACCENT_LIGHT);
        assert_eq!(palette.accent_bright, ACCENT_BRIGHT);
        assert_eq!(palette.accent_on, ACCENT_ON_LIGHT);
        assert_eq!(palette.cta_fill, ACCENT_BRIGHT);
        assert_eq!(palette.cta_text, ACCENT_ON_LIGHT);
        assert_eq!(palette.filter_selected_fill, ACCENT_BRIGHT);
        assert_eq!(palette.filter_selected_text, ACCENT_ON_LIGHT);
        assert_eq!(palette.tile, SURFACE_LIGHT);
        assert_eq!(palette.icon_well, ELEVATED_LIGHT);
        assert_eq!(
            palette.border,
            Color32::from_rgba_unmultiplied(
                BORDER_LIGHT_BASE.r(),
                BORDER_LIGHT_BASE.g(),
                BORDER_LIGHT_BASE.b(),
                20
            )
        );
        assert_eq!(palette.border.a(), 20);
    }

    #[test]
    fn text_pairs_meet_aa_contrast() {
        for palette in [Palette::dark(), Palette::light()] {
            assert!(
                contrast_ratio(palette.text, palette.void) >= 4.5,
                "{:?} text on void contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.text, palette.surface) >= 4.5,
                "{:?} text on surface contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.text, palette.tile) >= 4.5,
                "{:?} tile text contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.text, palette.tile_selected) >= 4.5,
                "{:?} selected tile text contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.cta_text, palette.cta_fill) >= 4.5,
                "{:?} CTA contrast {}",
                palette.mode,
                contrast_ratio(palette.cta_text, palette.cta_fill)
            );
            assert!(
                contrast_ratio(palette.log_text, palette.log_inner) >= 4.5,
                "{:?} log contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.filter_selected_text, palette.filter_selected_fill) >= 4.5,
                "{:?} filter chip contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.modal_text, palette.modal_fill) >= 4.5,
                "{:?} modal text contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.cancel_text, palette.cancel_fill) >= 4.5,
                "{:?} cancel button contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.muted, palette.surface) >= 3.0,
                "{:?} muted on surface",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.text, palette.icon_well) >= 3.0,
                "{:?} letter fallback on icon well",
                palette.mode
            );
            assert_ne!(
                palette.icon_well, palette.tile,
                "{:?} icon well must step off the tile",
                palette.mode
            );
            assert_ne!(palette.tile, palette.void);
            assert_ne!(palette.border, palette.surface);
        }
    }

    #[test]
    fn toggle_cycles_and_names_parse() {
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Dark);
        assert_eq!(ThemeMode::parse("dark"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::parse("LIGHT\n"), Some(ThemeMode::Light));
        assert_eq!(ThemeMode::parse("nope"), None);
        assert_eq!(ThemeMode::Dark.toggle_tooltip(), "Switch to light theme");
        assert_eq!(ThemeMode::Light.toggle_tooltip(), "Switch to dark theme");
    }

    #[test]
    fn theme_icons_are_not_shipped_as_rasters() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(
            !root.join("assets/theme/sun-dark-mode.png").exists(),
            "sun PNG must not be shipped"
        );
        assert!(
            !root.join("assets/theme/moon-light-mode.png").exists(),
            "moon PNG must not be shipped"
        );
        assert!(
            !root.join("assets/theme").exists(),
            "assets/theme rasters must not be shipped"
        );
        let prod_theme = production_theme();
        let prod_main = include_str!("main.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("main module");
        assert!(!prod_theme.contains("include_bytes!"));
        assert!(!prod_theme.contains("paint_sun"));
        assert!(!prod_theme.contains("paint_moon"));
        assert!(!prod_theme.contains("ThemeIcon"));
        assert!(!prod_theme.contains("fn toggle_icon"));
        assert!(prod_theme.contains("paint_theme_toggle"));
        assert!(!prod_main.contains("assets/theme/"));
        assert!(!prod_main.contains("paint_sun"));
        assert!(!prod_main.contains("paint_moon"));
        let apply = include_str!("theme.rs")
            .split("pub fn apply_theme")
            .nth(1)
            .and_then(|rest| rest.split("pub fn config_dir").next())
            .expect("apply_theme body");
        assert!(
            !apply.contains("cta_fill"),
            "active widgets must not use yellow CTA fill"
        );
        assert!(apply.contains("widget_hover"));
    }

    #[test]
    fn theme_toggle_has_eight_rays_at_45_degrees() {
        let angles = sun_ray_angles();
        assert_eq!(angles.len(), 8);
        for (i, angle) in angles.iter().enumerate() {
            let expected = (i as f32) * 45.0_f32.to_radians();
            assert!((angle - expected).abs() < 1.0e-5);
        }
        assert_eq!(cardinal_ray_rects().len(), 4);
    }

    #[test]
    fn theme_toggle_crescent_horns_face_right() {
        assert_eq!(SUN_DISK, (12.0, 12.0, 7.8));
        assert_eq!(CRESCENT_A, (12.0, 12.4, 5.4));
        assert_eq!(CRESCENT_B, (14.0, 10.4, 4.5));
        assert!(
            CRESCENT_B.0 > CRESCENT_A.0,
            "subtracted circle must sit to the right so horns face right"
        );
        let rect = Rect::from_center_size(pos2(40.0, 40.0), vec2(26.0, 26.0));
        let hole_a = map_viewbox(CRESCENT_A.0, CRESCENT_A.1, rect);
        let hole_b = map_viewbox(CRESCENT_B.0, CRESCENT_B.1, rect);
        assert!(hole_b.x > hole_a.x);
        assert!(hole_b.x > rect.center().x);
    }

    #[test]
    fn config_dir_requires_absolute_paths() {
        let xdg = Path::new("/tmp/xdg-config");
        assert_eq!(
            config_dir_from_env(Some(xdg), None).as_deref(),
            Some(Path::new("/tmp/xdg-config/linux-it-guy-toolbox"))
        );
        assert_eq!(
            config_dir_from_env(None, Some(Path::new("/home/chris"))).as_deref(),
            Some(Path::new("/home/chris/.config/linux-it-guy-toolbox"))
        );
        assert_eq!(config_dir_from_env(Some(Path::new("relative")), None), None);
        assert_eq!(config_dir_from_env(None, Some(Path::new("rel-home"))), None);
        assert_eq!(config_dir_from_env(None, None), None);
    }

    #[test]
    fn theme_file_round_trips() {
        let dir = std::env::temp_dir().join(format!(
            "linux-it-guy-toolbox-theme-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let path = dir.join("theme");
        save_theme_mode_to(&path, ThemeMode::Light).unwrap();
        assert_eq!(load_theme_mode_from(&path), Some(ThemeMode::Light));
        save_theme_mode_to(&path, ThemeMode::Dark).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "dark\n");
        assert_eq!(load_theme_mode_from(&path), Some(ThemeMode::Dark));
        fs::write(&path, "bogus\n").unwrap();
        assert_eq!(load_theme_mode_from(&path), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
