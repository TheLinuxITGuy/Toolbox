//! The Linux IT Guy brand themes.
//!
//! Colors come from the brand board (Colours):
//! - Primary / Secondary: `#CDEDFE` pale sky blue
//! - Accent: `#E9FC12` neon yellow
//! - Background: `#1A365D` navy
//!
//! Dark: navy canvas, pale-blue text and surfaces, yellow accents/CTAs.
//! Light: pale-blue canvas, navy text and surfaces, yellow accents/CTAs.
//!
//! Yellow is never used as body text on pale-blue fills (poor contrast). CTA
//! labels are navy on yellow. On pale-blue fills, yellow is borders and icons
//! only.

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{Color32, Context, Stroke, Vec2, Visuals};

pub const PALE_SKY: Color32 = Color32::from_rgb(0xCD, 0xED, 0xFE);
pub const ACCENT: Color32 = Color32::from_rgb(0xE9, 0xFC, 0x12);
pub const NAVY: Color32 = Color32::from_rgb(0x1A, 0x36, 0x5D);

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

    /// Icon shown on the toggle: sun means "switch to light", moon means "switch to dark".
    pub fn toggle_icon(self) -> ThemeIcon {
        match self {
            Self::Dark => ThemeIcon::Sun,
            Self::Light => ThemeIcon::Moon,
        }
    }

    pub fn toggle_tooltip(self) -> &'static str {
        match self {
            Self::Dark => "Switch to light theme",
            Self::Light => "Switch to dark theme",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThemeIcon {
    Sun,
    Moon,
}

/// Derived chrome colors for one theme. Brand primaries stay exact hex values;
/// mixes are only used for hover, depth, and borders.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub mode: ThemeMode,
    pub background: Color32,
    pub sidebar: Color32,
    pub chrome_text: Color32,
    pub chrome_subtle: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_selected: Color32,
    pub on_surface: Color32,
    pub on_surface_subtle: Color32,
    pub toolbar: Color32,
    pub summary: Color32,
    pub log_frame: Color32,
    pub log_inner: Color32,
    pub log_text: Color32,
    pub border: Color32,
    pub accent: Color32,
    pub cta_fill: Color32,
    pub cta_text: Color32,
    pub widget_bg: Color32,
    pub widget_hover: Color32,
    pub input_fill: Color32,
    pub input_border: Color32,
    pub checkbox_empty: Color32,
    pub checkbox_border: Color32,
    pub checkbox_check: Color32,
    pub nav_selected: Color32,
    pub nav_hover: Color32,
    pub nav_marker: Color32,
    pub chip_selected: Color32,
    pub chip_idle: Color32,
    pub chip_idle_text: Color32,
    pub chip_selected_text: Color32,
    pub filter_selected_fill: Color32,
    pub filter_selected_text: Color32,
    pub filter_idle_text: Color32,
    pub count_badge: Color32,
    pub count_badge_text: Color32,
    pub page_icon_fill: Color32,
    pub icon_well: Color32,
    pub badge_native_fill: Color32,
    pub badge_native_stroke: Color32,
    pub badge_native_text: Color32,
    pub badge_flatpak_fill: Color32,
    pub badge_flatpak_stroke: Color32,
    pub badge_flatpak_text: Color32,
    pub modal_fill: Color32,
    pub modal_text: Color32,
    pub cancel_fill: Color32,
    pub cancel_text: Color32,
    pub theme_icon: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        // Navy chrome, pale-blue surfaces and text, yellow CTAs.
        let surface = mix(PALE_SKY, NAVY, 0.10);
        let surface_hover = mix(PALE_SKY, NAVY, 0.16);
        let surface_selected = mix(PALE_SKY, NAVY, 0.20);
        let sidebar = mix(NAVY, Color32::BLACK, 0.14);
        let chrome_subtle = mix(PALE_SKY, NAVY, 0.28);

        Self {
            mode: ThemeMode::Dark,
            background: NAVY,
            sidebar,
            chrome_text: PALE_SKY,
            chrome_subtle,
            surface,
            surface_hover,
            surface_selected,
            on_surface: NAVY,
            on_surface_subtle: mix(NAVY, PALE_SKY, 0.28),
            toolbar: surface,
            summary: surface,
            log_frame: surface,
            log_inner: mix(NAVY, Color32::BLACK, 0.28),
            log_text: PALE_SKY,
            border: mix(NAVY, PALE_SKY, 0.32),
            accent: ACCENT,
            cta_fill: ACCENT,
            cta_text: NAVY,
            widget_bg: mix(PALE_SKY, NAVY, 0.08),
            widget_hover: mix(PALE_SKY, NAVY, 0.18),
            input_fill: mix(PALE_SKY, Color32::WHITE, 0.22),
            input_border: mix(NAVY, PALE_SKY, 0.40),
            checkbox_empty: mix(PALE_SKY, Color32::WHITE, 0.18),
            checkbox_border: mix(NAVY, PALE_SKY, 0.35),
            checkbox_check: NAVY,
            nav_selected: mix(NAVY, PALE_SKY, 0.16),
            nav_hover: mix(NAVY, PALE_SKY, 0.10),
            nav_marker: ACCENT,
            chip_selected: surface,
            chip_idle: mix(NAVY, PALE_SKY, 0.10),
            chip_idle_text: PALE_SKY,
            chip_selected_text: NAVY,
            filter_selected_fill: ACCENT,
            filter_selected_text: NAVY,
            filter_idle_text: NAVY,
            count_badge: mix(NAVY, PALE_SKY, 0.18),
            count_badge_text: PALE_SKY,
            page_icon_fill: mix(NAVY, PALE_SKY, 0.20),
            icon_well: mix(PALE_SKY, NAVY, 0.08),
            badge_native_fill: mix(PALE_SKY, NAVY, 0.08),
            badge_native_stroke: ACCENT,
            badge_native_text: NAVY,
            badge_flatpak_fill: mix(PALE_SKY, NAVY, 0.14),
            badge_flatpak_stroke: mix(NAVY, PALE_SKY, 0.20),
            badge_flatpak_text: NAVY,
            modal_fill: mix(PALE_SKY, NAVY, 0.06),
            modal_text: NAVY,
            cancel_fill: mix(PALE_SKY, Color32::WHITE, 0.35),
            cancel_text: NAVY,
            theme_icon: ACCENT,
        }
    }

    pub fn light() -> Self {
        // Pale-blue chrome, navy surfaces and text, yellow CTAs.
        let surface = mix(NAVY, PALE_SKY, 0.08);
        let surface_hover = mix(NAVY, PALE_SKY, 0.16);
        let surface_selected = mix(NAVY, PALE_SKY, 0.20);
        let sidebar = mix(PALE_SKY, Color32::WHITE, 0.16);
        let chrome_subtle = mix(NAVY, PALE_SKY, 0.32);

        Self {
            mode: ThemeMode::Light,
            background: PALE_SKY,
            sidebar,
            chrome_text: NAVY,
            chrome_subtle,
            surface,
            surface_hover,
            surface_selected,
            on_surface: PALE_SKY,
            on_surface_subtle: mix(PALE_SKY, NAVY, 0.22),
            toolbar: surface,
            summary: surface,
            log_frame: surface,
            log_inner: mix(NAVY, Color32::BLACK, 0.18),
            log_text: PALE_SKY,
            border: mix(PALE_SKY, NAVY, 0.26),
            accent: ACCENT,
            cta_fill: ACCENT,
            cta_text: NAVY,
            widget_bg: mix(NAVY, PALE_SKY, 0.12),
            widget_hover: mix(NAVY, PALE_SKY, 0.22),
            input_fill: mix(NAVY, PALE_SKY, 0.14),
            input_border: mix(PALE_SKY, NAVY, 0.28),
            checkbox_empty: mix(NAVY, PALE_SKY, 0.16),
            checkbox_border: mix(PALE_SKY, NAVY, 0.28),
            checkbox_check: NAVY,
            nav_selected: mix(PALE_SKY, NAVY, 0.12),
            nav_hover: mix(PALE_SKY, NAVY, 0.08),
            nav_marker: ACCENT,
            chip_selected: surface,
            chip_idle: mix(PALE_SKY, NAVY, 0.08),
            chip_idle_text: NAVY,
            chip_selected_text: PALE_SKY,
            filter_selected_fill: ACCENT,
            filter_selected_text: NAVY,
            filter_idle_text: PALE_SKY,
            count_badge: mix(PALE_SKY, NAVY, 0.14),
            count_badge_text: NAVY,
            page_icon_fill: mix(NAVY, PALE_SKY, 0.12),
            icon_well: mix(NAVY, PALE_SKY, 0.10),
            badge_native_fill: mix(NAVY, PALE_SKY, 0.10),
            badge_native_stroke: ACCENT,
            badge_native_text: PALE_SKY,
            badge_flatpak_fill: mix(NAVY, PALE_SKY, 0.16),
            badge_flatpak_stroke: mix(PALE_SKY, NAVY, 0.20),
            badge_flatpak_text: PALE_SKY,
            modal_fill: mix(PALE_SKY, Color32::WHITE, 0.35),
            modal_text: NAVY,
            cancel_fill: mix(PALE_SKY, Color32::WHITE, 0.55),
            cancel_text: NAVY,
            theme_icon: NAVY,
        }
    }
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
    style.visuals.panel_fill = palette.background;
    style.visuals.extreme_bg_color = palette.input_fill;
    style.visuals.faint_bg_color = palette.surface;
    style.visuals.override_text_color = Some(palette.chrome_text);
    style.visuals.window_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, palette.chrome_text);
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.widgets.inactive.bg_fill = palette.widget_bg;
    style.visuals.widgets.inactive.weak_bg_fill = palette.widget_bg;
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, palette.chrome_text);
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, palette.border);
    style.visuals.widgets.hovered.bg_fill = palette.widget_hover;
    style.visuals.widgets.hovered.weak_bg_fill = palette.widget_hover;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, palette.chrome_text);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette.accent);
    style.visuals.widgets.active.bg_fill = palette.cta_fill;
    style.visuals.widgets.active.weak_bg_fill = palette.cta_fill;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, palette.cta_text);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, palette.cta_fill);
    style.visuals.selection.bg_fill = palette.cta_fill;
    style.visuals.selection.stroke = Stroke::new(1.0_f32, palette.cta_text);
    style.visuals.hyperlink_color = palette.chrome_text;
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

/// Yellow-on-pale-sky is below AA for body text; callers must not use that pair.
#[cfg(test)]
fn yellow_on_pale_sky_is_poor_contrast() -> bool {
    contrast_ratio(ACCENT, PALE_SKY) < 3.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_hex_values_are_exact() {
        assert_eq!(
            format!(
                "#{:02X}{:02X}{:02X}",
                PALE_SKY.r(),
                PALE_SKY.g(),
                PALE_SKY.b()
            ),
            "#CDEDFE"
        );
        assert_eq!(
            format!("#{:02X}{:02X}{:02X}", ACCENT.r(), ACCENT.g(), ACCENT.b()),
            "#E9FC12"
        );
        assert_eq!(
            format!("#{:02X}{:02X}{:02X}", NAVY.r(), NAVY.g(), NAVY.b()),
            "#1A365D"
        );
        assert_eq!(PALE_SKY, Color32::from_rgb(205, 237, 254));
        assert_eq!(ACCENT, Color32::from_rgb(233, 252, 18));
        assert_eq!(NAVY, Color32::from_rgb(26, 54, 93));
    }

    #[test]
    fn dark_uses_navy_canvas_pale_text_yellow_cta() {
        let palette = Palette::dark();
        assert_eq!(palette.background, NAVY);
        assert_eq!(palette.chrome_text, PALE_SKY);
        assert_eq!(palette.on_surface, NAVY);
        assert_eq!(palette.accent, ACCENT);
        assert_eq!(palette.cta_fill, ACCENT);
        assert_eq!(palette.cta_text, NAVY);
        assert_eq!(palette.filter_selected_fill, ACCENT);
        assert_eq!(palette.filter_selected_text, NAVY);
        assert_ne!(palette.surface, NAVY);
        assert!(contrast_ratio(palette.surface, NAVY) > 2.0);
    }

    #[test]
    fn light_uses_pale_canvas_navy_text_yellow_cta() {
        let palette = Palette::light();
        assert_eq!(palette.background, PALE_SKY);
        assert_eq!(palette.chrome_text, NAVY);
        assert_eq!(palette.on_surface, PALE_SKY);
        assert_eq!(palette.accent, ACCENT);
        assert_eq!(palette.cta_fill, ACCENT);
        assert_eq!(palette.cta_text, NAVY);
        assert_eq!(palette.filter_selected_fill, ACCENT);
        assert_eq!(palette.filter_selected_text, NAVY);
        assert_ne!(palette.surface, PALE_SKY);
        assert!(contrast_ratio(palette.surface, PALE_SKY) > 2.0);
    }

    #[test]
    fn text_pairs_meet_aa_contrast() {
        for palette in [Palette::dark(), Palette::light()] {
            assert!(
                contrast_ratio(palette.chrome_text, palette.background) >= 4.5,
                "{:?} chrome text contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.on_surface, palette.surface) >= 4.5,
                "{:?} surface text contrast",
                palette.mode
            );
            assert!(
                contrast_ratio(palette.cta_text, palette.cta_fill) >= 4.5,
                "{:?} CTA contrast",
                palette.mode
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
        }
    }

    #[test]
    fn yellow_is_not_used_as_text_on_pale_sky() {
        assert!(yellow_on_pale_sky_is_poor_contrast());
        for palette in [Palette::dark(), Palette::light()] {
            if palette.background == PALE_SKY {
                assert_ne!(palette.chrome_text, ACCENT);
            }
            if contrast_ratio(palette.surface, PALE_SKY) < 1.4 {
                assert_ne!(palette.on_surface, ACCENT);
            }
            assert_ne!(palette.cta_text, ACCENT);
            assert_ne!(palette.filter_selected_text, ACCENT);
        }
    }

    #[test]
    fn toggle_cycles_and_names_parse() {
        assert_eq!(ThemeMode::Dark.toggle(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.toggle(), ThemeMode::Dark);
        assert_eq!(ThemeMode::parse("dark"), Some(ThemeMode::Dark));
        assert_eq!(ThemeMode::parse("LIGHT\n"), Some(ThemeMode::Light));
        assert_eq!(ThemeMode::parse("nope"), None);
        assert_eq!(ThemeMode::Dark.toggle_icon(), ThemeIcon::Sun);
        assert_eq!(ThemeMode::Light.toggle_icon(), ThemeIcon::Moon);
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
