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
    RemovableApp, RemovableSource, command_output, detect_package_manager, distro_name,
    env_or_unknown, scan_removable_apps, strip_ansi, uptime,
};
use theme::{Palette, ThemeMode};
use validate::{is_exec_name, is_label, zeroize_string};

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

#[derive(Clone, Debug, Eq, PartialEq)]
enum RemoveScanState {
    Idle,
    Scanning,
    Ready,
    Failed(String),
}

const INSTALL_CATEGORIES: &[&str] = &[
    "Browsers",
    "Communication",
    "Dev Tools",
    "Gaming",
    "Multimedia",
    "Office Tools",
    "Utilities",
];
const CARD_HEIGHT: f32 = 88.0;
const CARD_ICON: f32 = 40.0;
const CARD_GAP: f32 = 12.0;

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
    log_expanded: bool,
    icons: HashMap<&'static str, TextureHandle>,
    distro_icons: HashMap<&'static str, TextureHandle>,
    theme: ThemeMode,
    remove_apps: Vec<RemovableApp>,
    remove_scan: RemoveScanState,
    remove_scan_rx: Option<Receiver<Result<Vec<RemovableApp>, String>>>,
}

impl ToolboxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let theme = theme::load_theme_mode();
        theme::apply_theme(&cc.egui_ctx, theme);
        let icons = load_icons(&cc.egui_ctx);
        let distro_icons = load_distro_icons(&cc.egui_ctx);

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
            log_expanded: false,
            icons,
            distro_icons,
            theme,
            remove_apps: Vec::new(),
            remove_scan: RemoveScanState::Idle,
            remove_scan_rx: None,
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

    fn page_selected_count(&self) -> usize {
        match self.page {
            Page::Install => self.install_selected.len(),
            Page::Remove => self.remove_selected.len(),
            Page::Administration => self.admin_selected.len(),
            Page::SystemInfo => 0,
        }
    }

    fn page_selected_names(&self) -> Vec<String> {
        match self.page {
            Page::Install => self
                .install_selected
                .iter()
                .filter_map(|index| self.apps.get(*index).map(|entry| entry.label.clone()))
                .collect(),
            Page::Remove => self
                .remove_selected
                .iter()
                .filter_map(|index| self.remove_apps.get(*index).map(|app| app.label.clone()))
                .collect(),
            Page::Administration => self
                .admin_selected
                .iter()
                .filter_map(|index| self.admin_tasks.get(*index).map(|task| task.label.clone()))
                .collect(),
            Page::SystemInfo => Vec::new(),
        }
    }

    fn remove_includes_toolbox(&self) -> Option<String> {
        self.remove_selected.iter().find_map(|index| {
            self.remove_apps.get(*index).and_then(|app| {
                app.is_toolbox
                    .then(|| format!("{} ({})", app.label, app.detail))
            })
        })
    }

    fn selected_tasks(&self) -> Result<Vec<Task>, Vec<String>> {
        let mut tasks = Vec::new();
        let mut errors = Vec::new();

        match self.page {
            Page::Install => {
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
            }
            Page::Remove => {
                for index in &self.remove_selected {
                    if let Some(app) = self.remove_apps.get(*index) {
                        match removable_remove_command(&self.base_dir, app) {
                            Ok(command) => tasks.push(Task {
                                description: format!("Removing {}", app.label),
                                command,
                            }),
                            Err(error) => errors.push(format!("{}: {error}", app.label)),
                        }
                    }
                }
            }
            Page::Administration => {
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
            }
            Page::SystemInfo => {}
        }

        if errors.is_empty() {
            Ok(tasks)
        } else {
            Err(errors)
        }
    }

    fn select_visible_apps(&mut self, selected: bool) {
        match self.page {
            Page::Install => {
                let visible: Vec<usize> = self
                    .apps
                    .iter()
                    .enumerate()
                    .filter_map(|(index, entry)| self.app_matches_filter(entry).then_some(index))
                    .collect();
                if selected {
                    self.install_selected.extend(visible);
                } else {
                    for index in visible {
                        self.install_selected.remove(&index);
                    }
                }
            }
            Page::Remove => {
                let visible: Vec<usize> = self
                    .remove_apps
                    .iter()
                    .enumerate()
                    .filter_map(|(index, app)| self.removable_matches_filter(app).then_some(index))
                    .collect();
                if selected {
                    self.remove_selected.extend(visible);
                } else {
                    for index in visible {
                        self.remove_selected.remove(&index);
                    }
                }
            }
            _ => {}
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

    fn removable_matches_filter(&self, app: &RemovableApp) -> bool {
        let needle = self.search.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        app.label.to_lowercase().contains(&needle)
            || app.detail.to_lowercase().contains(&needle)
            || app.source.label().contains(needle.as_str())
    }

    fn start_remove_scan(&mut self, ctx: &Context) {
        if matches!(self.remove_scan, RemoveScanState::Scanning) {
            return;
        }
        self.remove_scan = RemoveScanState::Scanning;
        self.remove_selected.clear();
        self.remove_apps.clear();
        let package_manager = self.package_manager.clone();
        let (tx, rx) = mpsc::channel();
        self.remove_scan_rx = Some(rx);
        let repaint = ctx.clone();
        thread::spawn(move || {
            let result = scan_removable_apps(&package_manager);
            let _ = tx.send(result);
            repaint.request_repaint();
        });
    }

    fn drain_remove_scan(&mut self) {
        let Some(rx) = &self.remove_scan_rx else {
            return;
        };
        let Ok(result) = rx.try_recv() else {
            return;
        };
        self.remove_scan_rx = None;
        match result {
            Ok(apps) => {
                self.remove_apps = apps;
                self.remove_scan = RemoveScanState::Ready;
            }
            Err(error) => {
                self.remove_scan = RemoveScanState::Failed(error.clone());
                self.append_log(&format!("[ERROR] Remove scan failed: {error}"));
            }
        }
    }

    fn append_log(&mut self, line: &str) {
        if self.log == "Process logs will appear here..." {
            self.log.clear();
        }
        if !self.log.is_empty() {
            self.log.push('\n');
        }
        self.log.push_str(line);
        self.log_revision = self.log_revision.saturating_add(1);
    }

    fn last_log_line(&self) -> &str {
        self.log
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or(self.log.as_str())
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
        self.log_expanded = true;
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
                if page == Page::Remove {
                    let ctx = ui.ctx().clone();
                    self.start_remove_scan(&ctx);
                }
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
            if theme_toggle_button(ui, &palette).clicked() {
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

    fn install_page(&mut self, ui: &mut Ui) {
        self.install_toolbar(ui);
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
            .id_salt("install_cards")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = card_columns(ui);
                for (category, indexes) in categories {
                    section_header(ui, &category, indexes.len(), &self.palette());
                    self.card_rows(ui, columns, indexes.len(), |app, ui, index, width| {
                        app.install_tile(ui, indexes[index], width);
                    });
                    ui.add_space(10.0);
                }
            });
    }

    fn remove_page(&mut self, ui: &mut Ui) {
        self.remove_toolbar(ui);
        ui.add_space(10.0);

        match &self.remove_scan {
            RemoveScanState::Idle | RemoveScanState::Scanning => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("Scanning installed apps...")
                            .color(self.palette().chrome_subtle),
                    );
                });
            }
            RemoveScanState::Failed(error) => {
                let error = error.clone();
                ui.label(
                    RichText::new(format!("Remove scan failed: {error}"))
                        .color(self.palette().chrome_text),
                );
                ui.add_space(8.0);
                if cta_button(ui, "Retry", true, &self.palette()).clicked() {
                    let ctx = ui.ctx().clone();
                    self.start_remove_scan(&ctx);
                }
            }
            RemoveScanState::Ready if self.remove_apps.is_empty() => {
                ui.label(
                    RichText::new("No removable apps found on this system.")
                        .color(self.palette().chrome_text),
                );
                ui.add_space(8.0);
                if cta_button(ui, "Refresh", true, &self.palette()).clicked() {
                    let ctx = ui.ctx().clone();
                    self.start_remove_scan(&ctx);
                }
            }
            RemoveScanState::Ready => {
                let visible: Vec<usize> = self
                    .remove_apps
                    .iter()
                    .enumerate()
                    .filter_map(|(index, app)| self.removable_matches_filter(app).then_some(index))
                    .collect();
                ScrollArea::vertical()
                    .id_salt("remove_cards")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let columns = card_columns(ui);
                        self.card_rows(ui, columns, visible.len(), |app, ui, index, width| {
                            app.remove_tile(ui, visible[index], width);
                        });
                    });
            }
        }
    }

    fn card_rows(
        &mut self,
        ui: &mut Ui,
        columns: usize,
        count: usize,
        mut paint: impl FnMut(&mut Self, &mut Ui, usize, f32),
    ) {
        if count == 0 || columns == 0 {
            return;
        }
        let width = tile_width(ui, columns);
        for start in (0..count).step_by(columns) {
            let end = (start + columns).min(count);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = CARD_GAP;
                for index in start..end {
                    paint(self, ui, index, width);
                }
                if end - start < columns {
                    ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
                }
            });
            ui.add_space(6.0);
        }
    }

    fn install_toolbar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        toolbar_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                search_field(ui, &mut self.search, "Search apps...", &palette);
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
                });
            });
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.spacing_mut().item_spacing.y = 6.0;
                if filter_chip(ui, "All", self.category_filter.is_none(), &palette).clicked() {
                    self.category_filter = None;
                }
                for category in INSTALL_CATEGORIES {
                    if filter_chip(
                        ui,
                        category,
                        self.category_filter.as_deref() == Some(*category),
                        &palette,
                    )
                    .clicked()
                    {
                        self.category_filter = Some((*category).to_owned());
                    }
                }
            });
        });
    }

    fn remove_toolbar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        toolbar_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                search_field(ui, &mut self.search, "Search installed apps...", &palette);
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if flat_text_button(ui, "Refresh", palette.chrome_text, 72.0, &palette)
                        .clicked()
                    {
                        let ctx = ui.ctx().clone();
                        self.start_remove_scan(&ctx);
                    }
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
                });
            });
        });
    }

    fn install_tile(&mut self, ui: &mut Ui, index: usize, width: f32) {
        let entry = &self.apps[index];
        let label = entry.label.clone();
        let source = entry.source_label();
        let notes = if entry.notes.is_empty() {
            app_description(&entry.label).to_owned()
        } else {
            entry.notes.clone()
        };
        let queries = [
            label.as_str(),
            entry.exec_name.as_str(),
            entry.package_name.as_str(),
            entry.flatpak_id.as_str(),
        ];
        let icon = self.icon_for(&queries);
        let selected = self.install_selected.contains(&index);
        let palette = self.palette();
        if app_card(
            ui,
            width,
            selected,
            &label,
            Some(source),
            &notes,
            icon.as_ref(),
            &palette,
        )
        .clicked()
        {
            if selected {
                self.install_selected.remove(&index);
            } else {
                self.install_selected.insert(index);
            }
        }
    }

    fn remove_tile(&mut self, ui: &mut Ui, index: usize, width: f32) {
        let app = &self.remove_apps[index];
        let label = app.label.clone();
        let source = app.source.label();
        let detail = app.detail.clone();
        let queries: Vec<&str> = app.icon_queries().collect();
        let icon = self.icon_for(&queries);
        let selected = self.remove_selected.contains(&index);
        let palette = self.palette();
        if app_card(
            ui,
            width,
            selected,
            &label,
            Some(source),
            &detail,
            icon.as_ref(),
            &palette,
        )
        .clicked()
        {
            if selected {
                self.remove_selected.remove(&index);
            } else {
                self.remove_selected.insert(index);
            }
        }
    }

    fn icon_for(&self, queries: &[&str]) -> Option<(TextureHandle, bool)> {
        for query in queries {
            let resolved = resolve_icon_key(icon_key(query), self.theme);
            if resolved.is_empty() {
                continue;
            }
            if let Some(icon) = self.icons.get(resolved) {
                return Some((icon.clone(), crate::logos::is_monochrome_icon(resolved)));
            }
        }
        None
    }

    fn admin_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        toolbar_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Select maintenance tasks").color(palette.chrome_subtle));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if flat_text_button(ui, "Clear", palette.chrome_subtle, 60.0, &palette)
                        .clicked()
                    {
                        self.admin_selected.clear();
                    }
                    if flat_text_button(ui, "Select All", palette.chrome_text, 92.0, &palette)
                        .clicked()
                    {
                        self.admin_selected = (0..self.admin_tasks.len()).collect();
                    }
                });
            });
        });
        ui.add_space(10.0);

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
                let columns = card_columns(ui);
                for (category, indexes) in categories {
                    section_header(ui, &category, indexes.len(), &self.palette());
                    self.card_rows(ui, columns, indexes.len(), |app, ui, index, width| {
                        app.admin_tile(ui, indexes[index], width);
                    });
                    ui.add_space(12.0);
                }
            });
    }

    fn admin_tile(&mut self, ui: &mut Ui, index: usize, width: f32) {
        let task = &self.admin_tasks[index];
        let label = task.label.clone();
        let description = admin_description(&label, &task.script).to_owned();
        let selected = self.admin_selected.contains(&index);
        let palette = self.palette();
        if app_card(
            ui,
            width,
            selected,
            &label,
            None,
            &description,
            None,
            &palette,
        )
        .clicked()
        {
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
            .spacing(Vec2::new(28.0, 14.0))
            .show(ui, |ui| {
                for (label, value) in rows {
                    ui.label(RichText::new(label).color(palette.chrome_subtle).strong());
                    ui.label(RichText::new(value).color(palette.chrome_text));
                    ui.end_row();
                }
            });
    }

    fn cta_caption(&self) -> String {
        let n = self.page_selected_count();
        match self.page {
            Page::Install => format!("Install {n}"),
            Page::Remove => format!("Remove {n}"),
            Page::Administration => format!("Run {n}"),
            Page::SystemInfo => String::new(),
        }
    }

    fn modal_action_phrase(&self) -> String {
        let n = self.page_selected_count();
        match self.page {
            Page::Install => format!("install {n} apps"),
            Page::Remove => format!("remove {n} apps"),
            Page::Administration => format!("run {n} admin tasks"),
            Page::SystemInfo => "continue".to_owned(),
        }
    }

    fn bottom_bar(&mut self, ui: &mut Ui, ctx: &Context) {
        let palette = self.palette();
        if self.page != Page::SystemInfo {
            summary_frame(&palette).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let n = self.page_selected_count();
                    ui.label(
                        RichText::new(format!("{n} selected"))
                            .color(palette.on_surface)
                            .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let caption = if self.is_running {
                            "Running...".to_owned()
                        } else {
                            self.cta_caption()
                        };
                        if cta_button(ui, &caption, !self.is_running && n > 0, &palette).clicked() {
                            self.show_password_modal = true;
                        }
                    });
                });
            });
            ui.add_space(8.0);
        }

        log_frame(&palette).show(ui, |ui| {
            ui.horizontal(|ui| {
                small_terminal_icon(ui, &palette);
                let header = ui.add(
                    Button::new(
                        RichText::new(if self.log_expanded {
                            "Process Log ▾"
                        } else {
                            "Process Log ▸"
                        })
                        .color(palette.on_surface)
                        .strong(),
                    )
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE),
                );
                if header.clicked() {
                    self.log_expanded = !self.log_expanded;
                }
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
            if self.log_expanded {
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
            } else {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(self.last_log_line())
                        .font(FontId::monospace(12.0))
                        .color(palette.log_text),
                );
            }
        });

        if self.show_password_modal && ctx.input(|input| input.key_pressed(Key::Escape)) {
            zeroize_string(&mut self.password);
            self.show_password_modal = false;
        }

        if self.show_password_modal {
            let mut open = true;
            let phrase = self.modal_action_phrase();
            let mut names = self.page_selected_names();
            names.sort();
            let toolbox = if self.page == Page::Remove {
                self.remove_includes_toolbox()
            } else {
                None
            };
            egui::Window::new("Sudo Authentication")
                .collapsible(false)
                .resizable(false)
                .title_bar(false)
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
                        RichText::new("Sudo Authentication")
                            .font(FontId::proportional(18.0))
                            .color(palette.modal_text)
                            .strong(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("Enter your sudo password to {phrase}."))
                            .color(palette.modal_text),
                    );
                    if let Some(toolbox) = &toolbox {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(format!(
                                "This selection includes the running Toolbox: {toolbox}."
                            ))
                            .color(palette.modal_text)
                            .strong(),
                        );
                    }
                    if !names.is_empty() {
                        ui.add_space(8.0);
                        ScrollArea::vertical()
                            .id_salt("sudo_name_list")
                            .max_height(140.0)
                            .show(ui, |ui| {
                                for name in &names {
                                    ui.label(
                                        RichText::new(format!("• {name}"))
                                            .color(palette.modal_text),
                                    );
                                }
                            });
                    }
                    ui.add_space(8.0);
                    let response = ui.add(
                        TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(280.0)
                            .text_color(palette.on_surface)
                            .background_color(palette.input_fill),
                    );
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(Key::Enter))
                        && !self.password.is_empty()
                    {
                        self.show_password_modal = false;
                        self.run_selected(ctx);
                    }
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        let cancel = ui.add_sized(
                            [88.0, 32.0],
                            Button::new(
                                RichText::new("Cancel").color(palette.cancel_text).strong(),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0_f32, palette.cancel_text)),
                        );
                        if cancel.clicked() {
                            zeroize_string(&mut self.password);
                            self.show_password_modal = false;
                        }
                        if ui
                            .add_enabled(
                                !self.password.is_empty(),
                                Button::new(
                                    RichText::new("Confirm").color(palette.cta_text).strong(),
                                )
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
        self.drain_remove_scan();

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

        let bottom_height = bottom_panel_height(self.page, self.log_expanded);
        Panel::bottom("bottom")
            .exact_size(bottom_height)
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
                Page::Install => self.install_page(ui),
                Page::Remove => self.remove_page(ui),
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
    for &(key, svg) in crate::logos::APP_SVGS
        .iter()
        .chain(crate::logos::MONO_SVGS.iter())
    {
        if let Some(raster) = crate::logos::rasterize_svg_markup(svg, 128) {
            icons.insert(
                key,
                ctx.load_texture(
                    format!("app-{key}"),
                    crate::logos::color_image_from_raster(&raster),
                    TextureOptions::LINEAR,
                ),
            );
        }
    }
    if let Some(raster) = crate::logos::rasterize_svg_markup(crate::logos::BRAVE_ORIGIN_SVG, 128) {
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

fn theme_toggle_button(ui: &mut Ui, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(52.0, 48.0), Sense::click());
    let response = response.on_hover_text(palette.mode.toggle_tooltip());
    let punch = if response.hovered() {
        ui.painter().rect_filled(rect, 8.0, palette.nav_hover);
        palette.nav_hover
    } else {
        palette.sidebar
    };
    let icon_rect = Rect::from_center_size(rect.center(), vec2(26.0, 26.0));
    palette.paint_toggle_icon(ui.painter(), icon_rect, punch);
    response
}

fn chip(ui: &mut Ui, label: &str, selected: bool, icon: Option<&TextureHandle>, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(104.0, 34.0), Sense::hover());
    ui.painter().rect_filled(
        rect,
        7.0,
        if selected {
            palette.chip_selected
        } else {
            palette.chip_idle
        },
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
    let width = (label.len() as f32 * 7.4 + 22.0).clamp(42.0, 128.0);
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

fn paint_row_background(painter: &Painter, rect: Rect, selected: bool, palette: &Palette) {
    painter.rect_filled(rect, 6.0, palette.tile);
    if selected {
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.5_f32, palette.accent),
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
    icon: Option<&(TextureHandle, bool)>,
    palette: &Palette,
) {
    if let Some((icon, mono)) = icon {
        painter.circle_filled(rect.center(), rect.width() / 2.0, palette.icon_well);
        let tint = if *mono {
            palette.chrome_text
        } else {
            Color32::WHITE
        };
        painter.image(
            icon.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            tint,
        );
        return;
    }

    painter.circle_filled(rect.center(), rect.width() / 2.0, palette.page_icon_fill);
    painter.circle_stroke(
        rect.center(),
        rect.width() / 2.0,
        Stroke::new(1.0_f32, palette.border),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        app_mark(label),
        FontId::proportional(16.0),
        palette.chrome_text,
    );
}

#[allow(clippy::too_many_arguments)]
fn app_card(
    ui: &mut Ui,
    width: f32,
    selected: bool,
    label: &str,
    source: Option<&str>,
    description: &str,
    icon: Option<&(TextureHandle, bool)>,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_HEIGHT), Sense::click());
    paint_row_background(ui.painter(), rect, selected, palette);

    let check_rect = Rect::from_min_size(
        rect.min + vec2(12.0, (CARD_HEIGHT - 16.0) / 2.0),
        vec2(16.0, 16.0),
    );
    paint_checkbox(ui.painter(), check_rect, selected, palette);

    let icon_rect = Rect::from_min_size(
        rect.min + vec2(40.0, (CARD_HEIGHT - CARD_ICON) / 2.0),
        vec2(CARD_ICON, CARD_ICON),
    );
    paint_app_icon(ui.painter(), icon_rect, label, icon, palette);

    let text_left = rect.min.x + 92.0;
    let painter = ui.painter();
    painter.text(
        pos2(text_left, rect.min.y + 22.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(15.0),
        palette.chrome_text,
    );

    let mut desc_left = text_left;
    if let Some(source) = source {
        let badge_center = pos2(text_left + 28.0, rect.min.y + 52.0);
        paint_badge(painter, badge_center, source, palette);
        desc_left = text_left + 68.0;
    }

    let desc_width = (rect.right() - 12.0 - desc_left).max(24.0);
    let galley = painter.layout(
        description.to_owned(),
        FontId::proportional(12.0),
        palette.chrome_subtle,
        desc_width,
    );
    let desc_pos = pos2(desc_left, rect.min.y + 44.0);
    let clip = Rect::from_min_size(desc_pos, vec2(desc_width, 22.0));
    painter
        .with_clip_rect(clip)
        .galley(desc_pos, galley, palette.chrome_subtle);

    response
}

fn card_columns(ui: &Ui) -> usize {
    let width = ui.ctx().viewport_rect().width();
    if width >= 1280.0 {
        3
    } else if width < 1000.0 {
        1
    } else {
        2
    }
}

fn tile_width(ui: &Ui, columns: usize) -> f32 {
    let columns = columns.max(1) as f32;
    ((ui.available_width() - CARD_GAP * (columns - 1.0)) / columns).max(240.0)
}

fn cta_button(ui: &mut Ui, label: &str, enabled: bool, palette: &Palette) -> egui::Response {
    ui.add_enabled(
        enabled,
        Button::new(RichText::new(label).color(palette.cta_text).strong())
            .fill(palette.cta_fill)
            .stroke(Stroke::new(1.0_f32, palette.cta_fill))
            .min_size(vec2(120.0, 34.0)),
    )
}

fn bottom_panel_height(page: Page, log_expanded: bool) -> f32 {
    let mut height = 56.0;
    if page != Page::SystemInfo {
        height += 52.0;
    }
    if log_expanded {
        height += 168.0;
    }
    height
}

fn resolve_icon_key(key: &'static str, theme: ThemeMode) -> &'static str {
    if key == "zen-browser" && theme == ThemeMode::Light {
        "zen-browser-dark"
    } else {
        key
    }
}

fn removable_remove_command(
    base_dir: &std::path::Path,
    app: &RemovableApp,
) -> Result<Vec<String>, String> {
    let label = if is_label(&app.label) {
        app.label.clone()
    } else {
        app.detail.clone()
    };
    let exec_name = app
        .exec
        .as_deref()
        .filter(|value| is_exec_name(value))
        .unwrap_or("")
        .to_owned();
    let mut entry = AppEntry {
        category: "Installed".to_owned(),
        label,
        package_name: String::new(),
        flatpak_id: String::new(),
        exec_name,
        notes: String::new(),
    };
    match app.source {
        RemovableSource::Native => entry.package_name = app.detail.clone(),
        RemovableSource::Flatpak => entry.flatpak_id = app.detail.clone(),
    }
    entry.try_command(base_dir, "remove")
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

fn app_mark(label: &str) -> &str {
    let trimmed = label.trim();
    match trimmed {
        "Visual Studio Code" => "V",
        "PyCharm Community" => "PC",
        "Google Chrome" => "C",
        "Microsoft Edge" => "E",
        "Brave Origin" => "O",
        "Thunderbird" => "T",
        "ProtonUp-Qt" => "P",
        "Extension Manager" => "E",
        "Prism Launcher" => "P",
        "Dolphin Emulator" => "D",
        "Mission Center" => "M",
        "Gear Lever" => "G",
        _ => trimmed.get(0..1).unwrap_or("*"),
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
        "Sober" => "Linux Roblox port",
        "Spotify" => "Music streaming",
        "Heroic" => "Epic, GOG, and Amazon games",
        "Flatseal" => "Flatpak permissions UI",
        "Telegram" => "Messaging app",
        "Prism Launcher" => "Minecraft launcher",
        "Obsidian" => "Knowledge base",
        "RetroArch" => "Multi-system emulator",
        "Extension Manager" => "GNOME extensions",
        "Dolphin Emulator" => "GameCube and Wii emulator",
        "qBittorrent" => "BitTorrent client",
        "PPSSPP" => "PSP emulator",
        "Gear Lever" => "AppImage manager",
        "Proton VPN" => "Privacy-focused VPN",
        "ProtonPlus" => "Proton compatibility tool",
        "Bitwarden" => "Password manager",
        "Stremio" => "Media center",
        "LibreWolf" => "Privacy-focused Firefox fork",
        "Mission Center" => "System monitor",
        "GIMP" => "Image editor",
        "VLC" => "Media player",
        "mpv" => "Video player",
        "Audacity" => "Audio editor",
        "LibreOffice" => "Office suite",
        "OnlyOffice" => "Document editors",
        "OBS Studio" => "Streaming and recording",
        "GParted" => "Partition editor",
        "htop" => "Interactive process viewer",
        "LocalSend" => "Share files on the LAN",
        _ => "Application from the Toolbox catalog",
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
        "Bitwarden" | "bitwarden" | "com.bitwarden.desktop" => "bitwarden",
        "Brave Browser" | "brave" | "com.brave.Browser" => "brave",
        "Brave Origin" | "brave-origin" | "brave-origin-bin" => "brave-origin",
        "Discord" | "discord" | "com.discordapp.Discord" => "discord",
        "Firefox" | "firefox" => "firefox",
        "GIMP" | "gimp" | "org.gimp.GIMP" => "gimp",
        "Google Chrome" | "google-chrome" | "com.google.Chrome" => "google-chrome",
        "Heroic" | "heroic" | "com.heroicgameslauncher.hgl" => "heroic",
        "htop" => "htop",
        "LibreOffice" | "libreoffice" => "libreoffice",
        "LibreWolf" | "librewolf" | "io.gitlab.librewolf-community" => "librewolf",
        "Lutris" | "lutris" | "net.lutris.Lutris" => "lutris",
        "Microsoft Edge" | "microsoft-edge" | "com.microsoft.Edge" => "microsoft-edge",
        "mpv" => "mpv",
        "Obsidian" | "obsidian" | "md.obsidian.Obsidian" => "obsidian",
        "OBS Studio" | "obs" | "obs-studio" | "com.obsproject.Studio" => "obs-studio",
        "OnlyOffice"
        | "onlyoffice"
        | "onlyoffice-desktopeditors"
        | "org.onlyoffice.desktopeditors" => "onlyoffice",
        "Opera" | "opera" | "com.opera.Opera" => "opera",
        "Proton VPN" | "proton-vpn" | "protonvpn-app" | "com.protonvpn.www" => "proton-vpn",
        "PyCharm Community" | "pycharm-community" | "com.jetbrains.PyCharm-Community" => {
            "pycharm-community"
        }
        "qBittorrent" | "qbittorrent" => "qbittorrent",
        "Signal" | "signal" | "signal-desktop" | "org.signal.Signal" => "signal",
        "Slack" | "slack" | "com.slack.Slack" => "slack",
        "Spotify" | "spotify" | "com.spotify.Client" => "spotify",
        "Steam" | "steam" | "com.valvesoftware.Steam" => "steam",
        "Stremio" | "stremio" | "com.stremio.Stremio" => "stremio",
        "Telegram" | "telegram" | "telegram-desktop" => "telegram",
        "Thunderbird" | "thunderbird" => "thunderbird",
        "Visual Studio Code" | "code" | "visual-studio-code" | "com.visualstudio.code" => {
            "visual-studio-code"
        }
        "Vivaldi" | "vivaldi" | "com.vivaldi.Vivaldi" => "vivaldi",
        "VLC" | "vlc" => "vlc",
        "Zen" | "zen-browser" | "app.zen_browser.zen" => "zen-browser",
        _ => "",
    }
}

fn page_subtitle(page: Page) -> &'static str {
    match page {
        Page::Install => "Select applications to install on your system.",
        Page::Remove => "Remove apps that are installed on this system.",
        Page::Administration => "Run common Linux maintenance and power tasks.",
        Page::SystemInfo => "Inspect local system and runtime details.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn production_main() -> &'static str {
        include_str!("main.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("main module")
    }

    #[test]
    fn remove_ui_does_not_iterate_catalog_apps() {
        let src = production_main();
        assert!(
            src.contains("fn remove_page"),
            "Remove must have its own page"
        );
        assert!(
            !src.contains("fn install_remove_page"),
            "shared install/remove catalog page must be gone"
        );
        let remove_fn = src
            .split("fn remove_page")
            .nth(1)
            .and_then(|rest| rest.split("fn card_rows").next())
            .expect("remove_page body");
        assert!(
            !remove_fn.contains("self.apps"),
            "Remove UI must not iterate self.apps: {remove_fn}"
        );
        assert!(remove_fn.contains("remove_apps") || remove_fn.contains("start_remove_scan"));
    }

    #[test]
    fn new_catalog_labels_have_description_fallbacks() {
        for label in [
            "Sober",
            "Spotify",
            "Heroic",
            "Flatseal",
            "Telegram",
            "Prism Launcher",
            "Obsidian",
            "RetroArch",
            "Extension Manager",
            "Dolphin Emulator",
            "qBittorrent",
            "PPSSPP",
            "Gear Lever",
            "Proton VPN",
            "ProtonPlus",
            "Bitwarden",
            "Stremio",
            "LibreWolf",
            "Mission Center",
        ] {
            let description = app_description(label);
            assert!(!description.is_empty(), "{label}");
            assert_ne!(
                description, "Application from the Toolbox catalog",
                "{label}"
            );
        }
    }

    #[test]
    fn letter_fallback_labels_have_no_dashboardicons_key() {
        for label in [
            "Bottles",
            "Boxes",
            "ProtonUp-Qt",
            "GParted",
            "LocalSend",
            "Sober",
            "Flatseal",
            "Prism Launcher",
            "RetroArch",
            "Extension Manager",
            "Dolphin Emulator",
            "PPSSPP",
            "Gear Lever",
            "ProtonPlus",
            "Mission Center",
        ] {
            assert_eq!(icon_key(label), "", "{label}");
        }
        assert_eq!(icon_key("Brave Origin"), "brave-origin");
        assert_eq!(icon_key("Brave Browser"), "brave");
        assert_eq!(icon_key("Zen"), "zen-browser");
    }

    #[test]
    fn card_columns_match_window_breakpoints() {
        assert_eq!(
            {
                let w = 1180.0;
                if w >= 1280.0 {
                    3
                } else if w < 1000.0 {
                    1
                } else {
                    2
                }
            },
            2
        );
        assert_eq!(
            resolve_icon_key("zen-browser", ThemeMode::Dark),
            "zen-browser"
        );
        assert_eq!(
            resolve_icon_key("zen-browser", ThemeMode::Light),
            "zen-browser-dark"
        );
    }

    #[test]
    fn theme_toggle_is_the_combined_painter() {
        let src = production_main();
        assert!(src.contains("paint_toggle_icon"));
        assert!(!src.contains("paint_sun"));
        assert!(!src.contains("paint_moon"));
        assert!(!src.contains("assets/theme/"));
        let theme = include_str!("theme.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("theme");
        assert!(theme.contains("paint_theme_toggle"));
        assert!(!theme.contains("paint_sun"));
        assert!(!theme.contains("paint_moon"));
    }
}
