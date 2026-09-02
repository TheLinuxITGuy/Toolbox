mod catalog;
mod logos;
mod runner;
mod system;
mod theme;
mod validate;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use eframe::egui::{
    self, Align, Align2, Button, CentralPanel, Color32, Context, FontData, FontDefinitions,
    FontFamily, FontId, Frame, Grid, Key, Layout, Painter, Panel, Pos2, Rect, RichText, ScrollArea,
    Sense, Stroke, StrokeKind, TextEdit, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2,
};

use catalog::{AdminTask, AppEntry, CatalogLoad, Task, admin_tasks, find_base_dir, load_apps};
use runner::{RunnerMessage, run_tasks};
use system::{
    command_output, detect_package_manager, distro_name, env_or_unknown, strip_ansi, uptime,
};
use theme::{Palette, ThemeIcon, ThemeMode};
use validate::zeroize_string;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 800.0])
            .with_min_inner_size([940.0, 660.0])
            .with_maximized(true),
        ..Default::default()
    };

    eframe::run_native(
        "The Linux IT Guy Toolbox",
        options,
        Box::new(|cc| Ok(Box::new(ToolboxApp::new(cc)))),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Page {
    Install,
    Remove,
    Administration,
    SystemInfo,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Page::Install => "Install",
            Page::Remove => "Remove",
            Page::Administration => "Administration",
            Page::SystemInfo => "System Info",
        }
    }
}

struct ToolboxApp {
    base_dir: PathBuf,
    page: Page,
    apps: Vec<AppEntry>,
    admin_tasks: Vec<AdminTask>,
    install_selected: HashSet<usize>,
    remove_selected: HashSet<usize>,
    admin_selected: HashSet<usize>,
    search: String,
    category_filter: Option<String>,
    distro_name: String,
    package_manager: String,
    log: String,
    password: String,
    show_password_modal: bool,
    is_running: bool,
    tx: Option<Sender<RunnerMessage>>,
    rx: Option<Receiver<RunnerMessage>>,
    log_revision: u64,
    icons: HashMap<&'static str, TextureHandle>,
    distro_icons: HashMap<&'static str, TextureHandle>,
    theme_icons: HashMap<&'static str, TextureHandle>,
    theme: ThemeMode,
}

impl ToolboxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let theme = theme::load_theme_mode();
        theme::apply_theme(&cc.egui_ctx, theme);
        let icons = load_icons(&cc.egui_ctx);
        let distro_icons = load_distro_icons(&cc.egui_ctx);
        let theme_icons = load_theme_icons(&cc.egui_ctx);

        let discovery = find_base_dir();
        let catalog = if discovery.found {
            load_apps(&discovery.base_dir)
        } else {
            CatalogLoad {
                error: Some(discovery.not_found_message()),
                ..CatalogLoad::default()
            }
        };
        let base_dir = discovery.base_dir;
        let distro_name = distro_name();
        let package_manager = detect_package_manager();

        let mut log = "Process logs will appear here...".to_owned();
        if let Some(error) = &catalog.error {
            log = format!("[ERROR] {error}");
        }
        for warning in &catalog.warnings {
            if log == "Process logs will appear here..." {
                log.clear();
            }
            log.push('\n');
            log.push_str("[WARN] ");
            log.push_str(warning);
        }

        Self {
            base_dir,
            page: Page::Install,
            apps: catalog.apps,
            admin_tasks: admin_tasks(),
            install_selected: HashSet::new(),
            remove_selected: HashSet::new(),
            admin_selected: HashSet::new(),
            search: String::new(),
            category_filter: None,
            distro_name,
            package_manager,
            log,
            password: String::new(),
            show_password_modal: false,
            is_running: false,
            tx: None,
            rx: None,
            log_revision: 0,
            icons,
            distro_icons,
            theme_icons,
            theme,
        }
    }

    fn palette(&self) -> Palette {
        self.theme.palette()
    }

    fn toggle_theme(&mut self, ctx: &Context) {
        self.theme = self.theme.toggle();
        theme::apply_theme(ctx, self.theme);
        if let Err(error) = theme::save_theme_mode(self.theme) {
            if self.log == "Process logs will appear here..." {
                self.log.clear();
            }
            self.log.push_str(&format!(
                "\n[WARN] Could not save theme preference: {error}"
            ));
            self.log_revision = self.log_revision.saturating_add(1);
        }
    }

    fn selected_counts(&self) -> (usize, usize, usize) {
        (
            self.install_selected.len(),
            self.remove_selected.len(),
            self.admin_selected.len(),
        )
    }

    fn total_selected(&self) -> usize {
        let (install, remove, admin) = self.selected_counts();
        install + remove + admin
    }

    fn selected_tasks(&self) -> Result<Vec<Task>, Vec<String>> {
        let mut tasks = Vec::new();
        let mut errors = Vec::new();

        for index in &self.install_selected {
            if let Some(entry) = self.apps.get(*index) {
                match entry.try_command(&self.base_dir, "install") {
                    Ok(command) => tasks.push(Task {
                        description: format!("Installing {}", entry.label),
                        command,
                    }),
                    Err(error) => errors.push(format!("{}: {error}", entry.label)),
                }
            }
        }

        for index in &self.remove_selected {
            if let Some(entry) = self.apps.get(*index) {
                match entry.try_command(&self.base_dir, "remove") {
                    Ok(command) => tasks.push(Task {
                        description: format!("Removing {}", entry.label),
                        command,
                    }),
                    Err(error) => errors.push(format!("{}: {error}", entry.label)),
                }
            }
        }

        for index in &self.admin_selected {
            if let Some(task) = self.admin_tasks.get(*index) {
                match task.try_command(&self.base_dir) {
                    Ok(command) => tasks.push(Task {
                        description: format!("Running {}", task.label),
                        command,
                    }),
                    Err(error) => errors.push(format!("{}: {error}", task.label)),
                }
            }
        }

        if errors.is_empty() {
            Ok(tasks)
        } else {
            Err(errors)
        }
    }

    fn clear_selection(&mut self) {
        self.install_selected.clear();
        self.remove_selected.clear();
        self.admin_selected.clear();
    }

    fn select_visible_apps(&mut self, selected: bool) {
        let visible: Vec<usize> = self
            .apps
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| self.app_matches_filter(entry).then_some(index))
            .collect();

        let target = match self.page {
            Page::Install => &mut self.install_selected,
            Page::Remove => &mut self.remove_selected,
            _ => return,
        };

        if selected {
            target.extend(visible);
        } else {
            for index in visible {
                target.remove(&index);
            }
        }
    }

    fn app_matches_filter(&self, entry: &AppEntry) -> bool {
        if let Some(category) = &self.category_filter
            && &entry.category != category
        {
            return false;
        }

        let needle = self.search.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }

        entry.label.to_lowercase().contains(&needle)
            || entry.package_name.to_lowercase().contains(&needle)
            || entry.flatpak_id.to_lowercase().contains(&needle)
            || entry.category.to_lowercase().contains(&needle)
    }

    fn run_selected(&mut self, ctx: &Context) {
        if self.is_running {
            return;
        }

        let tasks = match self.selected_tasks() {
            Ok(tasks) if !tasks.is_empty() => tasks,
            Ok(_) => return,
            Err(errors) => {
                self.log = "[ERROR] Refusing to run tasks with invalid identifiers.".to_owned();
                for error in errors {
                    self.log.push('\n');
                    self.log.push_str(&error);
                }
                self.log_revision = self.log_revision.saturating_add(1);
                zeroize_string(&mut self.password);
                return;
            }
        };

        let (tx, rx) = mpsc::channel();
        let password = std::mem::take(&mut self.password);
        self.log = format!("Queued {} task(s)...", tasks.len());
        self.log_revision = self.log_revision.saturating_add(1);
        self.is_running = true;
        self.tx = Some(tx.clone());
        self.rx = Some(rx);

        let repaint = ctx.clone();
        thread::spawn(move || {
            run_tasks(tasks, password, tx);
            repaint.request_repaint();
        });
    }

    fn drain_runner_messages(&mut self) {
        let mut done = false;
        if let Some(rx) = &self.rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    RunnerMessage::Log(line) => {
                        if self.log == "Process logs will appear here..." {
                            self.log.clear();
                        }
                        self.log.push('\n');
                        self.log.push_str(&strip_ansi(&line));
                        self.log_revision = self.log_revision.saturating_add(1);
                    }
                    RunnerMessage::Done => done = true,
                }
            }
        }

        if done {
            self.is_running = false;
            zeroize_string(&mut self.password);
            self.tx = None;
            self.rx = None;
            self.log.push_str("\n\n[DONE] All tasks finished.");
            self.log_revision = self.log_revision.saturating_add(1);
        }
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            toolbox_icon(ui, 26.0, &palette);
            ui.label(
                RichText::new("Toolbox")
                    .font(FontId::proportional(17.0))
                    .color(palette.chrome_text)
                    .strong(),
            );
        });
        ui.add_space(24.0);

        for page in [
            Page::Install,
            Page::Remove,
            Page::Administration,
            Page::SystemInfo,
        ] {
            if nav_button(ui, page, self.page == page, &palette).clicked() {
                self.page = page;
                self.category_filter = None;
                self.search.clear();
            }
            ui.add_space(4.0);
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(12.0);
            let about = nav_aux_button(ui, "About", "i", &palette);
            if about.clicked() {
                self.log.push_str("\n[INFO] The Linux IT Guy Toolbox");
                self.log_revision = self.log_revision.saturating_add(1);
            }
            ui.add_space(4.0);
            let theme_icon = theme_icon_texture(self, palette.mode).cloned();
            if theme_toggle_button(ui, &palette, theme_icon.as_ref()).clicked() {
                let ctx = ui.ctx().clone();
                self.toggle_theme(&ctx);
            }
            ui.add_space(20.0);
            ui.label(
                RichText::new(format!("Package manager: {}", self.package_manager))
                    .color(palette.chrome_subtle),
            );
            ui.label(
                RichText::new(&self.distro_name)
                    .color(palette.chrome_text)
                    .strong(),
            );
        });
    }

    fn header(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.set_min_height(76.0);
        ui.horizontal(|ui| {
            page_icon(ui, self.page, 46.0, true, &palette);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(self.page.title())
                        .font(FontId::proportional(25.0))
                        .color(palette.chrome_text)
                        .strong(),
                );
                ui.label(RichText::new(page_subtitle(self.page)).color(palette.chrome_subtle));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for label in ["Fedora", "Debian", "Arch"] {
                    let selected = distro_selected(label, &self.package_manager, &self.distro_name);
                    chip(
                        ui,
                        label,
                        selected,
                        self.distro_icons.get(distro_key(label)),
                        &palette,
                    );
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Supported Distros:")
                        .color(palette.chrome_subtle)
                        .strong(),
                );
            });
        });
    }

    fn install_remove_page(&mut self, ui: &mut Ui, install: bool) {
        self.toolbar(ui);
        ui.add_space(10.0);

        let mut categories: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, entry) in self.apps.iter().enumerate() {
            if self.app_matches_filter(entry) {
                categories
                    .entry(entry.category.clone())
                    .or_default()
                    .push(index);
            }
        }

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (category_index, (category, indexes)) in categories.into_iter().enumerate() {
                    if category_index == 0 {
                        self.section_header_with_actions(ui, &category, indexes.len());
                    } else {
                        section_header(ui, &category, indexes.len(), &self.palette());
                    }
                    let tile_width = ((ui.available_width() - 12.0) / 2.0).max(360.0);
                    for chunk in indexes.chunks(2) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            for index in chunk {
                                self.app_tile(ui, *index, install, tile_width);
                            }
                            if chunk.len() == 1 {
                                ui.allocate_exact_size(vec2(tile_width, 1.0), Sense::hover());
                            }
                        });
                        ui.add_space(6.0);
                    }
                    ui.add_space(10.0);
                }
            });
    }

    fn section_header_with_actions(&mut self, ui: &mut Ui, title: &str, count: usize) {
        let palette = self.palette();
        ui.horizontal(|ui| {
            category_icon(ui, title, &palette);
            ui.label(
                RichText::new(title)
                    .font(FontId::proportional(15.0))
                    .color(palette.chrome_text)
                    .strong(),
            );
            count_badge(ui, count, &palette);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(1.0);
            });
        });
        ui.add_space(6.0);
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        toolbar_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                let actions_width = 420.0;
                let category_width = (ui.available_width() - actions_width).max(360.0);
                ui.allocate_ui_with_layout(
                    vec2(category_width, 34.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let categories = self.categories();
                        if filter_chip(ui, "All", self.category_filter.is_none(), &palette)
                            .clicked()
                        {
                            self.category_filter = None;
                        }
                        for category in categories {
                            if ui.available_width() < 74.0 {
                                break;
                            }
                            if filter_chip(
                                ui,
                                &category,
                                self.category_filter.as_ref() == Some(&category),
                                &palette,
                            )
                            .clicked()
                            {
                                self.category_filter = Some(category);
                            }
                        }
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if flat_text_button(ui, "Clear", palette.chrome_subtle, 60.0, &palette)
                        .clicked()
                    {
                        self.select_visible_apps(false);
                    }
                    if flat_text_button(ui, "Select All", palette.chrome_text, 92.0, &palette)
                        .clicked()
                    {
                        self.select_visible_apps(true);
                    }
                    search_field(ui, &mut self.search, "Filter apps...", &palette);
                });
            });
        });
    }

    fn categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self
            .apps
            .iter()
            .map(|entry| entry.category.clone())
            .collect();
        categories.sort();
        categories.dedup();
        categories
    }

    fn app_tile(&mut self, ui: &mut Ui, index: usize, install: bool, width: f32) {
        let entry = &self.apps[index];
        let label = entry.label.clone();
        let icon = self
            .icons
            .get(icon_key(&entry.label))
            .or_else(|| self.icons.get(icon_key(entry.exec_name.as_str())))
            .cloned();
        let source = entry.source_label();
        let detail = if !entry.flatpak_id.is_empty() {
            entry.flatpak_id.clone()
        } else if !entry.package_name.is_empty() {
            entry.package_name.clone()
        } else {
            "custom task".to_owned()
        };
        let notes = if entry.notes.is_empty() {
            app_description(&entry.label).to_owned()
        } else {
            entry.notes.clone()
        };
        let selected = if install {
            self.install_selected.contains(&index)
        } else {
            self.remove_selected.contains(&index)
        };
        let palette = self.palette();

        let (rect, response) = ui.allocate_exact_size(vec2(width, 42.0), Sense::click());
        let hovered = response.hovered();
        paint_row_background(ui.painter(), rect, hovered, selected, &palette);

        let mut check_rect = Rect::from_min_size(rect.min + vec2(10.0, 13.0), vec2(16.0, 16.0));
        check_rect = check_rect.expand(0.5);
        paint_checkbox(ui.painter(), check_rect, selected, &palette);

        let icon_rect = Rect::from_min_size(rect.min + vec2(34.0, 8.0), vec2(26.0, 26.0));
        paint_app_icon(ui.painter(), icon_rect, &label, icon.as_ref(), &palette);

        let label_text = elide(&label, 19);

        ui.painter().text(
            pos2(rect.min.x + 70.0, rect.center().y - 1.0),
            Align2::LEFT_CENTER,
            label_text,
            FontId::proportional(14.0),
            palette.chrome_text,
        );

        let badge_center_x = rect.min.x + width * 0.42;
        paint_badge(
            ui.painter(),
            pos2(badge_center_x, rect.center().y),
            source,
            &palette,
        );

        let description_x = rect.min.x + width * 0.62;
        ui.painter().text(
            pos2(description_x, rect.center().y),
            Align2::LEFT_CENTER,
            if notes.is_empty() {
                detail.as_str()
            } else {
                notes.as_str()
            },
            FontId::proportional(11.5),
            palette.chrome_subtle,
        );

        if response.clicked() {
            self.set_app_selected(index, install, !selected);
        }
    }

    fn set_app_selected(&mut self, index: usize, install: bool, selected: bool) {
        if install {
            if selected {
                self.install_selected.insert(index);
                self.remove_selected.remove(&index);
            } else {
                self.install_selected.remove(&index);
            }
        } else if selected {
            self.remove_selected.insert(index);
            self.install_selected.remove(&index);
        } else {
            self.remove_selected.remove(&index);
        }
    }

    fn admin_page(&mut self, ui: &mut Ui) {
        let mut categories: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, task) in self.admin_tasks.iter().enumerate() {
            categories
                .entry(task.category.clone())
                .or_default()
                .push(index);
        }

        ScrollArea::vertical()
            .id_salt("administration_tasks")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (category_index, (category, indexes)) in categories.into_iter().enumerate() {
                    if category_index == 0 {
                        self.admin_section_header_with_actions(ui, &category, indexes.len());
                    } else {
                        section_header(ui, &category, indexes.len(), &self.palette());
                    }

                    let tile_width = ((ui.available_width() - 12.0) / 2.0).max(360.0);
                    for chunk in indexes.chunks(2) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 12.0;
                            for index in chunk {
                                self.admin_tile(ui, *index, tile_width);
                            }
                            if chunk.len() == 1 {
                                ui.allocate_exact_size(vec2(tile_width, 1.0), Sense::hover());
                            }
                        });
                        ui.add_space(6.0);
                    }
                    ui.add_space(12.0);
                }
            });
    }

    fn admin_section_header_with_actions(&mut self, ui: &mut Ui, title: &str, count: usize) {
        let palette = self.palette();
        ui.horizontal(|ui| {
            category_icon(ui, title, &palette);
            ui.label(
                RichText::new(title)
                    .font(FontId::proportional(15.0))
                    .color(palette.chrome_text)
                    .strong(),
            );
            count_badge(ui, count, &palette);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_sized(
                        [68.0, 32.0],
                        Button::new(RichText::new("Clear").color(palette.chrome_subtle))
                            .fill(palette.tile)
                            .stroke(Stroke::new(1.0_f32, palette.border)),
                    )
                    .clicked()
                {
                    self.admin_selected.clear();
                }
                if ui
                    .add_sized(
                        [92.0, 32.0],
                        Button::new(RichText::new("Select All").color(palette.chrome_text))
                            .fill(palette.tile)
                            .stroke(Stroke::new(1.0_f32, palette.border)),
                    )
                    .clicked()
                {
                    self.admin_selected = (0..self.admin_tasks.len()).collect();
                }
            });
        });
        ui.add_space(6.0);
    }

    fn admin_tile(&mut self, ui: &mut Ui, index: usize, width: f32) {
        let task = &self.admin_tasks[index];
        let label = task.label.clone();
        let script = task.script.clone();
        let selected = self.admin_selected.contains(&index);
        let palette = self.palette();

        let (rect, response) = ui.allocate_exact_size(vec2(width, 42.0), Sense::click());
        paint_row_background(ui.painter(), rect, response.hovered(), selected, &palette);

        let mut check_rect = Rect::from_min_size(rect.min + vec2(10.0, 13.0), vec2(16.0, 16.0));
        check_rect = check_rect.expand(0.5);
        paint_checkbox(ui.painter(), check_rect, selected, &palette);

        let icon_rect = Rect::from_min_size(rect.min + vec2(34.0, 8.0), vec2(26.0, 26.0));
        paint_admin_icon(ui.painter(), icon_rect, &label);

        ui.painter().text(
            pos2(rect.min.x + 70.0, rect.center().y - 1.0),
            Align2::LEFT_CENTER,
            elide(&label, 24),
            FontId::proportional(14.0),
            palette.chrome_text,
        );

        ui.painter().text(
            pos2(rect.min.x + width * 0.58, rect.center().y),
            Align2::LEFT_CENTER,
            admin_description(&label, &script),
            FontId::proportional(11.5),
            palette.chrome_subtle,
        );

        if response.clicked() {
            if selected {
                self.admin_selected.remove(&index);
            } else {
                self.admin_selected.insert(index);
            }
        }
    }

    fn system_info_page(&mut self, ui: &mut Ui) {
        let rows = [
            ("OS", self.distro_name.clone()),
            ("Host", command_output("hostname", &[])),
            ("Kernel", command_output("uname", &["-r"])),
            ("Uptime", uptime()),
            ("Shell", env_or_unknown(&["SHELL"])),
            (
                "DE/WM",
                env_or_unknown(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]),
            ),
            ("Package Manager", self.package_manager.clone()),
            ("App Directory", self.base_dir.display().to_string()),
        ];

        let palette = self.palette();
        Grid::new("system_info_grid")
            .num_columns(2)
            .spacing(Vec2::new(18.0, 12.0))
            .show(ui, |ui| {
                for (label, value) in rows {
                    ui.label(RichText::new(label).color(palette.chrome_subtle).strong());
                    panel_frame(&palette).show(ui, |ui| {
                        ui.label(RichText::new(value).color(palette.on_surface));
                    });
                    ui.end_row();
                }
            });
    }

    fn bottom_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        let (install, remove, admin) = self.selected_counts();
        let palette = self.palette();
        summary_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                let summary = if self.total_selected() == 0 {
                    "0 apps selected".to_owned()
                } else {
                    format!("{} selected", self.total_selected(),)
                };
                ui.label(RichText::new(summary).color(palette.on_surface).strong());
                ui.separator();
                ui.label(
                    RichText::new(format!("{} tasks", self.total_selected()))
                        .color(palette.on_surface_subtle),
                );
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{} install  /  {} remove  /  {} admin",
                        install, remove, admin
                    ))
                    .color(palette.on_surface_subtle),
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let run = ui.add_enabled(
                        !self.is_running && self.total_selected() > 0,
                        Button::new(
                            RichText::new(if self.is_running {
                                "Running..."
                            } else {
                                "Run Selected Tasks"
                            })
                            .color(palette.cta_text)
                            .strong(),
                        )
                        .fill(palette.cta_fill)
                        .stroke(Stroke::new(1.0_f32, palette.cta_fill)),
                    );
                    if run.clicked() {
                        self.show_password_modal = true;
                    }
                    if ui
                        .add_enabled(
                            !self.is_running,
                            Button::new(RichText::new("Clear Selection").color(palette.on_surface))
                                .fill(palette.surface_hover)
                                .stroke(Stroke::new(1.0_f32, palette.border)),
                        )
                        .clicked()
                    {
                        self.clear_selection();
                    }
                });
            });
        });

        ui.add_space(8.0);
        log_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                small_terminal_icon(ui, &palette);
                ui.label(
                    RichText::new("Process Log")
                        .color(palette.on_surface)
                        .strong(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add(
                            Button::new(
                                RichText::new("Clear Log").color(palette.on_surface_subtle),
                            )
                            .fill(palette.surface_hover)
                            .stroke(Stroke::new(1.0_f32, palette.border)),
                        )
                        .clicked()
                    {
                        self.log = "Process logs will appear here...".to_owned();
                        self.log_revision = self.log_revision.saturating_add(1);
                    }
                });
            });
            ui.add_space(6.0);
            let log_height = (ui.available_height() - 6.0).max(96.0);
            let showing_placeholder = self.log == "Process logs will appear here...";
            Frame::new()
                .fill(palette.log_inner)
                .stroke(Stroke::new(1.0_f32, palette.border))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    if showing_placeholder {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(log_height - 18.0);
                        ui.label(
                            RichText::new(&self.log)
                                .font(FontId::monospace(12.5))
                                .color(palette.log_text),
                        );
                    } else {
                        ScrollArea::vertical()
                            .id_salt("process_log_scroll")
                            .stick_to_bottom(true)
                            .max_height(log_height)
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                ui.set_min_height(log_height - 18.0);
                                ui.label(
                                    RichText::new(&self.log)
                                        .font(FontId::monospace(12.5))
                                        .color(palette.log_text),
                                );
                                ui.scroll_to_cursor(Some(Align::BOTTOM));
                            });
                    }
                });
        });

        if self.show_password_modal && ctx.input(|input| input.key_pressed(Key::Escape)) {
            zeroize_string(&mut self.password);
            self.show_password_modal = false;
        }

        if self.show_password_modal {
            let mut open = true;
            egui::Window::new("Sudo Authentication")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .open(&mut open)
                .frame(
                    Frame::new()
                        .fill(palette.modal_fill)
                        .stroke(Stroke::new(1.0_f32, palette.border))
                        .inner_margin(16.0)
                        .corner_radius(8.0),
                )
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new("Enter your sudo password to run the selected tasks.")
                            .color(palette.modal_text),
                    );
                    ui.add_space(8.0);
                    let response = ui.add(
                        TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(280.0)
                            .text_color(palette.on_surface)
                            .background_color(palette.input_fill),
                    );
                    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        self.show_password_modal = false;
                        self.run_selected(ctx);
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized(
                                [88.0, 32.0],
                                Button::new(
                                    RichText::new("Cancel").color(palette.cancel_text).strong(),
                                )
                                .fill(palette.cancel_fill)
                                .stroke(Stroke::new(1.0_f32, palette.cancel_text)),
                            )
                            .clicked()
                        {
                            zeroize_string(&mut self.password);
                            self.show_password_modal = false;
                        }
                        if ui
                            .add_enabled(
                                !self.password.is_empty(),
                                Button::new(RichText::new("Run").color(palette.cta_text).strong())
                                    .fill(palette.cta_fill)
                                    .stroke(Stroke::new(1.0_f32, palette.cta_fill)),
                            )
                            .clicked()
                        {
                            self.show_password_modal = false;
                            self.run_selected(ctx);
                        }
                    });
                });
            if !open {
                zeroize_string(&mut self.password);
                self.show_password_modal = false;
            }
        }
    }
}

impl eframe::App for ToolboxApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.drain_runner_messages();

        let palette = self.palette();
        Panel::left("sidebar")
            .resizable(false)
            .exact_size(252.0)
            .frame(
                Frame::new()
                    .fill(palette.sidebar)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .inner_margin(14.0),
            )
            .show_inside(ui, |ui| self.sidebar(ui));

        Panel::top("header")
            .frame(
                Frame::new()
                    .fill(palette.background)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .inner_margin(egui::Margin::symmetric(16, 14)),
            )
            .show_inside(ui, |ui| self.header(ui));

        Panel::bottom("bottom")
            .exact_size(260.0)
            .frame(
                Frame::new()
                    .fill(palette.background)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show_inside(ui, |ui| {
                let ctx = ui.ctx().clone();
                self.bottom_bar(ui, &ctx);
            });

        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(palette.background)
                    .inner_margin(egui::Margin::symmetric(14, 8)),
            )
            .show_inside(ui, |ui| match self.page {
                Page::Install => self.install_remove_page(ui, true),
                Page::Remove => self.install_remove_page(ui, false),
                Page::Administration => self.admin_page(ui),
                Page::SystemInfo => self.system_info_page(ui),
            });
    }
}

fn install_fonts(ctx: &Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "Inter".to_owned(),
        FontData::from_static(include_bytes!("../assets/fonts/InterVariable.ttf")).into(),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".to_owned());
    ctx.set_fonts(fonts);
}

// App tiles: Chris's PNGs plus the Origin SVG, all compiled in. Missing the
// checkout's assets folder at runtime does not affect these textures.
fn load_icons(ctx: &Context) -> HashMap<&'static str, TextureHandle> {
    let mut icons = HashMap::new();
    for &(key, bytes) in crate::logos::APP_PNGS {
        if let Some(color_image) = crate::logos::decode_png(bytes) {
            icons.insert(
                key,
                ctx.load_texture(format!("app-{key}"), color_image, TextureOptions::LINEAR),
            );
        }
    }
    if let Some(raster) = crate::logos::rasterize_svg_markup(crate::logos::BRAVE_ORIGIN_SVG, 64) {
        icons.insert(
            "brave-origin",
            ctx.load_texture(
                "app-brave-origin",
                crate::logos::color_image_from_raster(&raster),
                TextureOptions::LINEAR,
            ),
        );
    }
    icons
}

fn load_distro_icons(ctx: &Context) -> HashMap<&'static str, TextureHandle> {
    let mut icons = HashMap::new();
    for &(key, svg) in crate::logos::DISTRO_SVGS {
        if let Some(raster) = crate::logos::rasterize_svg_markup(svg, 64) {
            let color_image = crate::logos::color_image_from_raster(&raster);
            icons.insert(
                key,
                ctx.load_texture(format!("distro-{key}"), color_image, TextureOptions::LINEAR),
            );
        }
    }
    icons
}

fn load_theme_icons(ctx: &Context) -> HashMap<&'static str, TextureHandle> {
    let mut icons = HashMap::new();
    for (key, bytes) in [
        ("sun", theme::SUN_DARK_MODE_PNG),
        ("moon", theme::MOON_LIGHT_MODE_PNG),
    ] {
        if let Some(color_image) = crate::logos::decode_png(bytes) {
            icons.insert(
                key,
                ctx.load_texture(format!("theme-{key}"), color_image, TextureOptions::LINEAR),
            );
        }
    }
    icons
}

fn theme_icon_texture(app: &ToolboxApp, mode: ThemeMode) -> Option<&TextureHandle> {
    match mode.toggle_icon() {
        ThemeIcon::Sun => app.theme_icons.get("sun"),
        ThemeIcon::Moon => app.theme_icons.get("moon"),
    }
}

fn nav_button(ui: &mut Ui, page: Page, selected: bool, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let fill = if selected {
        palette.nav_selected
    } else if response.hovered() {
        palette.nav_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 7.0, fill);
    if selected {
        let marker = Rect::from_min_size(rect.min + vec2(0.0, 9.0), vec2(3.0, 26.0));
        ui.painter().rect_filled(marker, 2.0, palette.nav_marker);
    }
    paint_page_symbol(
        ui.painter(),
        Rect::from_min_size(rect.min + vec2(18.0, 10.0), vec2(24.0, 24.0)),
        page,
        palette.chrome_text,
    );
    ui.painter().text(
        pos2(rect.min.x + 56.0, rect.center().y),
        Align2::LEFT_CENTER,
        page.title(),
        FontId::proportional(15.0),
        palette.chrome_text,
    );
    response
}

fn nav_aux_button(ui: &mut Ui, label: &str, symbol: &str, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 7.0, palette.nav_hover);
    }
    let icon = Rect::from_min_size(rect.min + vec2(20.0, 9.0), vec2(22.0, 22.0));
    ui.painter().circle_stroke(
        icon.center(),
        10.0,
        Stroke::new(1.5_f32, palette.chrome_subtle),
    );
    ui.painter().text(
        icon.center(),
        Align2::CENTER_CENTER,
        symbol,
        FontId::proportional(14.0),
        palette.chrome_subtle,
    );
    ui.painter().text(
        pos2(rect.min.x + 56.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        palette.chrome_subtle,
    );
    response
}

fn theme_toggle_button(
    ui: &mut Ui,
    palette: &Palette,
    icon: Option<&TextureHandle>,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(52.0, 48.0), Sense::click());
    let response = response.on_hover_text(palette.mode.toggle_tooltip());
    if response.hovered() {
        ui.painter().rect_filled(rect, 8.0, palette.nav_hover);
    }
    let icon_rect = Rect::from_center_size(rect.center(), vec2(44.0, 40.0));
    if let Some(icon) = icon {
        ui.painter().image(
            icon.id(),
            icon_rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    response
}

fn chip(ui: &mut Ui, label: &str, selected: bool, icon: Option<&TextureHandle>, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(104.0, 34.0), Sense::hover());
    let fill = if selected {
        palette.chip_selected
    } else {
        palette.chip_idle
    };
    ui.painter().rect_filled(rect, 7.0, fill);
    ui.painter().rect_stroke(
        rect,
        7.0,
        Stroke::new(
            1.0_f32,
            if selected {
                palette.accent
            } else {
                palette.border
            },
        ),
        StrokeKind::Inside,
    );
    distro_mark(
        ui.painter(),
        Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2(18.0, 18.0)),
        label,
        selected,
        icon,
        palette,
    );
    ui.painter().text(
        pos2(rect.min.x + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        if selected {
            palette.chip_selected_text
        } else {
            palette.chip_idle_text
        },
    );
}

fn filter_chip(ui: &mut Ui, label: &str, selected: bool, palette: &Palette) -> egui::Response {
    let width = (label.len() as f32 * 7.0 + 22.0).clamp(42.0, 104.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let fill = if selected {
        palette.filter_selected_fill
    } else if response.hovered() {
        palette.nav_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 5.0, fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.0),
        if selected {
            palette.filter_selected_text
        } else {
            palette.filter_idle_text
        },
    );
    response
}

fn flat_text_button(
    ui: &mut Ui,
    label: &str,
    color: Color32,
    width: f32,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 5.0, palette.nav_hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.5),
        color,
    );
    response
}

fn search_field(ui: &mut Ui, search: &mut String, hint: &str, palette: &Palette) {
    let response = Frame::new()
        .fill(palette.input_fill)
        .stroke(Stroke::new(1.0_f32, palette.input_border))
        .corner_radius(5.0)
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.set_min_width(260.0);
            ui.set_max_width(260.0);
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(18.0, 32.0), Sense::hover());
                paint_search_icon(ui.painter(), icon_rect, palette);
                ui.add_sized(
                    [220.0, 32.0],
                    TextEdit::singleline(search)
                        .hint_text(hint)
                        .font(FontId::proportional(13.0))
                        .desired_width(220.0)
                        .text_color(palette.on_surface)
                        .background_color(palette.input_fill),
                );
            });
        })
        .response;

    if response.clicked() {
        response.request_focus();
    }
}

fn paint_search_icon(painter: &Painter, rect: Rect, palette: &Palette) {
    let center = pos2(rect.left() + 8.0, rect.center().y - 1.0);
    painter.circle_stroke(center, 5.0, Stroke::new(1.4_f32, palette.on_surface_subtle));
    painter.line_segment(
        [
            pos2(center.x + 4.0, center.y + 4.0),
            pos2(center.x + 9.0, center.y + 9.0),
        ],
        Stroke::new(1.4_f32, palette.on_surface_subtle),
    );
}

fn count_badge(ui: &mut Ui, count: usize, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(28.0, 20.0), Sense::hover());
    ui.painter().rect_filled(rect, 10.0, palette.count_badge);
    ui.painter().rect_stroke(
        rect,
        10.0,
        Stroke::new(1.0_f32, palette.border),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{count}"),
        FontId::proportional(11.0),
        palette.count_badge_text,
    );
}

fn section_header(ui: &mut Ui, title: &str, count: usize, palette: &Palette) {
    ui.horizontal(|ui| {
        category_icon(ui, title, palette);
        ui.label(
            RichText::new(title)
                .font(FontId::proportional(15.0))
                .color(palette.chrome_text)
                .strong(),
        );
        count_badge(ui, count, palette);
    });
    ui.add_space(6.0);
}

fn toolbar_frame(palette: &Palette) -> Frame {
    Frame::new()
        .fill(palette.toolbar)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .inner_margin(egui::Margin::symmetric(12, 7))
}

fn summary_frame(palette: &Palette) -> Frame {
    Frame::new()
        .fill(palette.summary)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .inner_margin(egui::Margin::symmetric(12, 8))
}

fn log_frame(palette: &Palette) -> Frame {
    Frame::new()
        .fill(palette.log_frame)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .inner_margin(egui::Margin::symmetric(10, 8))
}

fn panel_frame(palette: &Palette) -> Frame {
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0_f32, palette.border))
        .inner_margin(12.0)
}

fn paint_row_background(
    painter: &Painter,
    rect: Rect,
    _hovered: bool,
    selected: bool,
    palette: &Palette,
) {
    painter.rect_filled(rect, 4.0, palette.tile);
    if selected {
        painter.rect_stroke(
            rect,
            4.0,
            Stroke::new(1.0_f32, palette.accent),
            StrokeKind::Inside,
        );
    }
}

fn paint_checkbox(painter: &Painter, rect: Rect, selected: bool, palette: &Palette) {
    painter.rect_filled(
        rect,
        3.0,
        if selected {
            palette.cta_fill
        } else {
            palette.checkbox_empty
        },
    );
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(
            1.0_f32,
            if selected {
                palette.cta_fill
            } else {
                palette.checkbox_border
            },
        ),
        StrokeKind::Inside,
    );
    if selected {
        let a = pos2(rect.left() + 4.0, rect.center().y);
        let b = pos2(rect.left() + 7.0, rect.bottom() - 5.0);
        let c = pos2(rect.right() - 4.0, rect.top() + 5.0);
        painter.line_segment([a, b], Stroke::new(2.0_f32, palette.checkbox_check));
        painter.line_segment([b, c], Stroke::new(2.0_f32, palette.checkbox_check));
    }
}

fn paint_app_icon(
    painter: &Painter,
    rect: Rect,
    label: &str,
    icon: Option<&TextureHandle>,
    palette: &Palette,
) {
    if let Some(icon) = icon {
        painter.circle_filled(rect.center(), rect.width() / 2.0, palette.icon_well);
        painter.image(
            icon.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }

    let color = app_color(label);
    painter.circle_filled(rect.center(), rect.width() / 2.0, color);
    painter.circle_stroke(
        rect.center(),
        rect.width() / 2.0,
        Stroke::new(1.0_f32, Color32::from_white_alpha(80)),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        app_mark(label),
        FontId::proportional(13.0),
        Color32::WHITE,
    );
}

fn paint_admin_icon(painter: &Painter, rect: Rect, label: &str) {
    let color = match label {
        "Enable Bluetooth" => Color32::from_rgb(37, 123, 218),
        "Disable Bluetooth" => Color32::from_rgb(105, 113, 124),
        "TLP (Laptops)" => Color32::from_rgb(65, 150, 91),
        "Powertop" => Color32::from_rgb(229, 168, 58),
        "Update System" => Color32::from_rgb(60, 128, 220),
        "nala (rank mirrors) - Debian only" => Color32::from_rgb(185, 48, 67),
        "Stacer" => Color32::from_rgb(52, 179, 174),
        "SWAP Fix" => Color32::from_rgb(142, 95, 214),
        "Fastfetch" => Color32::from_rgb(225, 116, 54),
        _ => Color32::from_rgb(82, 102, 126),
    };

    painter.circle_filled(rect.center(), rect.width() / 2.0, color);
    painter.circle_stroke(
        rect.center(),
        rect.width() / 2.0,
        Stroke::new(1.0_f32, Color32::from_white_alpha(80)),
    );

    let stroke = Stroke::new(1.7_f32, Color32::WHITE);
    let c = rect.center();
    match label {
        "Enable Bluetooth" | "Disable Bluetooth" => {
            painter.line_segment(
                [pos2(c.x, rect.top() + 5.0), pos2(c.x, rect.bottom() - 5.0)],
                stroke,
            );
            painter.line_segment(
                [pos2(c.x, c.y), pos2(rect.right() - 6.0, rect.top() + 7.0)],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(c.x, c.y),
                    pos2(rect.right() - 6.0, rect.bottom() - 7.0),
                ],
                stroke,
            );
            painter.line_segment(
                [pos2(c.x, rect.top() + 5.0), pos2(rect.right() - 6.0, c.y)],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(c.x, rect.bottom() - 5.0),
                    pos2(rect.right() - 6.0, c.y),
                ],
                stroke,
            );
            if label == "Disable Bluetooth" {
                painter.line_segment(
                    [
                        pos2(rect.left() + 6.0, rect.bottom() - 6.0),
                        pos2(rect.right() - 6.0, rect.top() + 6.0),
                    ],
                    Stroke::new(2.1_f32, Color32::WHITE),
                );
            }
        }
        "TLP (Laptops)" => {
            painter.rect_stroke(rect.shrink(6.0), 2.0, stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    pos2(rect.left() + 7.0, rect.bottom() - 6.0),
                    pos2(rect.right() - 7.0, rect.bottom() - 6.0),
                ],
                stroke,
            );
        }
        "Powertop" => {
            let points = [
                pos2(c.x + 1.0, rect.top() + 4.0),
                pos2(rect.left() + 8.0, c.y + 1.0),
                pos2(c.x - 1.0, c.y + 1.0),
                pos2(c.x - 1.0, rect.bottom() - 4.0),
                pos2(rect.right() - 7.0, c.y - 2.0),
                pos2(c.x + 1.0, c.y - 2.0),
            ];
            painter.add(egui::Shape::closed_line(points.to_vec(), stroke));
        }
        "Update System" => {
            painter.circle_stroke(c, 8.0, stroke);
            painter.line_segment(
                [
                    pos2(c.x + 6.0, rect.top() + 7.0),
                    pos2(rect.right() - 4.0, rect.top() + 7.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.right() - 4.0, rect.top() + 7.0),
                    pos2(rect.right() - 6.0, rect.top() + 12.0),
                ],
                stroke,
            );
        }
        "SWAP Fix" => {
            painter.line_segment(
                [
                    pos2(rect.left() + 6.0, c.y - 4.0),
                    pos2(rect.right() - 6.0, c.y - 4.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 6.0, c.y + 4.0),
                    pos2(rect.right() - 6.0, c.y + 4.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.right() - 10.0, c.y - 8.0),
                    pos2(rect.right() - 6.0, c.y - 4.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.right() - 10.0, c.y + 8.0),
                    pos2(rect.right() - 6.0, c.y + 4.0),
                ],
                stroke,
            );
        }
        "Fastfetch" => {
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "ff",
                FontId::proportional(12.0),
                Color32::WHITE,
            );
        }
        _ => {
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "*",
                FontId::proportional(16.0),
                Color32::WHITE,
            );
        }
    }
}

fn paint_badge(painter: &Painter, center: Pos2, label: &str, palette: &Palette) {
    let width = if label == "native + flatpak" {
        104.0
    } else {
        56.0
    };
    let rect = Rect::from_center_size(center, vec2(width, 22.0));
    let native = label.contains("native");
    let fill = if native {
        palette.badge_native_fill
    } else {
        palette.badge_flatpak_fill
    };
    let stroke = if native {
        palette.badge_native_stroke
    } else {
        palette.badge_flatpak_stroke
    };
    let text = if native {
        palette.badge_native_text
    } else {
        palette.badge_flatpak_text
    };
    painter.rect_filled(rect, 5.0, fill);
    painter.rect_stroke(rect, 5.0, Stroke::new(1.0_f32, stroke), StrokeKind::Inside);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.0),
        text,
    );
}

fn toolbox_icon(ui: &mut Ui, size: f32, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let painter = ui.painter();
    let body = Rect::from_min_max(
        pos2(rect.left() + 3.0, rect.top() + 8.0),
        pos2(rect.right() - 3.0, rect.bottom() - 3.0),
    );
    painter.rect_stroke(
        body,
        3.0,
        Stroke::new(1.8_f32, palette.chrome_text),
        StrokeKind::Inside,
    );
    painter.line_segment(
        [
            pos2(body.left(), body.top() + 6.0),
            pos2(body.right(), body.top() + 6.0),
        ],
        Stroke::new(1.4_f32, palette.chrome_text),
    );
    painter.rect_stroke(
        Rect::from_min_size(
            pos2(rect.center().x - 5.0, rect.top() + 4.0),
            vec2(10.0, 6.0),
        ),
        2.0,
        Stroke::new(1.5_f32, palette.chrome_text),
        StrokeKind::Inside,
    );
}

fn page_icon(ui: &mut Ui, page: Page, size: f32, filled: bool, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if filled {
        ui.painter()
            .circle_filled(rect.center(), size / 2.0, palette.page_icon_fill);
    }
    paint_page_symbol(
        ui.painter(),
        rect.shrink(size * 0.24),
        page,
        palette.chrome_text,
    );
}

fn paint_page_symbol(painter: &Painter, rect: Rect, page: Page, color: Color32) {
    let stroke = Stroke::new(1.8_f32, color);
    match page {
        Page::Install => {
            painter.line_segment(
                [
                    pos2(rect.center().x, rect.top()),
                    pos2(rect.center().x, rect.bottom() - 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 4.0, rect.center().y),
                    pos2(rect.center().x, rect.bottom() - 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.right() - 4.0, rect.center().y),
                    pos2(rect.center().x, rect.bottom() - 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left(), rect.bottom()),
                    pos2(rect.right(), rect.bottom()),
                ],
                stroke,
            );
        }
        Page::Remove => {
            painter.rect_stroke(
                Rect::from_min_max(
                    pos2(rect.left() + 3.0, rect.top() + 6.0),
                    pos2(rect.right() - 3.0, rect.bottom()),
                ),
                2.0,
                stroke,
                StrokeKind::Inside,
            );
            painter.line_segment(
                [
                    pos2(rect.left(), rect.top() + 5.0),
                    pos2(rect.right(), rect.top() + 5.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.center().x - 4.0, rect.top()),
                    pos2(rect.center().x + 4.0, rect.top()),
                ],
                stroke,
            );
        }
        Page::Administration => {
            painter.circle_stroke(rect.center(), rect.width() * 0.38, stroke);
            painter.line_segment(
                [rect.center(), pos2(rect.center().x, rect.top() + 4.0)],
                stroke,
            );
            painter.line_segment(
                [rect.center(), pos2(rect.right() - 4.0, rect.center().y)],
                stroke,
            );
        }
        Page::SystemInfo => {
            painter.rect_stroke(rect, 2.0, stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    pos2(rect.left() + 4.0, rect.center().y),
                    pos2(rect.left() + 9.0, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 9.0, rect.center().y),
                    pos2(rect.left() + 12.0, rect.top() + 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 12.0, rect.top() + 6.0),
                    pos2(rect.left() + 17.0, rect.bottom() - 5.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 17.0, rect.bottom() - 5.0),
                    pos2(rect.right() - 4.0, rect.bottom() - 5.0),
                ],
                stroke,
            );
        }
    }
}

fn category_icon(ui: &mut Ui, title: &str, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
    let painter = ui.painter();
    let stroke = Stroke::new(1.4_f32, palette.chrome_subtle);
    match title {
        "Browsers" => {
            painter.circle_stroke(rect.center(), 9.0, stroke);
            painter.line_segment(
                [
                    pos2(rect.left() + 3.0, rect.center().y),
                    pos2(rect.right() - 3.0, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.center().x, rect.top() + 3.0),
                    pos2(rect.center().x, rect.bottom() - 3.0),
                ],
                stroke,
            );
        }
        "Communication" => {
            painter.rect_stroke(rect.shrink(3.0), 3.0, stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    pos2(rect.left() + 7.0, rect.bottom() - 4.0),
                    pos2(rect.left() + 5.0, rect.bottom() - 1.0),
                ],
                stroke,
            );
        }
        "Dev Tools" => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "</>",
                FontId::proportional(12.0),
                palette.chrome_subtle,
            );
        }
        "Gaming" => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "pad",
                FontId::proportional(9.0),
                palette.chrome_subtle,
            );
        }
        _ => {
            painter.circle_stroke(rect.center(), 8.0, stroke);
        }
    }
}

fn small_terminal_icon(ui: &mut Ui, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    ui.painter().rect_stroke(
        rect.shrink(2.0),
        2.0,
        Stroke::new(1.0_f32, palette.on_surface_subtle),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ">",
        FontId::monospace(10.0),
        palette.on_surface_subtle,
    );
}

fn distro_mark(
    painter: &Painter,
    rect: Rect,
    label: &str,
    selected: bool,
    icon: Option<&TextureHandle>,
    palette: &Palette,
) {
    if let Some(icon) = icon {
        painter.image(
            icon.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            if selected {
                palette.chip_selected_text
            } else {
                palette.chip_idle_text
            },
        );
        return;
    }

    let color = if selected {
        palette.chip_selected_text
    } else {
        palette.chip_idle_text
    };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        &label[..1],
        FontId::proportional(18.0),
        color,
    );
}

fn distro_key(label: &str) -> &'static str {
    match label {
        "Arch" => "arch",
        "Debian" => "debian",
        "Fedora" => "fedora",
        _ => "",
    }
}

fn distro_selected(label: &str, package_manager: &str, distro_name: &str) -> bool {
    let distro = distro_name.to_lowercase();
    match label {
        "Arch" => package_manager == "pacman" || distro.contains("arch"),
        "Debian" => {
            package_manager == "apt-get" || distro.contains("debian") || distro.contains("ubuntu")
        }
        "Fedora" => package_manager == "dnf" || distro.contains("fedora"),
        _ => false,
    }
}

fn app_color(label: &str) -> Color32 {
    match label {
        "Firefox" => Color32::from_rgb(238, 90, 47),
        "Brave Browser" | "Brave Origin" => Color32::from_rgb(246, 85, 42),
        "Google Chrome" => Color32::from_rgb(66, 133, 244),
        "Microsoft Edge" => Color32::from_rgb(19, 158, 175),
        "Opera" => Color32::from_rgb(218, 38, 55),
        "Vivaldi" => Color32::from_rgb(219, 55, 73),
        "Discord" => Color32::from_rgb(88, 101, 242),
        "Signal" => Color32::from_rgb(61, 135, 245),
        "Slack" => Color32::from_rgb(60, 180, 120),
        "Thunderbird" => Color32::from_rgb(63, 130, 230),
        "Visual Studio Code" => Color32::from_rgb(36, 137, 214),
        "PyCharm Community" => Color32::from_rgb(70, 190, 98),
        "Steam" => Color32::from_rgb(78, 116, 158),
        "Lutris" => Color32::from_rgb(218, 111, 35),
        _ => Color32::from_rgb(82, 102, 126),
    }
}

fn app_mark(label: &str) -> &str {
    match label {
        "Visual Studio Code" => "V",
        "PyCharm Community" => "PC",
        "Google Chrome" => "C",
        "Microsoft Edge" => "E",
        "Brave Origin" => "O",
        "Thunderbird" => "T",
        "ProtonUp-Qt" => "P",
        _ => label.get(0..1).unwrap_or("*"),
    }
}

fn app_description(label: &str) -> &'static str {
    match label {
        "Brave Browser" => "Privacy-focused browser",
        "Brave Origin" => "Native Origin browser",
        "Google Chrome" => "The web browser from Google",
        "Firefox" => "Web browser from Mozilla",
        "Microsoft Edge" => "Microsoft's web browser",
        "Opera" => "Fast and secure browser",
        "Vivaldi" => "Customizable web browser",
        "Zen" => "Modern Firefox-based browser",
        "Discord" => "Voice and text chat",
        "Signal" => "Private messenger",
        "Slack" => "Team communication hub",
        "Thunderbird" => "Email client from Mozilla",
        "Visual Studio Code" => "Code editor redefined",
        "PyCharm Community" => "Python IDE",
        "Bottles" => "Run Windows apps on Linux",
        "Boxes" => "Simple VM manager",
        "Steam" => "Gaming platform",
        "Lutris" => "Open gaming platform",
        "ProtonUp-Qt" => "Manage Proton versions",
        _ => "",
    }
}

fn admin_description(label: &str, script: &str) -> &'static str {
    match label {
        "Enable Bluetooth" => "Unblock and start Bluetooth service",
        "Disable Bluetooth" => "Block and stop Bluetooth service",
        "TLP (Laptops)" => "Install and enable laptop power tuning",
        "Powertop" => "Install power analysis tooling",
        "Update System" => "Upgrade packages and Flatpaks",
        "nala (rank mirrors) - Debian only" => "Install nala and rank Debian mirrors",
        "Stacer" => "Install system optimizer",
        "SWAP Fix" => "Set vm.swappiness to 10",
        "Fastfetch" => "Install system info utility",
        _ => match script {
            "enable-bluetooth.sh" => "Enable Bluetooth",
            "disable-bluetooth.sh" => "Disable Bluetooth",
            "install-tlp.sh" => "Power management setup",
            "install-powertop.sh" => "Power usage analysis",
            "update-system.sh" => "System package upgrade",
            _ => "Run maintenance helper",
        },
    }
}

fn icon_key(label: &str) -> &'static str {
    match label {
        "Audacity" | "audacity" => "audacity",
        "Bottles" | "bottles" => "bottles",
        "Boxes" | "gnome-boxes" => "boxes",
        "Brave Browser" | "brave" => "brave",
        "Brave Origin" | "brave-origin" => "brave-origin",
        "Discord" | "discord" => "discord",
        "Firefox" | "firefox" => "firefox",
        "GIMP" | "gimp" => "gimp",
        "Google Chrome" | "google-chrome" => "google-chrome",
        "LocalSend" | "localsend" => "localsend",
        "Lutris" | "lutris" => "lutris",
        "Microsoft Edge" | "microsoft-edge" => "microsoft-edge",
        "OBS Studio" | "obs" => "obs-studio",
        "OnlyOffice" | "onlyoffice-desktopeditors" => "onlyoffice",
        "Opera" | "opera" => "opera",
        "ProtonUp-Qt" | "protonup-qt" => "protonup-qt",
        "PyCharm Community" | "pycharm-community" => "pycharm-community",
        "Signal" | "signal-desktop" => "signal",
        "Slack" | "slack" => "slack",
        "Steam" | "steam" => "steam",
        "Thunderbird" | "thunderbird" => "thunderbird",
        "Visual Studio Code" | "code" => "visual-studio-code",
        "Vivaldi" | "vivaldi" => "vivaldi",
        "VLC" | "vlc" => "vlc",
        "Zen" | "zen-browser" => "zen",
        _ => "",
    }
}

fn elide(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }

    let mut short = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    short.push('…');
    short
}

fn page_subtitle(page: Page) -> &'static str {
    match page {
        Page::Install => "Select applications to install on your system.",
        Page::Remove => "Choose applications to remove from your system.",
        Page::Administration => "Run common Linux maintenance and power tasks.",
        Page::SystemInfo => "Inspect local system and runtime details.",
    }
}
