use std::{
    collections::{BTreeMap, HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use eframe::egui::{
    self, Align, Align2, Button, CentralPanel, Color32, ColorImage, Context, FontData,
    FontDefinitions, FontFamily, FontId, Frame, Grid, Key, Layout, Painter, Panel, Pos2, Rect,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, TextEdit, TextureHandle, TextureOptions, Ui,
    Vec2, pos2, vec2,
};
use serde::Deserialize;

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

#[derive(Clone, Debug, Deserialize)]
struct CsvAppEntry {
    #[serde(rename = "Category")]
    category: String,
    #[serde(rename = "Label")]
    label: String,
    #[serde(rename = "Package Name")]
    package_name: String,
    #[serde(rename = "Flatpak ID")]
    flatpak_id: String,
    #[serde(rename = "Exec Name")]
    exec_name: String,
    #[serde(rename = "Notes")]
    notes: String,
}

#[derive(Clone, Debug)]
struct AppEntry {
    category: String,
    label: String,
    package_name: String,
    flatpak_id: String,
    exec_name: String,
    notes: String,
}

impl AppEntry {
    fn source_label(&self) -> &'static str {
        if self.flatpak_id.is_empty() {
            "native"
        } else if self.package_name.is_empty() {
            "flatpak"
        } else {
            "native + flatpak"
        }
    }

    fn command(&self, base_dir: &Path, action: &str) -> Vec<String> {
        vec![
            "bash".to_owned(),
            base_dir.join("main.sh").display().to_string(),
            "--label".to_owned(),
            self.label.clone(),
            "--package".to_owned(),
            self.package_name.clone(),
            "--flatpak".to_owned(),
            self.flatpak_id.clone(),
            "--exec".to_owned(),
            self.exec_name.clone(),
            action.to_owned(),
        ]
    }
}

#[derive(Clone, Debug)]
struct AdminTask {
    category: String,
    label: String,
    script: String,
}

impl AdminTask {
    fn command(&self, base_dir: &Path) -> Vec<String> {
        vec![
            "bash".to_owned(),
            base_dir.join(&self.script).display().to_string(),
        ]
    }
}

#[derive(Clone, Debug)]
struct Task {
    description: String,
    command: Vec<String>,
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
}

#[derive(Debug)]
enum RunnerMessage {
    Log(String),
    Done,
}

impl ToolboxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        apply_theme(&cc.egui_ctx);
        let icons = load_icons(&cc.egui_ctx);
        let distro_icons = load_distro_icons(&cc.egui_ctx);

        let base_dir = find_base_dir();
        let apps = load_apps(&base_dir);
        let distro_name = distro_name();
        let package_manager = detect_package_manager();

        Self {
            base_dir,
            page: Page::Install,
            apps,
            admin_tasks: admin_tasks(),
            install_selected: HashSet::new(),
            remove_selected: HashSet::new(),
            admin_selected: HashSet::new(),
            search: String::new(),
            category_filter: None,
            distro_name,
            package_manager,
            log: "Process logs will appear here...".to_owned(),
            password: String::new(),
            show_password_modal: false,
            is_running: false,
            tx: None,
            rx: None,
            log_revision: 0,
            icons,
            distro_icons,
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

    fn selected_tasks(&self) -> Vec<Task> {
        let mut tasks = Vec::new();

        for index in &self.install_selected {
            if let Some(entry) = self.apps.get(*index) {
                tasks.push(Task {
                    description: format!("Installing {}", entry.label),
                    command: entry.command(&self.base_dir, "install"),
                });
            }
        }

        for index in &self.remove_selected {
            if let Some(entry) = self.apps.get(*index) {
                tasks.push(Task {
                    description: format!("Removing {}", entry.label),
                    command: entry.command(&self.base_dir, "remove"),
                });
            }
        }

        for index in &self.admin_selected {
            if let Some(task) = self.admin_tasks.get(*index) {
                tasks.push(Task {
                    description: format!("Running {}", task.label),
                    command: task.command(&self.base_dir),
                });
            }
        }

        tasks
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
        if let Some(category) = &self.category_filter {
            if &entry.category != category {
                return false;
            }
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
        let tasks = self.selected_tasks();
        if tasks.is_empty() || self.is_running {
            return;
        }

        let (tx, rx) = mpsc::channel();
        let password = self.password.clone();
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
            self.password.clear();
            self.tx = None;
            self.rx = None;
            self.log.push_str("\n\n[DONE] All tasks finished.");
            self.log_revision = self.log_revision.saturating_add(1);
        }
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            toolbox_icon(ui, 26.0);
            ui.label(
                RichText::new("Toolbox")
                    .font(FontId::proportional(17.0))
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
            if nav_button(ui, page, self.page == page).clicked() {
                self.page = page;
                self.category_filter = None;
                self.search.clear();
            }
            ui.add_space(4.0);
        }

        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            ui.add_space(12.0);
            let about = nav_aux_button(ui, "About", "i");
            if about.clicked() {
                self.log.push_str("\n[INFO] The Linux IT Guy Toolbox");
                self.log_revision = self.log_revision.saturating_add(1);
            }
            ui.add_space(4.0);
            let settings = nav_aux_button(ui, "Settings", "*");
            if settings.clicked() {
                self.log
                    .push_str("\n[INFO] Settings are not available yet.");
                self.log_revision = self.log_revision.saturating_add(1);
            }
            ui.add_space(20.0);
            ui.label(
                RichText::new(format!("Package manager: {}", self.package_manager))
                    .color(subtle_text()),
            );
            ui.label(RichText::new(&self.distro_name).strong());
        });
    }

    fn header(&mut self, ui: &mut Ui) {
        ui.set_min_height(76.0);
        ui.horizontal(|ui| {
            page_icon(ui, self.page, 46.0, true);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(self.page.title())
                        .font(FontId::proportional(25.0))
                        .strong(),
                );
                ui.label(RichText::new(page_subtitle(self.page)).color(subtle_text()));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for label in ["NixOS", "Fedora", "Debian", "Arch"] {
                    let selected = distro_selected(label, &self.package_manager, &self.distro_name);
                    chip(
                        ui,
                        label,
                        selected,
                        self.distro_icons.get(distro_key(label)),
                    );
                }
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Supported Distros:")
                        .color(subtle_text())
                        .strong(),
                );
            });
        });
    }

    fn install_remove_page(&mut self, ui: &mut Ui, install: bool) {
        self.toolbar(ui, install);
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
                        section_header(ui, &category, indexes.len());
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
        ui.horizontal(|ui| {
            category_icon(ui, title);
            ui.label(
                RichText::new(title)
                    .font(FontId::proportional(15.0))
                    .strong(),
            );
            let (rect, _) = ui.allocate_exact_size(vec2(28.0, 20.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, 10.0, Color32::from_rgb(43, 49, 56));
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("{count}"),
                FontId::proportional(11.0),
                subtle_text(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(1.0);
            });
        });
        ui.add_space(6.0);
    }

    fn toolbar(&mut self, ui: &mut Ui, install: bool) {
        toolbar_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                let actions_width = 420.0;
                let category_width = (ui.available_width() - actions_width).max(360.0);
                ui.allocate_ui_with_layout(
                    vec2(category_width, 34.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let categories = self.categories();
                        if filter_chip(ui, "All", self.category_filter.is_none()).clicked() {
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
                            )
                            .clicked()
                            {
                                self.category_filter = Some(category);
                            }
                        }
                    },
                );

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if flat_text_button(ui, "Clear", subtle_text(), 60.0).clicked() {
                        self.select_visible_apps(false);
                    }
                    if flat_text_button(ui, "Select All", accent_blue(), 92.0).clicked() {
                        self.select_visible_apps(true);
                    }
                    search_field(
                        ui,
                        &mut self.search,
                        if install {
                            "Filter apps..."
                        } else {
                            "Filter apps..."
                        },
                    );
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

        let (rect, response) = ui.allocate_exact_size(vec2(width, 42.0), Sense::click());
        let hovered = response.hovered();
        paint_row_background(ui.painter(), rect, hovered, selected);

        let mut check_rect = Rect::from_min_size(rect.min + vec2(10.0, 13.0), vec2(16.0, 16.0));
        check_rect = check_rect.expand(0.5);
        paint_checkbox(ui.painter(), check_rect, selected);

        let icon_rect = Rect::from_min_size(rect.min + vec2(34.0, 8.0), vec2(26.0, 26.0));
        paint_app_icon(ui.painter(), icon_rect, &label, icon.as_ref());

        let label_text = elide(&label, 19);

        ui.painter().text(
            pos2(rect.min.x + 70.0, rect.center().y - 1.0),
            Align2::LEFT_CENTER,
            label_text,
            FontId::proportional(14.0),
            primary_text(),
        );

        let badge_center_x = rect.min.x + width * 0.42;
        paint_badge(ui.painter(), pos2(badge_center_x, rect.center().y), source);

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
            subtle_text(),
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
                        section_header(ui, &category, indexes.len());
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
        ui.horizontal(|ui| {
            category_icon(ui, title);
            ui.label(
                RichText::new(title)
                    .font(FontId::proportional(15.0))
                    .strong(),
            );
            let (rect, _) = ui.allocate_exact_size(vec2(28.0, 20.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, 10.0, Color32::from_rgb(43, 49, 56));
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("{count}"),
                FontId::proportional(11.0),
                subtle_text(),
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_sized(
                        [68.0, 32.0],
                        Button::new(RichText::new("Clear").color(subtle_text())),
                    )
                    .clicked()
                {
                    self.admin_selected.clear();
                }
                if ui
                    .add_sized(
                        [92.0, 32.0],
                        Button::new(RichText::new("Select All").color(accent_blue())),
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

        let (rect, response) = ui.allocate_exact_size(vec2(width, 42.0), Sense::click());
        paint_row_background(ui.painter(), rect, response.hovered(), selected);

        let mut check_rect = Rect::from_min_size(rect.min + vec2(10.0, 13.0), vec2(16.0, 16.0));
        check_rect = check_rect.expand(0.5);
        paint_checkbox(ui.painter(), check_rect, selected);

        let icon_rect = Rect::from_min_size(rect.min + vec2(34.0, 8.0), vec2(26.0, 26.0));
        paint_admin_icon(ui.painter(), icon_rect, &label);

        ui.painter().text(
            pos2(rect.min.x + 70.0, rect.center().y - 1.0),
            Align2::LEFT_CENTER,
            elide(&label, 24),
            FontId::proportional(14.0),
            primary_text(),
        );

        ui.painter().text(
            pos2(rect.min.x + width * 0.58, rect.center().y),
            Align2::LEFT_CENTER,
            admin_description(&label, &script),
            FontId::proportional(11.5),
            subtle_text(),
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
            ("Host", command_output("hostname")),
            ("Kernel", command_output("uname -r")),
            ("Uptime", uptime()),
            (
                "Shell",
                env::var("SHELL").unwrap_or_else(|_| "Unknown".to_owned()),
            ),
            (
                "DE/WM",
                env::var("XDG_CURRENT_DESKTOP")
                    .or_else(|_| env::var("DESKTOP_SESSION"))
                    .unwrap_or_else(|_| "Unknown".to_owned()),
            ),
            ("Package Manager", self.package_manager.clone()),
            ("App Directory", self.base_dir.display().to_string()),
        ];

        Grid::new("system_info_grid")
            .num_columns(2)
            .spacing(Vec2::new(18.0, 12.0))
            .show(ui, |ui| {
                for (label, value) in rows {
                    ui.label(RichText::new(label).color(subtle_text()).strong());
                    panel_frame().show(ui, |ui| {
                        ui.label(value);
                    });
                    ui.end_row();
                }
            });
    }

    fn bottom_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        let (install, remove, admin) = self.selected_counts();
        summary_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                let summary = if self.total_selected() == 0 {
                    "0 apps selected".to_owned()
                } else {
                    format!("{} selected", self.total_selected(),)
                };
                ui.label(RichText::new(summary).strong());
                ui.separator();
                ui.label(
                    RichText::new(format!("{} tasks", self.total_selected())).color(subtle_text()),
                );
                ui.separator();
                ui.label(
                    RichText::new(format!(
                        "{} install  /  {} remove  /  {} admin",
                        install, remove, admin
                    ))
                    .color(subtle_text()),
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
                            .strong(),
                        ),
                    );
                    if run.clicked() {
                        self.show_password_modal = true;
                    }
                    if ui
                        .add_enabled(!self.is_running, Button::new("Clear Selection"))
                        .clicked()
                    {
                        self.clear_selection();
                    }
                });
            });
        });

        ui.add_space(8.0);
        log_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                small_terminal_icon(ui);
                ui.label(RichText::new("Process Log").strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .button(RichText::new("Clear Log").color(subtle_text()))
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
                .fill(Color32::from_rgb(8, 10, 11))
                .stroke(Stroke::new(1.0, Color32::from_rgb(42, 48, 52)))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    if showing_placeholder {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(log_height - 18.0);
                        ui.label(
                            RichText::new(&self.log)
                                .font(FontId::monospace(12.5))
                                .color(Color32::from_rgb(87, 225, 116)),
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
                                        .color(Color32::from_rgb(87, 225, 116)),
                                );
                                ui.scroll_to_cursor(Some(Align::BOTTOM));
                            });
                    }
                });
        });

        if self.show_password_modal {
            egui::Window::new("Sudo Authentication")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Enter your sudo password to run the selected tasks.");
                    ui.add_space(8.0);
                    let response = ui.add(
                        TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(280.0),
                    );
                    if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                        self.show_password_modal = false;
                        self.run_selected(ctx);
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.password.clear();
                            self.show_password_modal = false;
                        }
                        if ui
                            .add_enabled(!self.password.is_empty(), Button::new("Run"))
                            .clicked()
                        {
                            self.show_password_modal = false;
                            self.run_selected(ctx);
                        }
                    });
                });
        }
    }
}

impl eframe::App for ToolboxApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.drain_runner_messages();

        Panel::left("sidebar")
            .resizable(false)
            .exact_size(252.0)
            .frame(
                Frame::new()
                    .fill(Color32::from_rgb(21, 25, 30))
                    .stroke(Stroke::new(1.0, border()))
                    .inner_margin(14.0),
            )
            .show_inside(ui, |ui| self.sidebar(ui));

        Panel::top("header")
            .frame(
                Frame::new()
                    .fill(background())
                    .stroke(Stroke::new(1.0, border()))
                    .inner_margin(egui::Margin::symmetric(16, 14)),
            )
            .show_inside(ui, |ui| self.header(ui));

        Panel::bottom("bottom")
            .exact_size(260.0)
            .frame(
                Frame::new()
                    .fill(background())
                    .stroke(Stroke::new(1.0, border()))
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show_inside(ui, |ui| {
                let ctx = ui.ctx().clone();
                self.bottom_bar(ui, &ctx);
            });

        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(background())
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

fn load_icons(ctx: &Context) -> HashMap<&'static str, TextureHandle> {
    let mut icons = HashMap::new();
    for (key, bytes) in [
        (
            "audacity",
            include_bytes!("../assets/icons/audacity.png").as_slice(),
        ),
        (
            "bottles",
            include_bytes!("../assets/icons/bottles.png").as_slice(),
        ),
        (
            "boxes",
            include_bytes!("../assets/icons/boxes.png").as_slice(),
        ),
        (
            "brave",
            include_bytes!("../assets/icons/brave.png").as_slice(),
        ),
        (
            "discord",
            include_bytes!("../assets/icons/discord.png").as_slice(),
        ),
        (
            "firefox",
            include_bytes!("../assets/icons/firefox.png").as_slice(),
        ),
        (
            "gimp",
            include_bytes!("../assets/icons/gimp.png").as_slice(),
        ),
        (
            "google-chrome",
            include_bytes!("../assets/icons/google-chrome.png").as_slice(),
        ),
        (
            "localsend",
            include_bytes!("../assets/icons/localsend.png").as_slice(),
        ),
        (
            "lutris",
            include_bytes!("../assets/icons/lutris.png").as_slice(),
        ),
        (
            "microsoft-edge",
            include_bytes!("../assets/icons/microsoft-edge.png").as_slice(),
        ),
        (
            "obs-studio",
            include_bytes!("../assets/icons/obs-studio.png").as_slice(),
        ),
        (
            "onlyoffice",
            include_bytes!("../assets/icons/onlyoffice.png").as_slice(),
        ),
        (
            "opera",
            include_bytes!("../assets/icons/opera.png").as_slice(),
        ),
        (
            "protonup-qt",
            include_bytes!("../assets/icons/protonup-qt.png").as_slice(),
        ),
        (
            "pycharm-community",
            include_bytes!("../assets/icons/pycharm-community.png").as_slice(),
        ),
        (
            "signal",
            include_bytes!("../assets/icons/signal.png").as_slice(),
        ),
        (
            "slack",
            include_bytes!("../assets/icons/slack.png").as_slice(),
        ),
        (
            "steam",
            include_bytes!("../assets/icons/steam.png").as_slice(),
        ),
        (
            "thunderbird",
            include_bytes!("../assets/icons/thunderbird.png").as_slice(),
        ),
        (
            "visual-studio-code",
            include_bytes!("../assets/icons/visual-studio-code.png").as_slice(),
        ),
        (
            "vivaldi",
            include_bytes!("../assets/icons/vivaldi.png").as_slice(),
        ),
        ("vlc", include_bytes!("../assets/icons/vlc.png").as_slice()),
        ("zen", include_bytes!("../assets/icons/zen.png").as_slice()),
    ] {
        if let Ok(image) = image::load_from_memory(bytes) {
            let rgba = image.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            icons.insert(
                key,
                ctx.load_texture(key, color_image, TextureOptions::LINEAR),
            );
        }
    }
    icons
}

fn load_distro_icons(ctx: &Context) -> HashMap<&'static str, TextureHandle> {
    let mut icons = HashMap::new();
    for (key, svg) in [
        (
            "arch",
            include_bytes!("../assets/distros/arch.svg").as_slice(),
        ),
        (
            "debian",
            include_bytes!("../assets/distros/debian.svg").as_slice(),
        ),
        (
            "fedora",
            include_bytes!("../assets/distros/fedora.svg").as_slice(),
        ),
        (
            "nixos",
            include_bytes!("../assets/distros/nixos.svg").as_slice(),
        ),
    ] {
        if let Some(color_image) = rasterize_svg(svg, 64) {
            icons.insert(
                key,
                ctx.load_texture(format!("distro-{key}"), color_image, TextureOptions::LINEAR),
            );
        }
    }
    icons
}

fn rasterize_svg(svg: &[u8], size: u32) -> Option<ColorImage> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg, &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let tree_size = tree.size();
    let scale = (size as f32 / tree_size.width()).min(size as f32 / tree_size.height());
    let tx = (size as f32 - tree_size.width() * scale) / 2.0;
    let ty = (size as f32 - tree_size.height() * scale) / 2.0;
    let transform = tiny_skia::Transform::from_translate(tx, ty).pre_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Some(ColorImage::from_rgba_unmultiplied(
        [size as usize, size as usize],
        pixmap.data(),
    ))
}

fn apply_theme(ctx: &Context) {
    let mut style = (*ctx.global_style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(13.0, 7.0);
    style.spacing.interact_size = Vec2::new(36.0, 32.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.window_fill = background();
    style.visuals.panel_fill = background();
    style.visuals.override_text_color = Some(primary_text());
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, primary_text());
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(32, 38, 44);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 50, 58);
    style.visuals.widgets.active.bg_fill = accent_blue();
    style.visuals.selection.bg_fill = accent_blue();
    ctx.set_global_style(style);
}

fn nav_button(ui: &mut Ui, page: Page, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
    let fill = if selected {
        Color32::from_rgb(42, 69, 111)
    } else if response.hovered() {
        Color32::from_rgb(31, 38, 45)
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 7.0, fill);
    if selected {
        let marker = Rect::from_min_size(rect.min + vec2(0.0, 9.0), vec2(3.0, 26.0));
        ui.painter().rect_filled(marker, 2.0, accent_blue());
    }
    paint_page_symbol(
        ui.painter(),
        Rect::from_min_size(rect.min + vec2(18.0, 10.0), vec2(24.0, 24.0)),
        page,
        primary_text(),
    );
    ui.painter().text(
        pos2(rect.min.x + 56.0, rect.center().y),
        Align2::LEFT_CENTER,
        page.title(),
        FontId::proportional(15.0),
        primary_text(),
    );
    response
}

fn nav_aux_button(ui: &mut Ui, label: &str, symbol: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 7.0, Color32::from_rgb(31, 38, 45));
    }
    let icon = Rect::from_min_size(rect.min + vec2(20.0, 9.0), vec2(22.0, 22.0));
    ui.painter()
        .circle_stroke(icon.center(), 10.0, Stroke::new(1.5, subtle_text()));
    ui.painter().text(
        icon.center(),
        Align2::CENTER_CENTER,
        symbol,
        FontId::proportional(14.0),
        subtle_text(),
    );
    ui.painter().text(
        pos2(rect.min.x + 56.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        subtle_text(),
    );
    response
}

fn chip(ui: &mut Ui, label: &str, selected: bool, icon: Option<&TextureHandle>) {
    let (rect, _) = ui.allocate_exact_size(vec2(104.0, 34.0), Sense::hover());
    let fill = if selected {
        Color32::from_rgb(43, 76, 126)
    } else {
        Color32::from_rgb(31, 37, 43)
    };
    ui.painter().rect_filled(rect, 7.0, fill);
    ui.painter().rect_stroke(
        rect,
        7.0,
        Stroke::new(1.0, if selected { accent_blue() } else { border() }),
        StrokeKind::Inside,
    );
    distro_mark(
        ui.painter(),
        Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2(18.0, 18.0)),
        label,
        selected,
        icon,
    );
    ui.painter().text(
        pos2(rect.min.x + 38.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.0),
        primary_text(),
    );
}

fn filter_chip(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let width = (label.len() as f32 * 7.0 + 22.0).clamp(42.0, 104.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let fill = if selected {
        accent_teal()
    } else if response.hovered() {
        Color32::from_rgb(36, 43, 50)
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
            Color32::WHITE
        } else {
            subtle_text()
        },
    );
    response
}

fn flat_text_button(ui: &mut Ui, label: &str, color: Color32, width: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 5.0, Color32::from_rgb(35, 41, 47));
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

fn search_field(ui: &mut Ui, search: &mut String, hint: &str) {
    let search_fill = Color32::from_rgb(31, 35, 41);
    let response = Frame::new()
        .fill(search_fill)
        .stroke(Stroke::new(1.0, Color32::from_rgb(55, 63, 72)))
        .corner_radius(5.0)
        .inner_margin(egui::Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.set_min_width(260.0);
            ui.set_max_width(260.0);
            ui.horizontal(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(18.0, 32.0), Sense::hover());
                paint_search_icon(ui.painter(), icon_rect);
                ui.add_sized(
                    [220.0, 32.0],
                    TextEdit::singleline(search)
                        .hint_text(hint)
                        .font(FontId::proportional(13.0))
                        .desired_width(220.0)
                        .background_color(search_fill),
                );
            });
        })
        .response;

    if response.clicked() {
        response.request_focus();
    }
}

fn paint_search_icon(painter: &Painter, rect: Rect) {
    let center = pos2(rect.left() + 8.0, rect.center().y - 1.0);
    painter.circle_stroke(center, 5.0, Stroke::new(1.4, subtle_text()));
    painter.line_segment(
        [
            pos2(center.x + 4.0, center.y + 4.0),
            pos2(center.x + 9.0, center.y + 9.0),
        ],
        Stroke::new(1.4, subtle_text()),
    );
}

fn section_header(ui: &mut Ui, title: &str, count: usize) {
    ui.horizontal(|ui| {
        category_icon(ui, title);
        ui.label(
            RichText::new(title)
                .font(FontId::proportional(15.0))
                .strong(),
        );
        let (rect, _) = ui.allocate_exact_size(vec2(28.0, 20.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, 10.0, Color32::from_rgb(43, 49, 56));
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            format!("{count}"),
            FontId::proportional(11.0),
            subtle_text(),
        );
    });
    ui.add_space(6.0);
}

fn toolbar_frame() -> Frame {
    Frame::new()
        .fill(Color32::from_rgb(24, 29, 34))
        .stroke(Stroke::new(1.0, border()))
        .inner_margin(egui::Margin::symmetric(12, 7))
}

fn summary_frame() -> Frame {
    Frame::new()
        .fill(Color32::from_rgb(25, 30, 36))
        .stroke(Stroke::new(1.0, border()))
        .inner_margin(egui::Margin::symmetric(12, 8))
}

fn log_frame() -> Frame {
    Frame::new()
        .fill(Color32::from_rgb(20, 24, 29))
        .stroke(Stroke::new(1.0, border()))
        .inner_margin(egui::Margin::symmetric(10, 8))
}

fn panel_frame() -> Frame {
    Frame::new()
        .fill(panel())
        .stroke(Stroke::new(1.0, border()))
        .inner_margin(12.0)
}

fn paint_row_background(painter: &Painter, rect: Rect, hovered: bool, selected: bool) {
    let fill = if selected {
        Color32::from_rgb(36, 52, 72)
    } else if hovered {
        Color32::from_rgb(39, 45, 52)
    } else {
        Color32::from_rgb(31, 36, 42)
    };
    painter.rect_filled(rect, 4.0, fill);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, if selected { accent_blue() } else { border() }),
        StrokeKind::Inside,
    );
}

fn paint_checkbox(painter: &Painter, rect: Rect, selected: bool) {
    painter.rect_filled(
        rect,
        3.0,
        if selected {
            accent_blue()
        } else {
            Color32::from_rgb(29, 36, 43)
        },
    );
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(
            1.0,
            if selected {
                accent_blue_light()
            } else {
                Color32::from_rgb(90, 100, 108)
            },
        ),
        StrokeKind::Inside,
    );
    if selected {
        let a = pos2(rect.left() + 4.0, rect.center().y);
        let b = pos2(rect.left() + 7.0, rect.bottom() - 5.0);
        let c = pos2(rect.right() - 4.0, rect.top() + 5.0);
        painter.line_segment([a, b], Stroke::new(2.0, Color32::WHITE));
        painter.line_segment([b, c], Stroke::new(2.0, Color32::WHITE));
    }
}

fn paint_app_icon(painter: &Painter, rect: Rect, label: &str, icon: Option<&TextureHandle>) {
    if let Some(icon) = icon {
        painter.circle_filled(
            rect.center(),
            rect.width() / 2.0,
            Color32::from_rgb(24, 29, 34),
        );
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
        Stroke::new(1.0, Color32::from_white_alpha(80)),
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
        Stroke::new(1.0, Color32::from_white_alpha(80)),
    );

    let stroke = Stroke::new(1.7, Color32::WHITE);
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
                    Stroke::new(2.1, Color32::WHITE),
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

fn paint_badge(painter: &Painter, center: Pos2, label: &str) {
    let width = if label == "native + flatpak" {
        104.0
    } else {
        56.0
    };
    let rect = Rect::from_center_size(center, vec2(width, 22.0));
    let native = label.contains("native");
    let fill = if native {
        Color32::from_rgb(32, 72, 44)
    } else {
        Color32::from_rgb(32, 55, 91)
    };
    let stroke = if native {
        Color32::from_rgb(80, 145, 83)
    } else {
        Color32::from_rgb(76, 120, 189)
    };
    painter.rect_filled(rect, 5.0, fill);
    painter.rect_stroke(rect, 5.0, Stroke::new(1.0, stroke), StrokeKind::Inside);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(11.0),
        if native {
            Color32::from_rgb(151, 230, 143)
        } else {
            Color32::from_rgb(142, 184, 246)
        },
    );
}

fn toolbox_icon(ui: &mut Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let painter = ui.painter();
    let body = Rect::from_min_max(
        pos2(rect.left() + 3.0, rect.top() + 8.0),
        pos2(rect.right() - 3.0, rect.bottom() - 3.0),
    );
    painter.rect_stroke(
        body,
        3.0,
        Stroke::new(1.8, primary_text()),
        StrokeKind::Inside,
    );
    painter.line_segment(
        [
            pos2(body.left(), body.top() + 6.0),
            pos2(body.right(), body.top() + 6.0),
        ],
        Stroke::new(1.4, primary_text()),
    );
    painter.rect_stroke(
        Rect::from_min_size(
            pos2(rect.center().x - 5.0, rect.top() + 4.0),
            vec2(10.0, 6.0),
        ),
        2.0,
        Stroke::new(1.5, primary_text()),
        StrokeKind::Inside,
    );
}

fn page_icon(ui: &mut Ui, page: Page, size: f32, filled: bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    if filled {
        ui.painter()
            .circle_filled(rect.center(), size / 2.0, Color32::from_rgb(44, 75, 128));
    }
    paint_page_symbol(ui.painter(), rect.shrink(size * 0.24), page, primary_text());
}

fn paint_page_symbol(painter: &Painter, rect: Rect, page: Page, color: Color32) {
    let stroke = Stroke::new(1.8, color);
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

fn category_icon(ui: &mut Ui, title: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(22.0, 22.0), Sense::hover());
    let painter = ui.painter();
    let stroke = Stroke::new(1.4, subtle_text());
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
                subtle_text(),
            );
        }
        "Gaming" => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "pad",
                FontId::proportional(9.0),
                subtle_text(),
            );
        }
        _ => {
            painter.circle_stroke(rect.center(), 8.0, stroke);
        }
    }
}

fn small_terminal_icon(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
    ui.painter().rect_stroke(
        rect.shrink(2.0),
        2.0,
        Stroke::new(1.0, subtle_text()),
        StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ">",
        FontId::monospace(10.0),
        subtle_text(),
    );
}

fn distro_mark(
    painter: &Painter,
    rect: Rect,
    label: &str,
    selected: bool,
    icon: Option<&TextureHandle>,
) {
    if let Some(icon) = icon {
        painter.image(
            icon.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            if selected {
                Color32::WHITE
            } else {
                Color32::from_rgb(190, 199, 207)
            },
        );
        return;
    }

    let color = if selected {
        Color32::from_rgb(125, 179, 255)
    } else {
        subtle_text()
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
        "NixOS" => "nixos",
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
        "NixOS" => package_manager == "nix-env" || distro.contains("nixos"),
        "Fedora" => package_manager == "dnf" || distro.contains("fedora"),
        _ => false,
    }
}

fn app_color(label: &str) -> Color32 {
    match label {
        "Firefox" => Color32::from_rgb(238, 90, 47),
        "Brave Browser" => Color32::from_rgb(246, 85, 42),
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
        "Brave Browser" => "B",
        "Thunderbird" => "T",
        "ProtonUp-Qt" => "P",
        _ => label.get(0..1).unwrap_or("*"),
    }
}

fn app_description(label: &str) -> &'static str {
    match label {
        "Brave Browser" => "Privacy-focused browser",
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

fn background() -> Color32 {
    Color32::from_rgb(18, 24, 27)
}

fn panel() -> Color32 {
    Color32::from_rgb(30, 36, 42)
}

fn border() -> Color32 {
    Color32::from_rgb(49, 60, 67)
}

fn primary_text() -> Color32 {
    Color32::from_rgb(233, 238, 241)
}

fn subtle_text() -> Color32 {
    Color32::from_rgb(164, 174, 183)
}

fn accent_teal() -> Color32 {
    Color32::from_rgb(12, 145, 132)
}

fn accent_blue() -> Color32 {
    Color32::from_rgb(55, 105, 196)
}

fn accent_blue_light() -> Color32 {
    Color32::from_rgb(112, 161, 255)
}

fn page_subtitle(page: Page) -> &'static str {
    match page {
        Page::Install => "Select applications to install on your system.",
        Page::Remove => "Choose applications to remove from your system.",
        Page::Administration => "Run common Linux maintenance and power tasks.",
        Page::SystemInfo => "Inspect local system and runtime details.",
    }
}

fn find_base_dir() -> PathBuf {
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join("apps_config.csv").exists() {
                return dir.to_path_buf();
            }
        }
    }

    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn load_apps(base_dir: &Path) -> Vec<AppEntry> {
    let path = base_dir.join("apps_config.csv");
    let Ok(mut reader) = csv::Reader::from_path(path) else {
        return Vec::new();
    };

    reader
        .deserialize::<CsvAppEntry>()
        .filter_map(Result::ok)
        .filter(|entry| !entry.category.trim().is_empty() && !entry.label.trim().is_empty())
        .map(|entry| AppEntry {
            category: entry.category.trim().to_owned(),
            label: entry.label.trim().to_owned(),
            package_name: entry.package_name.trim().to_owned(),
            flatpak_id: entry.flatpak_id.trim().to_owned(),
            exec_name: entry.exec_name.trim().to_owned(),
            notes: entry.notes.trim().to_owned(),
        })
        .collect()
}

fn admin_tasks() -> Vec<AdminTask> {
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

fn run_tasks(tasks: Vec<Task>, password: String, tx: Sender<RunnerMessage>) {
    for task in tasks {
        let _ = tx.send(RunnerMessage::Log(format!(
            "[INFO] Starting: {}",
            task.description
        )));
        let _ = tx.send(RunnerMessage::Log(format!(
            "Command: sudo -S {}",
            task.command.join(" ")
        )));

        let Some((program, args)) = task.command.split_first() else {
            let _ = tx.send(RunnerMessage::Log("[ERROR] Empty command.".to_owned()));
            continue;
        };

        let output = Command::new("sudo")
            .arg("-S")
            .arg(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                if let Some(stdin) = child.stdin.as_mut() {
                    use std::io::Write;
                    stdin.write_all(format!("{password}\n").as_bytes())?;
                }
                child.wait_with_output()
            });

        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stdout.lines().chain(stderr.lines()) {
                    let _ = tx.send(RunnerMessage::Log(line.to_owned()));
                }
                if output.status.success() {
                    let _ = tx.send(RunnerMessage::Log(format!(
                        "[SUCCESS] {} completed successfully.",
                        task.description
                    )));
                } else {
                    let code = output
                        .status
                        .code()
                        .map_or_else(|| "unknown".to_owned(), |code| code.to_string());
                    let _ = tx.send(RunnerMessage::Log(format!(
                        "[ERROR] {} failed with return code {code}.",
                        task.description
                    )));
                }
            }
            Err(error) => {
                let _ = tx.send(RunnerMessage::Log(format!(
                    "[EXCEPTION] Failed to run {}: {error}",
                    task.description
                )));
            }
        }
    }

    let _ = tx.send(RunnerMessage::Done);
}

fn distro_name() -> String {
    let Ok(contents) = fs::read_to_string("/etc/os-release") else {
        return "Linux".to_owned();
    };

    let mut fallback = None;
    for line in contents.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return value.trim_matches('"').to_owned();
        }
        if let Some(value) = line.strip_prefix("NAME=") {
            fallback = Some(value.trim_matches('"').to_owned());
        }
    }
    fallback.unwrap_or_else(|| "Linux".to_owned())
}

fn detect_package_manager() -> String {
    for manager in ["apt-get", "pacman", "dnf", "nix-env"] {
        if Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {manager} >/dev/null 2>&1"))
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
        {
            return manager.to_owned();
        }
    }
    "unknown".to_owned()
}

fn command_output(command: &str) -> String {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return "Unknown".to_owned();
    };
    Command::new(program)
        .args(parts)
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "Unknown".to_owned())
}

fn uptime() -> String {
    let Ok(contents) = fs::read_to_string("/proc/uptime") else {
        return "Unknown".to_owned();
    };
    let Ok(seconds) = contents
        .split_whitespace()
        .next()
        .unwrap_or("0")
        .parse::<f64>()
    else {
        return "Unknown".to_owned();
    };

    let total = seconds as u64;
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;

    match (days, hours) {
        (0, 0) => format!("{minutes}m"),
        (0, _) => format!("{hours}h {minutes}m"),
        _ => format!("{days}d {hours}h {minutes}m"),
    }
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }

    output
}
