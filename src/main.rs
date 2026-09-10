mod catalog;
mod logos;
mod runner;
mod system;
mod theme;
mod validate;

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

use eframe::egui::{
    self, Align, Align2, Area, Button, CentralPanel, Color32, Context, FontData, FontDefinitions,
    FontFamily, FontId, Frame, Grid, Id, Key, Layout, Order, Painter, Panel, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, TextEdit, TextureHandle, TextureOptions, Ui, Vec2, pos2,
    vec2,
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
            .with_inner_size([1280.0, 840.0])
            .with_min_inner_size([960.0, 680.0])
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
    Home,
    Apps,
    Recipes,
    System,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Apps => "Apps",
            Page::Recipes => "Recipes",
            Page::System => "System",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppsMode {
    Install,
    Remove,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum RemoveScanState {
    Idle,
    Scanning,
    Ready,
    Failed(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TaskStatus {
    Queued,
    Running,
    Success,
    Failed,
}

#[derive(Clone, Debug)]
struct RunItem {
    description: String,
    status: TaskStatus,
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
const RAIL_WIDTH: f32 = 68.0;
const CARD_HEIGHT: f32 = 164.0;
const CARD_ICON: f32 = 44.0;
const CARD_GAP: f32 = 12.0;
const CARD_RADIUS: f32 = 12.0;
const CHECK_SIZE: f32 = 22.0;
const SEARCH_HEIGHT: f32 = 44.0;
const DOCK_MIN_WIDTH: f32 = 420.0;
const DOCK_RADIUS: f32 = 20.0;
const CONTENT_TWO_COL: f32 = 720.0;
const INTENT_HEIGHT: f32 = 128.0;
const GLANCE_HEIGHT: f32 = 96.0;
const TITLE_SIZE: f32 = 26.0;
const FRESH_SETUP_LABELS: &[&str] = &["Update System", "Fastfetch"];
const LAPTOP_POWER_LABELS: &[&str] = &["TLP (Laptops)", "Powertop"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Glyph {
    Plus,
    Bolt,
    Briefcase,
    Sliders,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RecentRun {
    kind: String,
    summary: String,
}

struct ToolboxApp {
    base_dir: PathBuf,
    page: Page,
    apps_mode: AppsMode,
    apps: Vec<AppEntry>,
    admin_tasks: Vec<AdminTask>,
    install_selected: HashSet<usize>,
    remove_selected: HashSet<usize>,
    admin_selected: HashSet<usize>,
    install_search: String,
    install_category: Option<String>,
    remove_search: String,
    distro_name: String,
    package_manager: String,
    host: String,
    kernel: String,
    shell: String,
    de_wm: String,
    recent_runs: Vec<RecentRun>,
    log: String,
    password: String,
    show_password_modal: bool,
    is_running: bool,
    tx: Option<Sender<RunnerMessage>>,
    rx: Option<Receiver<RunnerMessage>>,
    log_revision: u64,
    run_drawer_open: bool,
    run_items: Vec<RunItem>,
    icons: HashMap<&'static str, TextureHandle>,
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
            page: Page::Home,
            apps_mode: AppsMode::Install,
            apps: catalog.apps,
            admin_tasks: admin_tasks(),
            install_selected: HashSet::new(),
            remove_selected: HashSet::new(),
            admin_selected: HashSet::new(),
            install_search: String::new(),
            install_category: None,
            remove_search: String::new(),
            distro_name,
            package_manager,
            host: command_output("hostname", &[]),
            kernel: command_output("uname", &["-r"]),
            shell: env_or_unknown(&["SHELL"]),
            de_wm: env_or_unknown(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]),
            recent_runs: Vec::new(),
            log,
            password: String::new(),
            show_password_modal: false,
            is_running: false,
            tx: None,
            rx: None,
            log_revision: 0,
            run_drawer_open: false,
            run_items: Vec::new(),
            icons,
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

    fn switch_page(&mut self, page: Page) {
        self.page = page;
    }

    fn staged_count(&self, mode: AppsMode) -> usize {
        match mode {
            AppsMode::Install => self.install_selected.len(),
            AppsMode::Remove => self.remove_selected.len(),
        }
    }

    fn dock_can_appear(&self) -> bool {
        matches!(self.page, Page::Apps | Page::Recipes)
    }

    fn dock_visible(&self) -> bool {
        !self.is_running
            && match self.page {
                Page::Apps => self.staged_count(self.apps_mode) > 0,
                Page::Recipes => !self.admin_selected.is_empty(),
                Page::Home | Page::System => false,
            }
    }

    fn dock_count(&self) -> usize {
        match self.page {
            Page::Recipes => self.admin_selected.len(),
            _ => self.staged_count(self.apps_mode),
        }
    }

    fn dock_names(&self) -> Vec<String> {
        match self.page {
            Page::Recipes => self.admin_selected_names(),
            _ => self.active_selected_names(),
        }
    }

    fn selection_locked(&self) -> bool {
        self.is_running
    }

    fn selected_labels(
        indices: &HashSet<usize>,
        labels: impl Fn(usize) -> Option<String>,
    ) -> Vec<String> {
        let mut names: Vec<String> = indices.iter().copied().filter_map(labels).collect();
        names.sort();
        names
    }

    fn install_selected_names(&self) -> Vec<String> {
        Self::selected_labels(&self.install_selected, |index| {
            self.apps.get(index).map(|entry| entry.label.clone())
        })
    }

    fn remove_selected_names(&self) -> Vec<String> {
        Self::selected_labels(&self.remove_selected, |index| {
            self.remove_apps.get(index).map(|app| app.label.clone())
        })
    }

    fn admin_selected_names(&self) -> Vec<String> {
        Self::selected_labels(&self.admin_selected, |index| {
            self.admin_tasks.get(index).map(|task| task.label.clone())
        })
    }

    fn active_selected_names(&self) -> Vec<String> {
        match self.apps_mode {
            AppsMode::Install => self.install_selected_names(),
            AppsMode::Remove => self.remove_selected_names(),
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

    fn push_install_tasks(&self, tasks: &mut Vec<Task>, errors: &mut Vec<String>) {
        let mut indices: Vec<usize> = self.install_selected.iter().copied().collect();
        indices.sort_unstable();
        for index in indices {
            if let Some(entry) = self.apps.get(index) {
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

    fn push_remove_tasks(&self, tasks: &mut Vec<Task>, errors: &mut Vec<String>) {
        let mut indices: Vec<usize> = self.remove_selected.iter().copied().collect();
        indices.sort_unstable();
        for index in indices {
            if let Some(app) = self.remove_apps.get(index) {
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

    fn push_admin_tasks(&self, tasks: &mut Vec<Task>, errors: &mut Vec<String>) {
        let mut indices: Vec<usize> = self.admin_selected.iter().copied().collect();
        indices.sort_unstable();
        for index in indices {
            if let Some(task) = self.admin_tasks.get(index) {
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

    fn selected_tasks(&self) -> Result<Vec<Task>, Vec<String>> {
        let mut tasks = Vec::new();
        let mut errors = Vec::new();
        match self.page {
            Page::Recipes => self.push_admin_tasks(&mut tasks, &mut errors),
            Page::Apps => match self.apps_mode {
                AppsMode::Install => self.push_install_tasks(&mut tasks, &mut errors),
                AppsMode::Remove => self.push_remove_tasks(&mut tasks, &mut errors),
            },
            Page::Home | Page::System => {}
        }
        if errors.is_empty() {
            Ok(tasks)
        } else {
            Err(errors)
        }
    }

    #[allow(dead_code)]
    fn clear_all_selections(&mut self) {
        self.install_selected.clear();
        self.remove_selected.clear();
        self.admin_selected.clear();
    }

    fn clear_active_mode_selections(&mut self) {
        match self.apps_mode {
            AppsMode::Install => self.install_selected.clear(),
            AppsMode::Remove => self.remove_selected.clear(),
        }
    }

    fn clear_dock_selections(&mut self) {
        match self.page {
            Page::Recipes => self.admin_selected.clear(),
            Page::Apps => self.clear_active_mode_selections(),
            Page::Home | Page::System => {}
        }
    }

    fn preselect_admin_labels(&mut self, labels: &[&str]) {
        self.admin_selected.clear();
        for (index, task) in self.admin_tasks.iter().enumerate() {
            if labels.contains(&task.label.as_str()) {
                self.admin_selected.insert(index);
            }
        }
    }

    fn go_install_apps(&mut self) {
        self.apps_mode = AppsMode::Install;
        self.switch_page(Page::Apps);
    }

    fn go_fresh_setup(&mut self) {
        self.preselect_admin_labels(FRESH_SETUP_LABELS);
        self.switch_page(Page::Recipes);
    }

    fn go_laptop_power(&mut self) {
        self.preselect_admin_labels(LAPTOP_POWER_LABELS);
        self.switch_page(Page::Recipes);
    }

    fn go_system(&mut self) {
        self.switch_page(Page::System);
    }

    fn app_matches_filter(&self, entry: &AppEntry) -> bool {
        if let Some(category) = &self.install_category
            && &entry.category != category
        {
            return false;
        }

        let needle = self.install_search.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }

        entry.label.to_lowercase().contains(&needle)
            || entry.package_name.to_lowercase().contains(&needle)
            || entry.flatpak_id.to_lowercase().contains(&needle)
            || entry.category.to_lowercase().contains(&needle)
    }

    fn removable_matches_filter(&self, app: &RemovableApp) -> bool {
        let needle = self.remove_search.trim().to_lowercase();
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

    fn apply_log_to_run_items(&mut self, line: &str) {
        if let Some(description) = line.strip_prefix("[INFO] Starting: ") {
            self.set_run_status(description, TaskStatus::Running);
        } else if let Some(rest) = line.strip_prefix("[SUCCESS] ") {
            if let Some(description) = rest.strip_suffix(" completed successfully.") {
                self.set_run_status(description, TaskStatus::Success);
            }
        } else if let Some(rest) = line.strip_prefix("[ERROR] ")
            && let Some(description) = rest.split(" failed").next()
        {
            self.set_run_status(description, TaskStatus::Failed);
        }
    }

    fn set_run_status(&mut self, description: &str, status: TaskStatus) {
        if let Some(item) = self
            .run_items
            .iter_mut()
            .find(|item| item.description == description)
        {
            item.status = status;
        }
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
        self.run_items = tasks
            .iter()
            .map(|task| RunItem {
                description: task.description.clone(),
                status: TaskStatus::Queued,
            })
            .collect();
        self.run_drawer_open = true;
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
        let mut lines = Vec::new();
        if let Some(rx) = &self.rx {
            while let Ok(message) = rx.try_recv() {
                match message {
                    RunnerMessage::Log(line) => lines.push(strip_ansi(&line)),
                    RunnerMessage::Done => done = true,
                }
            }
        }

        for line in lines {
            if self.log == "Process logs will appear here..." {
                self.log.clear();
            }
            self.log.push('\n');
            self.log.push_str(&line);
            self.log_revision = self.log_revision.saturating_add(1);
            self.apply_log_to_run_items(&line);
        }

        if done {
            self.record_recent_run();
            self.is_running = false;
            zeroize_string(&mut self.password);
            self.tx = None;
            self.rx = None;
            self.log.push_str("\n\n[DONE] All tasks finished.");
            self.log_revision = self.log_revision.saturating_add(1);
        }
    }

    fn record_recent_run(&mut self) {
        let (kind, names) = match self.page {
            Page::Recipes => ("Recipe", self.admin_selected_names()),
            Page::Apps if self.apps_mode == AppsMode::Remove => {
                ("Remove", self.remove_selected_names())
            }
            Page::Apps => ("Install", self.install_selected_names()),
            Page::Home | Page::System => return,
        };
        if names.is_empty() {
            return;
        }
        self.recent_runs.insert(
            0,
            RecentRun {
                kind: kind.to_owned(),
                summary: truncated_names(&names, 48),
            },
        );
        self.recent_runs.truncate(8);
    }

    fn rail(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            toolbox_icon(ui, 22.0, &palette);
            ui.add_space(18.0);

            for page in [Page::Home, Page::Apps, Page::Recipes, Page::System] {
                if rail_button(ui, page, self.page == page, &palette).clicked() {
                    self.switch_page(page);
                    if page == Page::Apps && self.apps_mode == AppsMode::Remove {
                        let ctx = ui.ctx().clone();
                        self.start_remove_scan(&ctx);
                    }
                }
                ui.add_space(8.0);
            }
        });

        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            ui.add_space(14.0);
            if theme_toggle_button(ui, &palette).clicked() {
                let ctx = ui.ctx().clone();
                self.toggle_theme(&ctx);
            }
        });
    }

    fn apps_page(&mut self, ui: &mut Ui) {
        self.apps_header(ui);
        ui.add_space(16.0);
        match self.apps_mode {
            AppsMode::Install => self.install_page(ui),
            AppsMode::Remove => self.remove_page(ui),
        }
    }

    fn apps_header(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Apps")
                    .font(FontId::proportional(TITLE_SIZE))
                    .color(palette.text)
                    .strong(),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if let Some(mode) = mode_segment(ui, self.apps_mode, &palette) {
                    self.apps_mode = mode;
                    if mode == AppsMode::Remove {
                        let ctx = ui.ctx().clone();
                        self.start_remove_scan(&ctx);
                    }
                }
            });
        });
        ui.add_space(16.0);

        let search = match self.apps_mode {
            AppsMode::Install => &mut self.install_search,
            AppsMode::Remove => &mut self.remove_search,
        };
        let hint = match self.apps_mode {
            AppsMode::Install => "Search apps...",
            AppsMode::Remove => "Search installed apps...",
        };
        search_field(ui, search, hint, &palette);

        if self.apps_mode == AppsMode::Install {
            ui.add_space(12.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.spacing_mut().item_spacing.y = 8.0;
                if filter_chip(ui, "All", self.install_category.is_none(), &palette).clicked() {
                    self.install_category = None;
                }
                for category in INSTALL_CATEGORIES {
                    if filter_chip(
                        ui,
                        category,
                        self.install_category.as_deref() == Some(*category),
                        &palette,
                    )
                    .clicked()
                    {
                        self.install_category = Some((*category).to_owned());
                    }
                }
            });
        }
    }

    fn install_page(&mut self, ui: &mut Ui) {
        let visible: Vec<usize> = self
            .apps
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| self.app_matches_filter(entry).then_some(index))
            .collect();
        ScrollArea::vertical()
            .id_salt("install_cards")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = card_columns(ui);
                self.card_rows(ui, columns, visible.len(), |app, ui, index, width| {
                    app.install_tile(ui, visible[index], width);
                });
            });
    }

    fn remove_page(&mut self, ui: &mut Ui) {
        match &self.remove_scan {
            RemoveScanState::Idle | RemoveScanState::Scanning => {
                centered_status(ui, |ui| {
                    ui.spinner();
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Scanning installed apps...").color(self.palette().muted),
                    );
                });
            }
            RemoveScanState::Failed(error) => {
                let error = error.clone();
                centered_status(ui, |ui| {
                    ui.label(
                        RichText::new(format!("Remove scan failed: {error}"))
                            .color(self.palette().text),
                    );
                    ui.add_space(12.0);
                    if cta_button(ui, "Retry", true, &self.palette()).clicked() {
                        let ctx = ui.ctx().clone();
                        self.start_remove_scan(&ctx);
                    }
                });
            }
            RemoveScanState::Ready if self.remove_apps.is_empty() => {
                centered_status(ui, |ui| {
                    ui.label(
                        RichText::new("No removable apps found on this system.")
                            .color(self.palette().text),
                    );
                    ui.add_space(12.0);
                    if cta_button(ui, "Refresh", true, &self.palette()).clicked() {
                        let ctx = ui.ctx().clone();
                        self.start_remove_scan(&ctx);
                    }
                });
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
            ui.add_space(CARD_GAP);
        }
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
        let locked = self.selection_locked();
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
            && !locked
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
        let locked = self.selection_locked();
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
            && !locked
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

    fn ready_line(&self) -> String {
        format!(
            "{} is ready · package manager {}",
            self.distro_name, self.package_manager
        )
    }

    fn glance_metrics(&self) -> [(&'static str, String); 4] {
        [
            ("OS", self.distro_name.clone()),
            ("Kernel", self.kernel.clone()),
            ("Uptime", uptime()),
            ("PM", self.package_manager.clone()),
        ]
    }

    fn system_detail_rows(&self) -> [(&'static str, String); 8] {
        [
            ("OS", self.distro_name.clone()),
            ("Host", self.host.clone()),
            ("Kernel", self.kernel.clone()),
            ("Uptime", uptime()),
            ("Shell", self.shell.clone()),
            ("DE/WM", self.de_wm.clone()),
            ("Package Manager", self.package_manager.clone()),
            ("App Directory", self.base_dir.display().to_string()),
        ]
    }

    fn copy_system_details(&self, ctx: &Context) {
        let text = self
            .system_detail_rows()
            .into_iter()
            .map(|(label, value)| format!("{label}: {value}"))
            .collect::<Vec<_>>()
            .join("\n");
        ctx.copy_text(text);
    }

    fn recipe_groups(&self) -> Vec<(String, Vec<usize>)> {
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for (index, task) in self.admin_tasks.iter().enumerate() {
            if let Some((_, items)) = groups
                .iter_mut()
                .find(|(category, _)| category == &task.category)
            {
                items.push(index);
            } else {
                groups.push((task.category.clone(), vec![index]));
            }
        }
        groups
    }

    fn home_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ScrollArea::vertical()
            .id_salt("home_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new(greeting())
                        .font(FontId::proportional(TITLE_SIZE))
                        .color(palette.text)
                        .strong(),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter()
                        .circle_filled(dot.center(), 3.5, palette.accent);
                    ui.add_space(4.0);
                    ui.label(RichText::new(self.ready_line()).color(palette.muted));
                });
                ui.add_space(22.0);

                let intents = [
                    (
                        Glyph::Plus,
                        "Install apps",
                        "Browse the catalog and stage what you need.",
                    ),
                    (
                        Glyph::Bolt,
                        "Fresh setup",
                        "One recipe for a new distro hop — essentials ready.",
                    ),
                    (
                        Glyph::Briefcase,
                        "Laptop power",
                        "TLP + Powertop tuned so the battery lasts.",
                    ),
                ];
                let columns = intent_columns(ui);
                let width = tile_width(ui, columns);
                let mut chosen = None;
                for (row_index, row) in intents.chunks(columns).enumerate() {
                    if row_index > 0 {
                        ui.add_space(CARD_GAP);
                    }
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        for (offset, (glyph, title, body)) in row.iter().enumerate() {
                            let index = row_index * columns + offset;
                            if intent_card(ui, width, *glyph, title, body, &palette).clicked() {
                                chosen = Some(index);
                            }
                        }
                    });
                }
                ui.add_space(CARD_GAP);
                match chosen {
                    Some(0) => self.go_install_apps(),
                    Some(1) => self.go_fresh_setup(),
                    Some(2) => self.go_laptop_power(),
                    _ => {}
                }

                ui.add_space(8.0);
                section_eyebrow(ui, "System glance", &palette);
                ui.add_space(10.0);
                let metrics = self.glance_metrics();
                let glance_cols = glance_columns(ui);
                let glance_width = tile_width(ui, glance_cols);
                let mut open_system = false;
                for start in (0..metrics.len()).step_by(glance_cols) {
                    let end = (start + glance_cols).min(metrics.len());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        for item in &metrics[start..end] {
                            if metric_tile(ui, glance_width, item.0, &item.1, true, &palette)
                                .clicked()
                            {
                                open_system = true;
                            }
                        }
                    });
                    ui.add_space(CARD_GAP);
                }
                if open_system {
                    self.go_system();
                }

                ui.add_space(8.0);
                section_eyebrow(ui, "Recent runs", &palette);
                ui.add_space(10.0);
                self.recent_runs_card(ui, &palette);
            });
    }

    fn recent_runs_card(&self, ui: &mut Ui, palette: &Palette) {
        Frame::new()
            .fill(palette.surface)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .corner_radius(CARD_RADIUS)
            .inner_margin(egui::Margin::symmetric(18, 16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.recent_runs.is_empty() {
                    ui.label(
                        RichText::new("No runs yet — stage apps or a recipe to get started.")
                            .color(palette.muted),
                    );
                    return;
                }
                for (index, run) in self.recent_runs.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(10.0);
                    }
                    ui.horizontal(|ui| {
                        paint_kind_chip(ui, &run.kind, palette);
                        ui.add_space(8.0);
                        ui.label(RichText::new(&run.summary).color(palette.text));
                    });
                }
            });
    }

    fn recipes_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.label(
            RichText::new("Recipes")
                .font(FontId::proportional(TITLE_SIZE))
                .color(palette.text)
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(RichText::new("One-click admin playbooks for this machine.").color(palette.muted));
        ui.add_space(16.0);

        let groups = self.recipe_groups();
        ScrollArea::vertical()
            .id_salt("recipes_cards")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (group_index, (category, indices)) in groups.iter().enumerate() {
                    if group_index > 0 {
                        ui.add_space(8.0);
                    }
                    section_eyebrow(ui, category, &palette);
                    ui.add_space(10.0);
                    let columns = card_columns(ui);
                    self.card_rows(ui, columns, indices.len(), |app, ui, index, width| {
                        app.recipe_tile(ui, indices[index], width);
                    });
                }
            });
    }

    fn recipe_tile(&mut self, ui: &mut Ui, index: usize, width: f32) {
        let task = &self.admin_tasks[index];
        let label = task.label.clone();
        let category = task.category.clone();
        let selected = self.admin_selected.contains(&index);
        let palette = self.palette();
        let locked = self.selection_locked();
        if recipe_card(ui, width, selected, &label, &category, &palette).clicked() && !locked {
            if selected {
                self.admin_selected.remove(&index);
            } else {
                self.admin_selected.insert(index);
            }
        }
    }

    fn system_info_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let metrics = self.glance_metrics();
        let rows = self.system_detail_rows();
        ScrollArea::vertical()
            .id_salt("system_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new(&self.distro_name)
                        .font(FontId::proportional(TITLE_SIZE))
                        .color(palette.text)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!(
                        "{} · {} · {}",
                        self.kernel,
                        self.package_manager,
                        uptime()
                    ))
                    .color(palette.muted),
                );
                ui.add_space(20.0);

                let glance_cols = glance_columns(ui);
                let glance_width = tile_width(ui, glance_cols);
                for start in (0..metrics.len()).step_by(glance_cols) {
                    let end = (start + glance_cols).min(metrics.len());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        for item in &metrics[start..end] {
                            metric_tile(ui, glance_width, item.0, &item.1, false, &palette);
                        }
                    });
                    ui.add_space(CARD_GAP);
                }

                ui.add_space(8.0);
                Frame::new()
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .corner_radius(CARD_RADIUS)
                    .inner_margin(egui::Margin::symmetric(18, 16))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Details").color(palette.text).strong());
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ghost_button(ui, "Copy all", &palette).clicked() {
                                    self.copy_system_details(ui.ctx());
                                }
                            });
                        });
                        ui.add_space(12.0);
                        Grid::new("system_info_grid")
                            .num_columns(2)
                            .spacing(Vec2::new(28.0, 12.0))
                            .show(ui, |ui| {
                                for (label, value) in &rows {
                                    ui.label(RichText::new(*label).color(palette.muted));
                                    ui.label(RichText::new(value).color(palette.text));
                                    ui.end_row();
                                }
                            });
                    });
            });
    }

    fn modal_count_line(&self) -> String {
        if self.page == Page::Recipes {
            let n = self.admin_selected.len();
            return if n == 1 {
                "This will run 1 recipe.".to_owned()
            } else {
                format!("This will run {n} recipes.")
            };
        }
        let n = self.staged_count(self.apps_mode);
        match self.apps_mode {
            AppsMode::Install if n == 1 => "This will install 1 app.".to_owned(),
            AppsMode::Install => format!("This will install {n} apps."),
            AppsMode::Remove if n == 1 => "This will remove 1 app.".to_owned(),
            AppsMode::Remove => format!("This will remove {n} apps."),
        }
    }

    fn review_groups(&self) -> Vec<(&'static str, Vec<String>)> {
        if self.page == Page::Recipes {
            let recipes = self.admin_selected_names();
            return if recipes.is_empty() {
                Vec::new()
            } else {
                vec![("Recipes", recipes)]
            };
        }
        let install = if self.apps_mode == AppsMode::Install {
            self.install_selected_names()
        } else {
            Vec::new()
        };
        let remove = if self.apps_mode == AppsMode::Remove {
            self.remove_selected_names()
        } else {
            Vec::new()
        };
        let mut groups = Vec::new();
        if !install.is_empty() {
            groups.push(("Install", install));
        }
        if !remove.is_empty() {
            groups.push(("Remove", remove));
        }
        groups
    }

    fn other_mode_staged_note(&self) -> Option<String> {
        if self.page != Page::Apps {
            return None;
        }
        match self.apps_mode {
            AppsMode::Install if !self.remove_selected.is_empty() => Some(format!(
                "{} also staged in Remove. Switch to Remove to run those.",
                self.remove_selected.len()
            )),
            AppsMode::Remove if !self.install_selected.is_empty() => Some(format!(
                "{} also staged in Install. Switch to Install to run those.",
                self.install_selected.len()
            )),
            _ => None,
        }
    }

    fn run_dock(&mut self, ctx: &Context) {
        if !self.dock_visible() {
            return;
        }

        let palette = self.palette();
        let count = self.dock_count();
        let names = truncated_names(&self.dock_names(), 42);

        let mut clear = false;
        let mut review = false;
        let y_offset = if self.run_drawer_open { -28.0 } else { -24.0 };

        Area::new(Id::new("run_dock"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, y_offset])
            .order(Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.surface)
                    .stroke(Stroke::new(1.0_f32, palette.border_strong))
                    .corner_radius(DOCK_RADIUS)
                    .inner_margin(egui::Margin::symmetric(18, 12))
                    .shadow(egui::Shadow {
                        offset: [0, 8],
                        blur: 24,
                        spread: 0,
                        color: Color32::from_black_alpha(80),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(DOCK_MIN_WIDTH);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.label(
                                RichText::new(format!("{count}"))
                                    .color(palette.accent)
                                    .strong(),
                            );
                            ui.label(RichText::new("staged").color(palette.text).strong());
                            ui.label(RichText::new(names).color(palette.muted));
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if cta_button(ui, "Review & Run  →", true, &palette).clicked() {
                                    review = true;
                                }
                                ui.add_space(8.0);
                                if ghost_button(ui, "Clear", &palette).clicked() {
                                    clear = true;
                                }
                            });
                        });
                    });
            });

        if clear {
            self.clear_dock_selections();
        }
        if review {
            self.show_password_modal = true;
        }
    }

    fn run_drawer(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Run").color(palette.text).strong());
            if self.is_running {
                ui.add_space(8.0);
                ui.spinner();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ghost_button(ui, "Dismiss", &palette).clicked() {
                    self.run_drawer_open = false;
                }
                ui.add_space(6.0);
                if ghost_button(ui, "Clear Log", &palette).clicked() {
                    self.log = "Process logs will appear here...".to_owned();
                    self.log_revision = self.log_revision.saturating_add(1);
                }
            });
        });
        ui.add_space(8.0);

        if !self.run_items.is_empty() {
            ScrollArea::vertical()
                .id_salt("run_task_status")
                .max_height(72.0)
                .show(ui, |ui| {
                    for item in &self.run_items {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(task_status_label(item.status))
                                    .color(task_status_color(item.status, &palette))
                                    .strong(),
                            );
                            ui.label(RichText::new(&item.description).color(palette.text));
                        });
                    }
                });
            ui.add_space(8.0);
        }

        let log_height = (ui.available_height() - 8.0).max(80.0);
        let showing_placeholder = self.log == "Process logs will appear here...";
        Frame::new()
            .fill(palette.log_inner)
            .stroke(Stroke::new(1.0_f32, palette.border))
            .inner_margin(8.0)
            .corner_radius(10.0)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("run_drawer_log")
                    .stick_to_bottom(true)
                    .max_height(log_height)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(log_height - 18.0);
                        ui.label(
                            RichText::new(&self.log)
                                .font(FontId::monospace(12.5))
                                .color(if showing_placeholder {
                                    palette.muted
                                } else {
                                    palette.log_text
                                }),
                        );
                        if !showing_placeholder {
                            ui.scroll_to_cursor(Some(Align::BOTTOM));
                        }
                    });
            });
    }

    fn sudo_modal(&mut self, ctx: &Context) {
        if !self.show_password_modal {
            return;
        }
        let palette = self.palette();

        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            zeroize_string(&mut self.password);
            self.show_password_modal = false;
            return;
        }

        let mut open = true;
        let count_line = self.modal_count_line();
        let groups = self.review_groups();
        let other_note = self.other_mode_staged_note();
        let toolbox = (self.apps_mode == AppsMode::Remove)
            .then(|| self.remove_includes_toolbox())
            .flatten();

        egui::Window::new("Review & Run")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .open(&mut open)
            .frame(
                Frame::new()
                    .fill(palette.modal_fill)
                    .stroke(Stroke::new(1.0_f32, palette.border_strong))
                    .inner_margin(18.0)
                    .corner_radius(12.0),
            )
            .show(ctx, |ui| {
                ui.set_min_width(320.0);
                ui.label(
                    RichText::new("Review & Run")
                        .font(FontId::proportional(18.0))
                        .color(palette.modal_text)
                        .strong(),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(count_line).color(palette.modal_text));
                if let Some(toolbox) = &toolbox {
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(format!(
                            "This selection includes the running Toolbox: {toolbox}."
                        ))
                        .color(palette.danger)
                        .strong(),
                    );
                }
                if !groups.is_empty() {
                    ui.add_space(10.0);
                    ScrollArea::vertical()
                        .id_salt("sudo_name_list")
                        .max_height(240.0)
                        .show(ui, |ui| {
                            let mut first_group = true;
                            for (header, names) in &groups {
                                if !first_group {
                                    ui.add_space(6.0);
                                }
                                first_group = false;
                                ui.label(RichText::new(*header).color(palette.modal_text).strong());
                                for name in names {
                                    ui.label(
                                        RichText::new(format!("• {name}"))
                                            .color(palette.modal_text),
                                    );
                                }
                            }
                        });
                }
                if let Some(note) = &other_note {
                    ui.add_space(8.0);
                    ui.label(RichText::new(note).color(palette.muted));
                }
                ui.add_space(12.0);
                ui.label(RichText::new("Sudo password").color(palette.muted).strong());
                ui.add_space(4.0);
                let response = ui.add(
                    TextEdit::singleline(&mut self.password)
                        .password(true)
                        .hint_text(RichText::new("Required to run").color(palette.muted))
                        .desired_width(280.0)
                        .text_color(palette.text)
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
                        Button::new(RichText::new("Cancel").color(palette.cancel_text).strong())
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
                            Button::new(RichText::new("Confirm").color(palette.cta_text).strong())
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

impl eframe::App for ToolboxApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.drain_runner_messages();
        self.drain_remove_scan();

        let palette = self.palette();
        Panel::left("rail")
            .resizable(false)
            .exact_size(RAIL_WIDTH)
            .frame(Frame::new().fill(palette.void).inner_margin(6.0))
            .show_inside(ui, |ui| self.rail(ui));

        if self.run_drawer_open {
            let height = (ui.ctx().content_rect().height() * 0.40).clamp(220.0, 480.0);
            Panel::bottom("run_drawer")
                .exact_size(height)
                .frame(
                    Frame::new()
                        .fill(palette.surface)
                        .stroke(Stroke::new(1.0_f32, palette.border_strong))
                        .inner_margin(egui::Margin::symmetric(16, 12)),
                )
                .show_inside(ui, |ui| self.run_drawer(ui));
        }

        CentralPanel::default()
            .frame(Frame::new().fill(palette.void).inner_margin(egui::Margin {
                left: 40,
                right: 40,
                top: 28,
                bottom: if self.dock_can_appear() { 120 } else { 40 },
            }))
            .show_inside(ui, |ui| match self.page {
                Page::Home => self.home_page(ui),
                Page::Apps => self.apps_page(ui),
                Page::Recipes => self.recipes_page(ui),
                Page::System => self.system_info_page(ui),
            });

        let ctx = ui.ctx().clone();
        self.run_dock(&ctx);
        self.sudo_modal(&ctx);
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

fn rail_button(ui: &mut Ui, page: Page, selected: bool, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    let fill = if selected {
        palette.elevated
    } else if response.hovered() {
        palette.nav_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 12.0, fill);
    let icon_color = if selected {
        palette.accent
    } else {
        palette.muted
    };
    paint_page_symbol(
        ui.painter(),
        Rect::from_center_size(rect.center(), vec2(22.0, 22.0)),
        page,
        icon_color,
    );
    if selected {
        let pip = pos2(rect.right() - 6.0, rect.center().y);
        ui.painter().circle_filled(pip, 3.0, palette.accent);
    }
    response.on_hover_text(page.title())
}

fn theme_toggle_button(ui: &mut Ui, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::click());
    let response = response.on_hover_text(palette.mode.toggle_tooltip());
    let punch = if response.hovered() {
        ui.painter().rect_filled(rect, 12.0, palette.nav_hover);
        palette.nav_hover
    } else {
        palette.void
    };
    let icon_rect = Rect::from_center_size(rect.center(), vec2(22.0, 22.0));
    palette.paint_toggle_icon(ui.painter(), icon_rect, punch);
    response
}

fn mode_segment(ui: &mut Ui, mode: AppsMode, palette: &Palette) -> Option<AppsMode> {
    let mut clicked = None;
    ui.allocate_ui_with_layout(
        vec2(196.0, 36.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            Frame::new()
                .fill(palette.surface)
                .stroke(Stroke::new(1.0_f32, palette.border))
                .corner_radius(18.0)
                .inner_margin(egui::Margin::symmetric(4, 4))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if segment_button(ui, "Install", mode == AppsMode::Install, palette).clicked() {
                        clicked = Some(AppsMode::Install);
                    }
                    if segment_button(ui, "Remove", mode == AppsMode::Remove, palette).clicked() {
                        clicked = Some(AppsMode::Remove);
                    }
                });
        },
    );
    clicked
}

fn segment_button(ui: &mut Ui, label: &str, selected: bool, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(88.0, 28.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 14.0, palette.text);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 14.0, palette.nav_hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        if selected {
            palette.void
        } else {
            palette.muted
        },
    );
    response
}

fn filter_chip(ui: &mut Ui, label: &str, selected: bool, palette: &Palette) -> egui::Response {
    let width = (label.len() as f32 * 7.4 + 24.0).clamp(48.0, 132.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 30.0), Sense::click());
    let fill = if selected {
        match palette.mode {
            ThemeMode::Dark => palette.filter_selected_fill,
            ThemeMode::Light => palette.accent_bright,
        }
    } else if response.hovered() {
        palette.surface_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 15.0, fill);
    if !selected {
        ui.painter().rect_stroke(
            rect,
            15.0,
            Stroke::new(1.0_f32, palette.border),
            StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.0),
        if selected {
            palette.filter_selected_text
        } else {
            palette.muted
        },
    );
    response
}

fn search_field(ui: &mut Ui, search: &mut String, hint: &str, palette: &Palette) {
    let width = ui.available_width();
    Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0_f32, palette.input_border))
        .corner_radius(14.0)
        .inner_margin(egui::Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(SEARCH_HEIGHT);
            ui.horizontal_centered(|ui| {
                let (icon_rect, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                paint_search_icon(ui.painter(), icon_rect, palette);
                ui.add_space(8.0);
                ui.add(
                    TextEdit::singleline(search)
                        .hint_text(RichText::new(hint).color(palette.muted))
                        .font(FontId::proportional(14.0))
                        .desired_width(ui.available_width())
                        .frame(Frame::NONE)
                        .text_color(palette.text)
                        .background_color(palette.input_fill),
                );
            });
        });
}

fn paint_search_icon(painter: &Painter, rect: Rect, palette: &Palette) {
    let center = pos2(rect.left() + 7.0, rect.center().y - 1.0);
    painter.circle_stroke(center, 6.0, Stroke::new(1.5_f32, palette.muted));
    painter.line_segment(
        [
            pos2(center.x + 4.5, center.y + 4.5),
            pos2(center.x + 9.0, center.y + 9.0),
        ],
        Stroke::new(1.5_f32, palette.muted),
    );
}

fn paint_card_background(painter: &Painter, rect: Rect, selected: bool, palette: &Palette) {
    painter.rect_filled(
        rect,
        CARD_RADIUS,
        if selected {
            palette.tile_selected
        } else {
            palette.tile
        },
    );
    if selected {
        painter.rect_filled(rect, CARD_RADIUS, palette.accent_dim);
    }
    painter.rect_stroke(
        rect,
        CARD_RADIUS,
        Stroke::new(
            if selected { 1.5_f32 } else { 1.0_f32 },
            if selected {
                palette.accent
            } else {
                palette.border
            },
        ),
        StrokeKind::Inside,
    );
}

fn paint_check(painter: &Painter, rect: Rect, selected: bool, palette: &Palette) {
    let center = rect.center();
    let radius = rect.width() / 2.0;
    if selected {
        painter.circle_filled(center, radius, palette.accent);
        let a = pos2(rect.left() + 6.0, center.y);
        let b = pos2(rect.left() + 9.5, rect.bottom() - 6.5);
        let c = pos2(rect.right() - 6.0, rect.top() + 6.5);
        painter.line_segment([a, b], Stroke::new(2.0_f32, palette.accent_on));
        painter.line_segment([b, c], Stroke::new(2.0_f32, palette.accent_on));
    } else {
        painter.circle_stroke(
            center,
            radius - 0.5,
            Stroke::new(1.2_f32, palette.border_strong),
        );
    }
}

fn paint_app_icon(
    painter: &Painter,
    rect: Rect,
    label: &str,
    icon: Option<&(TextureHandle, bool)>,
    palette: &Palette,
) {
    painter.circle_filled(rect.center(), rect.width() / 2.0, palette.icon_well);
    if let Some((icon, mono)) = icon {
        let tint = if *mono { palette.text } else { Color32::WHITE };
        painter.image(
            icon.id(),
            rect.shrink(4.0),
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            tint,
        );
        return;
    }

    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        app_mark(label),
        FontId::proportional(16.0),
        palette.text,
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
    paint_card_background(ui.painter(), rect, selected, palette);

    let icon_rect = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(CARD_ICON, CARD_ICON));
    paint_app_icon(ui.painter(), icon_rect, label, icon, palette);

    let check_rect = Rect::from_min_size(
        pos2(rect.right() - 16.0 - CHECK_SIZE, rect.min.y + 16.0),
        vec2(CHECK_SIZE, CHECK_SIZE),
    );
    paint_check(ui.painter(), check_rect, selected, palette);

    let text_left = rect.min.x + 16.0;
    let text_width = (rect.width() - 32.0).max(24.0);
    let painter = ui.painter();
    painter.text(
        pos2(text_left, rect.min.y + 72.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        palette.text,
    );

    let galley = painter.layout(
        description.to_owned(),
        FontId::proportional(12.0),
        palette.muted,
        text_width,
    );
    let desc_pos = pos2(text_left, rect.min.y + 86.0);
    let clip = Rect::from_min_size(desc_pos, vec2(text_width, 32.0));
    painter
        .with_clip_rect(clip)
        .galley(desc_pos, galley, palette.muted);

    if let Some(source) = source {
        paint_source_pill(
            painter,
            pos2(text_left + 4.0, rect.bottom() - 22.0),
            source,
            palette,
        );
    }

    response
}

fn card_columns(ui: &Ui) -> usize {
    if ui.available_width() < CONTENT_TWO_COL {
        2
    } else {
        3
    }
}

fn intent_columns(ui: &Ui) -> usize {
    if ui.available_width() < 560.0 {
        1
    } else if ui.available_width() < CONTENT_TWO_COL {
        2
    } else {
        3
    }
}

fn glance_columns(ui: &Ui) -> usize {
    if ui.available_width() < 640.0 { 2 } else { 4 }
}

fn tile_width(ui: &Ui, columns: usize) -> f32 {
    let columns = columns.max(1) as f32;
    ((ui.available_width() - CARD_GAP * (columns - 1.0)) / columns).max(220.0)
}

fn section_eyebrow(ui: &mut Ui, label: &str, palette: &Palette) {
    ui.label(
        RichText::new(label.to_ascii_uppercase())
            .font(FontId::proportional(11.0))
            .color(palette.muted),
    );
}

fn greeting() -> &'static str {
    greeting_for_hour(local_hour())
}

fn local_hour() -> u32 {
    command_output("date", &["+%H"])
        .parse()
        .ok()
        .filter(|hour| *hour < 24)
        .unwrap_or(18)
}

fn greeting_for_hour(hour: u32) -> &'static str {
    match hour {
        5..=11 => "Good morning",
        12..=16 => "Good afternoon",
        _ => "Good evening",
    }
}

fn intent_card(
    ui: &mut Ui,
    width: f32,
    glyph: Glyph,
    title: &str,
    body: &str,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, INTENT_HEIGHT), Sense::click());
    paint_surface_card(ui.painter(), rect, response.hovered(), palette);

    let well = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(36.0, 36.0));
    ui.painter().rect_filled(well, 10.0, palette.icon_well);
    paint_glyph(ui.painter(), well.shrink(8.0), glyph, palette.accent);

    let painter = ui.painter();
    let text_left = rect.min.x + 16.0;
    let text_width = (rect.width() - 32.0).max(24.0);
    painter.text(
        pos2(text_left, rect.min.y + 68.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(15.0),
        palette.text,
    );
    let galley = painter.layout(
        body.to_owned(),
        FontId::proportional(12.0),
        palette.muted,
        text_width,
    );
    painter
        .with_clip_rect(Rect::from_min_size(
            pos2(text_left, rect.min.y + 82.0),
            vec2(text_width, 34.0),
        ))
        .galley(pos2(text_left, rect.min.y + 82.0), galley, palette.muted);
    response
}

fn metric_tile(
    ui: &mut Ui,
    width: f32,
    label: &str,
    value: &str,
    clickable: bool,
    palette: &Palette,
) -> egui::Response {
    let sense = if clickable {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, GLANCE_HEIGHT), sense);
    paint_surface_card(ui.painter(), rect, response.hovered() && clickable, palette);
    let painter = ui.painter();
    painter.text(
        pos2(rect.min.x + 16.0, rect.min.y + 22.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(11.0),
        palette.muted,
    );
    painter.text(
        pos2(rect.min.x + 16.0, rect.min.y + 48.0),
        Align2::LEFT_CENTER,
        value,
        FontId::proportional(20.0),
        palette.text,
    );
    let bar = Rect::from_min_size(
        pos2(rect.min.x + 16.0, rect.bottom() - 18.0),
        vec2(36.0, 3.0),
    );
    painter.rect_filled(bar, 2.0, palette.accent);
    response
}

fn paint_surface_card(painter: &Painter, rect: Rect, hovered: bool, palette: &Palette) {
    painter.rect_filled(
        rect,
        CARD_RADIUS,
        if hovered {
            palette.surface_hover
        } else {
            palette.surface
        },
    );
    painter.rect_stroke(
        rect,
        CARD_RADIUS,
        Stroke::new(1.0_f32, palette.border),
        StrokeKind::Inside,
    );
}

fn recipe_card(
    ui: &mut Ui,
    width: f32,
    selected: bool,
    label: &str,
    category: &str,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, CARD_HEIGHT), Sense::click());
    paint_card_background(ui.painter(), rect, selected, palette);

    let well = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(CARD_ICON, CARD_ICON));
    ui.painter().rect_filled(well, 12.0, palette.icon_well);
    paint_glyph(
        ui.painter(),
        well.shrink(10.0),
        recipe_glyph(category),
        palette.accent,
    );

    let check_rect = Rect::from_min_size(
        pos2(rect.right() - 16.0 - CHECK_SIZE, rect.min.y + 16.0),
        vec2(CHECK_SIZE, CHECK_SIZE),
    );
    paint_check(ui.painter(), check_rect, selected, palette);

    let text_left = rect.min.x + 16.0;
    let text_width = (rect.width() - 32.0).max(24.0);
    let painter = ui.painter();
    painter.text(
        pos2(text_left, rect.min.y + 72.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        palette.text,
    );

    let galley = painter.layout(
        recipe_promise(label).to_owned(),
        FontId::proportional(12.0),
        palette.muted,
        text_width,
    );
    painter
        .with_clip_rect(Rect::from_min_size(
            pos2(text_left, rect.min.y + 86.0),
            vec2(text_width, 28.0),
        ))
        .galley(pos2(text_left, rect.min.y + 86.0), galley, palette.muted);

    if let Some(chip) = recipe_chip(label) {
        paint_warm_chip(
            painter,
            pos2(text_left + 4.0, rect.bottom() - 22.0),
            chip,
            palette,
        );
    }

    response
}

fn recipe_glyph(category: &str) -> Glyph {
    if category == "Power Management" {
        Glyph::Bolt
    } else {
        Glyph::Sliders
    }
}

fn recipe_promise(label: &str) -> &'static str {
    match label {
        "Enable Bluetooth" => "Turn Bluetooth on for this machine.",
        "Disable Bluetooth" => "Turn Bluetooth off to save power.",
        "TLP (Laptops)" => "Tune laptop power so the battery lasts.",
        "Powertop" => "Diagnose and cut background power waste.",
        "Update System" => "Bring packages on this machine up to date.",
        "nala (rank mirrors) - Debian only" => "Rank Debian mirrors for faster apt.",
        "Stacer" => "Lightweight Linux system optimizer.",
        "SWAP Fix" => "Lower swappiness so resume stays snappy.",
        "Fastfetch" => "Show a clean system summary in the terminal.",
        _ => "Admin playbook for this machine.",
    }
}

fn recipe_chip(label: &str) -> Option<&'static str> {
    match label {
        "Update System" | "SWAP Fix" => Some("Caution"),
        other if other.contains("Debian only") => Some("Debian only"),
        _ => None,
    }
}

fn paint_warm_chip(painter: &Painter, left_center: Pos2, label: &str, palette: &Palette) {
    let width = (label.len() as f32 * 7.2 + 16.0).clamp(56.0, 140.0);
    let rect = Rect::from_center_size(
        pos2(left_center.x + width / 2.0, left_center.y),
        vec2(width, 20.0),
    );
    painter.rect_filled(rect, 6.0, palette.caution);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(10.5),
        palette.caution_on,
    );
}

fn paint_kind_chip(ui: &mut Ui, kind: &str, palette: &Palette) {
    let width = (kind.len() as f32 * 7.2 + 16.0).clamp(56.0, 100.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
    let (fill, text) = match kind {
        "Recipe" => (palette.caution, palette.caution_on),
        _ => (palette.elevated, palette.accent),
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        kind,
        FontId::proportional(11.0),
        text,
    );
}

fn paint_glyph(painter: &Painter, rect: Rect, glyph: Glyph, color: Color32) {
    let stroke = Stroke::new(1.8_f32, color);
    match glyph {
        Glyph::Plus => {
            painter.line_segment(
                [
                    pos2(rect.center().x, rect.top()),
                    pos2(rect.center().x, rect.bottom()),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left(), rect.center().y),
                    pos2(rect.right(), rect.center().y),
                ],
                stroke,
            );
        }
        Glyph::Bolt => {
            painter.line_segment(
                [
                    pos2(rect.center().x + 3.0, rect.top()),
                    pos2(rect.left() + 2.0, rect.center().y + 1.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 2.0, rect.center().y + 1.0),
                    pos2(rect.center().x + 1.0, rect.center().y + 1.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.center().x + 1.0, rect.center().y + 1.0),
                    pos2(rect.center().x - 3.0, rect.bottom()),
                ],
                stroke,
            );
        }
        Glyph::Briefcase => {
            painter.rect_stroke(
                Rect::from_min_max(
                    pos2(rect.left() + 1.0, rect.center().y - 2.0),
                    pos2(rect.right() - 1.0, rect.bottom() - 1.0),
                ),
                2.0,
                stroke,
                StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_center_size(pos2(rect.center().x, rect.top() + 4.0), vec2(8.0, 6.0)),
                1.5,
                stroke,
                StrokeKind::Inside,
            );
        }
        Glyph::Sliders => {
            for (y_off, knob) in [(0.22, 0.65), (0.50, 0.35), (0.78, 0.55)] {
                let y = rect.top() + rect.height() * y_off;
                painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], stroke);
                painter.circle_filled(pos2(rect.left() + rect.width() * knob, y), 2.6, color);
            }
        }
    }
}

fn cta_button(ui: &mut Ui, label: &str, enabled: bool, palette: &Palette) -> egui::Response {
    ui.add_enabled(
        enabled,
        Button::new(RichText::new(label).color(palette.cta_text).strong())
            .fill(palette.cta_fill)
            .stroke(Stroke::new(1.0_f32, palette.cta_fill))
            .corner_radius(12.0)
            .min_size(vec2(132.0, 34.0)),
    )
}

fn ghost_button(ui: &mut Ui, label: &str, palette: &Palette) -> egui::Response {
    let width = (label.len() as f32 * 7.4 + 20.0).clamp(64.0, 120.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 8.0, palette.surface_hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.0),
        palette.muted,
    );
    response
}

fn centered_status(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(ui.available_size(), Layout::top_down(Align::Center), |ui| {
        ui.add_space(80.0);
        add(ui);
    });
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

fn paint_source_pill(painter: &Painter, left_center: Pos2, label: &str, palette: &Palette) {
    let text = source_pill_label(label);
    let width = (text.len() as f32 * 7.2 + 16.0).clamp(56.0, 140.0);
    let rect = Rect::from_center_size(
        pos2(left_center.x + width / 2.0, left_center.y),
        vec2(width, 20.0),
    );
    painter.rect_filled(rect, 6.0, palette.elevated);
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0_f32, palette.border),
        StrokeKind::Inside,
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(10.5),
        palette.muted,
    );
}

fn source_pill_label(source: &str) -> String {
    source.to_ascii_uppercase()
}

fn truncated_names(names: &[String], max_chars: usize) -> String {
    let joined = names.join(", ");
    if joined.chars().count() <= max_chars {
        return joined;
    }
    let mut output = String::new();
    for (index, ch) in joined.chars().enumerate() {
        if index + 1 >= max_chars {
            output.push('…');
            break;
        }
        output.push(ch);
    }
    output
}

fn task_status_label(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Queued => "Queued",
        TaskStatus::Running => "Running",
        TaskStatus::Success => "Done",
        TaskStatus::Failed => "Failed",
    }
}

fn task_status_color(status: TaskStatus, palette: &Palette) -> Color32 {
    match status {
        TaskStatus::Queued => palette.muted,
        TaskStatus::Running => palette.accent,
        TaskStatus::Success => palette.accent,
        TaskStatus::Failed => palette.danger,
    }
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
        Stroke::new(1.8_f32, palette.text),
        StrokeKind::Inside,
    );
    painter.line_segment(
        [
            pos2(body.left(), body.top() + 6.0),
            pos2(body.right(), body.top() + 6.0),
        ],
        Stroke::new(1.4_f32, palette.text),
    );
    painter.rect_stroke(
        Rect::from_min_size(
            pos2(rect.center().x - 5.0, rect.top() + 4.0),
            vec2(10.0, 6.0),
        ),
        2.0,
        Stroke::new(1.5_f32, palette.text),
        StrokeKind::Inside,
    );
}

fn paint_page_symbol(painter: &Painter, rect: Rect, page: Page, color: Color32) {
    let stroke = Stroke::new(1.7_f32, color);
    match page {
        Page::Home => {
            let peak = pos2(rect.center().x, rect.top() + 2.0);
            painter.line_segment([pos2(rect.left() + 2.0, rect.center().y), peak], stroke);
            painter.line_segment([peak, pos2(rect.right() - 2.0, rect.center().y)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(
                    pos2(rect.left() + 5.0, rect.center().y),
                    pos2(rect.right() - 5.0, rect.bottom() - 1.0),
                ),
                1.5,
                stroke,
                StrokeKind::Inside,
            );
        }
        Page::Apps => {
            let r = 3.2;
            let inset = 4.0;
            painter.circle_filled(pos2(rect.left() + inset, rect.top() + inset), r, color);
            painter.circle_filled(pos2(rect.right() - inset, rect.top() + inset), r, color);
            painter.circle_filled(pos2(rect.left() + inset, rect.bottom() - inset), r, color);
            painter.circle_filled(pos2(rect.right() - inset, rect.bottom() - inset), r, color);
        }
        Page::Recipes => {
            painter.rect_stroke(rect.shrink(2.0), 2.0, stroke, StrokeKind::Inside);
            painter.line_segment(
                [
                    pos2(rect.left() + 6.0, rect.top() + 7.0),
                    pos2(rect.right() - 6.0, rect.top() + 7.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 6.0, rect.center().y),
                    pos2(rect.right() - 6.0, rect.center().y),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 6.0, rect.bottom() - 7.0),
                    pos2(rect.right() - 8.0, rect.bottom() - 7.0),
                ],
                stroke,
            );
        }
        Page::System => {
            painter.circle_stroke(rect.center(), rect.width() * 0.28, stroke);
            painter.line_segment(
                [rect.center(), pos2(rect.center().x, rect.top() + 2.0)],
                stroke,
            );
            painter.line_segment(
                [rect.center(), pos2(rect.right() - 2.0, rect.center().y)],
                stroke,
            );
        }
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
        "Brave Browser" => "Privacy-focused browser with built-in shields.",
        "Brave Origin" => "Native Origin browser",
        "Google Chrome" => "The web browser from Google",
        "Firefox" => "Web browser from Mozilla.",
        "Microsoft Edge" => "Microsoft's web browser",
        "Opera" => "Fast and secure browser",
        "Vivaldi" => "Customizable web browser",
        "Zen" => "Modern Firefox-based browser",
        "Discord" => "Voice and text chat for communities.",
        "Signal" => "Private messenger.",
        "Slack" => "Team communication hub",
        "Thunderbird" => "Email client from Mozilla",
        "Visual Studio Code" => "Code editor redefined",
        "PyCharm Community" => "Python IDE",
        "Bottles" => "Run Windows apps on Linux",
        "Boxes" => "Simple VM manager",
        "Steam" => "Games library and storefront.",
        "Lutris" => "Open gaming platform",
        "ProtonUp-Qt" => "Manage Proton versions",
        "Sober" => "Linux Roblox port",
        "Spotify" => "Music streaming",
        "Heroic" => "Epic, GOG, and Amazon games",
        "Flatseal" => "Flatpak permissions UI",
        "Telegram" => "Messaging app",
        "Prism Launcher" => "Minecraft launcher",
        "Obsidian" => "Linked notes and knowledge base.",
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
        "GIMP" => "Image editing and compositing.",
        "VLC" => "Media player",
        "mpv" => "Video player",
        "Audacity" => "Audio editor",
        "LibreOffice" => "Office suite",
        "OnlyOffice" => "Document editors",
        "OBS Studio" => "Live streaming and recording.",
        "GParted" => "Partition editor",
        "htop" => "Interactive process viewer",
        "LocalSend" => "Share files across devices on LAN.",
        _ => "Application from the Toolbox catalog",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn production_main() -> &'static str {
        include_str!("main.rs")
            .split("#[cfg(test)]")
            .next()
            .expect("main module")
    }

    fn sample_app(label: &str, package: &str) -> AppEntry {
        AppEntry {
            category: "Browsers".to_owned(),
            label: label.to_owned(),
            package_name: package.to_owned(),
            flatpak_id: String::new(),
            exec_name: package.to_owned(),
            notes: String::new(),
        }
    }

    fn sample_removable(label: &str, detail: &str, is_toolbox: bool) -> RemovableApp {
        RemovableApp {
            label: label.to_owned(),
            source: RemovableSource::Native,
            detail: detail.to_owned(),
            exec: Some(detail.to_owned()),
            is_toolbox,
        }
    }

    fn test_app() -> ToolboxApp {
        ToolboxApp {
            base_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            page: Page::Apps,
            apps_mode: AppsMode::Install,
            apps: vec![sample_app("Firefox", "firefox"), sample_app("VLC", "vlc")],
            admin_tasks: vec![AdminTask {
                category: "System".to_owned(),
                label: "Update System".to_owned(),
                script: "update-system.sh".to_owned(),
            }],
            install_selected: HashSet::new(),
            remove_selected: HashSet::new(),
            admin_selected: HashSet::new(),
            install_search: String::new(),
            install_category: None,
            remove_search: String::new(),
            distro_name: "Debian".to_owned(),
            package_manager: "apt-get".to_owned(),
            host: "testhost".to_owned(),
            kernel: "6.12.0-test".to_owned(),
            shell: "/bin/bash".to_owned(),
            de_wm: "sway".to_owned(),
            recent_runs: Vec::new(),
            log: String::new(),
            password: String::new(),
            show_password_modal: false,
            is_running: false,
            tx: None,
            rx: None,
            log_revision: 0,
            run_drawer_open: false,
            run_items: Vec::new(),
            icons: HashMap::new(),
            theme: ThemeMode::Dark,
            remove_apps: vec![
                sample_removable("htop", "htop", false),
                sample_removable("Toolbox", "linux-it-guy-toolbox", true),
            ],
            remove_scan: RemoveScanState::Ready,
            remove_scan_rx: None,
        }
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
        let scan_fn = src
            .split("fn start_remove_scan")
            .nth(1)
            .and_then(|rest| rest.split("fn drain_remove_scan").next())
            .expect("start_remove_scan body");
        assert!(scan_fn.contains("remove_selected.clear()"));
        assert!(
            !scan_fn.contains("install_selected.clear()"),
            "Remove rescan must not clear install selections"
        );
        assert!(
            !scan_fn.contains("admin_selected.clear()"),
            "Remove rescan must not clear admin selections"
        );
        assert!(src.contains("clear_all_selections"));
        assert!(src.contains("push_install_tasks"));
        assert!(src.contains("push_remove_tasks"));
        assert!(src.contains("push_admin_tasks"));
    }

    #[test]
    fn run_queue_uses_active_apps_mode_only() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.admin_selected.insert(0);

        app.apps_mode = AppsMode::Install;
        assert_eq!(app.staged_count(app.apps_mode), 1);
        assert_eq!(app.modal_count_line(), "This will install 1 app.");
        let tasks = app.selected_tasks().expect("install tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Installing Firefox");
        assert_eq!(tasks[0].command.last().map(String::as_str), Some("install"));

        app.apps_mode = AppsMode::Remove;
        assert_eq!(app.staged_count(app.apps_mode), 1);
        assert_eq!(app.modal_count_line(), "This will remove 1 app.");
        let tasks = app.selected_tasks().expect("remove tasks");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Removing htop");
        assert_eq!(tasks[0].command.last().map(String::as_str), Some("remove"));
        assert!(app.install_selected.contains(&0));
    }

    #[test]
    fn review_lists_active_mode_names_grouped() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.install_selected.insert(1);
        app.remove_selected.insert(0);

        app.apps_mode = AppsMode::Install;
        assert_eq!(
            app.review_groups(),
            vec![("Install", vec!["Firefox".into(), "VLC".into()])]
        );
        assert_eq!(
            app.other_mode_staged_note().as_deref(),
            Some("1 also staged in Remove. Switch to Remove to run those.")
        );

        app.apps_mode = AppsMode::Remove;
        assert_eq!(app.review_groups(), vec![("Remove", vec!["htop".into()])]);
        assert_eq!(
            app.other_mode_staged_note().as_deref(),
            Some("2 also staged in Install. Switch to Install to run those.")
        );
        assert!(app.remove_includes_toolbox().is_none());
        app.remove_selected.insert(1);
        assert!(app.remove_includes_toolbox().is_some());
    }

    #[test]
    fn review_modal_shows_grouped_names_before_password() {
        let src = production_main();
        let groups = src
            .split("fn review_groups")
            .nth(1)
            .and_then(|rest| rest.split("fn other_mode_staged_note").next())
            .expect("review_groups body");
        assert!(groups.contains("\"Install\""));
        assert!(groups.contains("\"Remove\""));
        assert!(groups.contains("\"Recipes\""));
        let modal = src
            .split("fn sudo_modal")
            .nth(1)
            .and_then(|rest| rest.split("impl eframe::App").next())
            .expect("sudo_modal body");
        assert!(modal.contains("Review & Run"));
        assert!(modal.contains("review_groups"));
        assert!(modal.contains("sudo_name_list"));
        let name_list = modal.find("sudo_name_list").expect("name list");
        let password = modal.find("Sudo password").expect("password label");
        assert!(
            name_list < password,
            "staged names must appear before the sudo password field"
        );
    }

    #[test]
    fn switching_apps_mode_does_not_clear_other_selection() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.admin_selected.insert(0);
        app.install_search = "firefox".to_owned();
        app.install_category = Some("Browsers".to_owned());
        app.remove_search = "htop".to_owned();

        app.apps_mode = AppsMode::Remove;
        assert_eq!(app.page, Page::Apps);
        assert_eq!(app.install_search, "firefox");
        assert_eq!(app.install_category.as_deref(), Some("Browsers"));
        assert_eq!(app.remove_search, "htop");
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.staged_count(AppsMode::Install), 1);
        assert_eq!(app.staged_count(AppsMode::Remove), 1);

        app.apps_mode = AppsMode::Install;
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.staged_count(app.apps_mode), 1);

        app.switch_page(Page::Home);
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        assert!(!app.dock_visible());
    }

    #[test]
    fn dock_clear_only_clears_active_mode() {
        let mut app = test_app();
        app.apps_mode = AppsMode::Install;
        app.install_selected.insert(0);
        app.install_selected.insert(1);
        app.remove_selected.insert(0);
        app.admin_selected.insert(0);

        app.clear_active_mode_selections();
        assert!(app.install_selected.is_empty());
        assert!(app.remove_selected.contains(&0));
        assert!(app.admin_selected.contains(&0));
        assert_eq!(app.staged_count(AppsMode::Remove), 1);
        assert!(!app.dock_visible());

        app.apps_mode = AppsMode::Remove;
        assert!(app.dock_visible());
        app.clear_active_mode_selections();
        assert!(app.remove_selected.is_empty());
        assert!(!app.dock_visible());
    }

    #[test]
    fn dock_is_hidden_at_rest_and_while_running() {
        let mut app = test_app();
        assert!(!app.dock_visible());
        app.install_selected.insert(0);
        assert!(app.dock_visible());
        app.is_running = true;
        assert!(!app.dock_visible());
        assert!(app.selection_locked());
        app.is_running = false;
        app.switch_page(Page::System);
        assert!(!app.dock_visible());
    }

    #[test]
    fn run_freezes_selections_instead_of_clearing_them() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.is_running = true;
        assert!(app.selection_locked());
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        app.clear_all_selections();
        assert!(app.install_selected.is_empty());
        assert!(app.remove_selected.is_empty());
        assert!(app.admin_selected.is_empty());
    }

    #[test]
    fn source_pills_are_uppercase() {
        assert_eq!(source_pill_label("native"), "NATIVE");
        assert_eq!(source_pill_label("flatpak"), "FLATPAK");
        assert_eq!(source_pill_label("native + flatpak"), "NATIVE + FLATPAK");
    }

    #[test]
    fn truncated_names_keep_short_lists() {
        let names = vec!["Brave".into(), "Discord".into(), "Firefox".into()];
        assert_eq!(truncated_names(&names, 42), "Brave, Discord, Firefox");
        assert!(truncated_names(&names, 12).ends_with('…'));
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
    fn card_columns_use_content_width_breakpoint() {
        assert_eq!(CONTENT_TWO_COL, 720.0);
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
    fn chrome_uses_apps_mode_and_lumen_rail() {
        let src = production_main();
        assert!(src.contains("enum AppsMode"));
        assert!(src.contains("Page::Apps"));
        assert!(!src.contains("Page::Install"));
        assert!(!src.contains("Page::Remove"));
        assert!(!src.contains("Page::Administration"));
        assert!(!src.contains("Supported Distros"));
        assert!(!src.contains("Select All"));
        assert!(src.contains("Review & Run"));
        assert!(src.contains("run_dock"));
        assert_eq!(RAIL_WIDTH, 68.0);
        assert_eq!(CARD_HEIGHT, 164.0);
        assert_eq!(TITLE_SIZE, 26.0);
        assert!(src.contains("fn home_page"));
        assert!(src.contains("fn recipes_page"));
        assert!(!src.contains("fn stub_page"));
        assert!(!src.contains("Home is coming"));
        assert!(!src.contains("Recipes is coming"));
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

    #[test]
    fn retired_brand_hexes_are_gone_from_main_chrome() {
        let src = production_main();
        for retired in ["#E9FC12", "E9FC12", "#1A365D", "1A365D", "#CDEDFE"] {
            assert!(!src.contains(retired), "{retired}");
        }
    }

    #[test]
    fn home_intents_navigate_and_preselect_recipes() {
        let mut app = test_app();
        app.admin_tasks = admin_tasks();

        app.go_install_apps();
        assert_eq!(app.page, Page::Apps);
        assert_eq!(app.apps_mode, AppsMode::Install);

        app.go_fresh_setup();
        assert_eq!(app.page, Page::Recipes);
        let selected = app.admin_selected_names();
        assert!(selected.contains(&"Update System".to_owned()));
        assert!(selected.contains(&"Fastfetch".to_owned()));
        assert_eq!(selected.len(), 2);
        assert!(app.dock_visible());

        app.go_laptop_power();
        let selected = app.admin_selected_names();
        assert!(selected.contains(&"TLP (Laptops)".to_owned()));
        assert!(selected.contains(&"Powertop".to_owned()));
        assert_eq!(selected.len(), 2);
        assert!(!selected.contains(&"Update System".to_owned()));
    }

    #[test]
    fn home_glance_uses_live_fields_not_fake_cpu() {
        let app = test_app();
        let metrics = app.glance_metrics();
        assert_eq!(metrics[0].0, "OS");
        assert_eq!(metrics[1].0, "Kernel");
        assert_eq!(metrics[2].0, "Uptime");
        assert_eq!(metrics[3].0, "PM");
        assert_eq!(metrics[0].1, "Debian");
        assert_eq!(metrics[1].1, "6.12.0-test");
        assert_eq!(metrics[3].1, "apt-get");
        let src = production_main();
        assert!(src.contains("SYSTEM GLANCE") || src.contains("System glance"));
        assert!(!src.contains("\"CPU\""));
        assert!(!src.contains("No runs yet") || src.contains("stage apps or a recipe"));
        assert!(src.contains("No runs yet — stage apps or a recipe to get started."));
        let home = src
            .split("fn home_page")
            .nth(1)
            .and_then(|rest| rest.split("fn recent_runs_card").next())
            .expect("home_page");
        assert!(!home.contains("checkbox"));
        assert!(home.contains("go_install_apps"));
        assert!(home.contains("go_fresh_setup"));
        assert!(home.contains("go_laptop_power"));
        assert!(home.contains("go_system"));
    }

    #[test]
    fn recipes_stage_into_run_dock_and_review_group() {
        let mut app = test_app();
        app.admin_tasks = admin_tasks();
        app.install_selected.insert(0);
        app.page = Page::Recipes;
        app.admin_selected.insert(4);
        app.admin_selected.insert(8);

        assert!(app.dock_visible());
        assert_eq!(app.dock_count(), 2);
        assert_eq!(app.modal_count_line(), "This will run 2 recipes.");
        assert_eq!(
            app.review_groups(),
            vec![("Recipes", vec!["Fastfetch".into(), "Update System".into()])]
        );
        assert!(app.other_mode_staged_note().is_none());

        let tasks = app.selected_tasks().expect("recipe tasks");
        assert_eq!(tasks.len(), 2);
        assert!(
            tasks
                .iter()
                .all(|task| task.description.starts_with("Running "))
        );
        assert!(app.install_selected.contains(&0));

        app.clear_dock_selections();
        assert!(app.admin_selected.is_empty());
        assert!(app.install_selected.contains(&0));
        assert!(!app.dock_visible());
    }

    #[test]
    fn recipes_dock_hides_while_running_and_locks_selection() {
        let mut app = test_app();
        app.page = Page::Recipes;
        app.admin_selected.insert(0);
        assert!(app.dock_visible());
        app.is_running = true;
        assert!(!app.dock_visible());
        assert!(app.selection_locked());
        assert!(app.admin_selected.contains(&0));
        app.switch_page(Page::Home);
        app.is_running = false;
        assert!(!app.dock_visible());
        assert!(app.admin_selected.contains(&0));
    }

    #[test]
    fn every_admin_task_has_a_promise_and_optional_chip() {
        for task in admin_tasks() {
            let promise = recipe_promise(&task.label);
            assert_ne!(
                promise, "Admin playbook for this machine.",
                "{}",
                task.label
            );
            match task.label.as_str() {
                "Update System" | "SWAP Fix" => {
                    assert_eq!(recipe_chip(&task.label), Some("Caution"));
                }
                other if other.contains("Debian only") => {
                    assert_eq!(recipe_chip(&task.label), Some("Debian only"));
                }
                _ => assert_eq!(recipe_chip(&task.label), None),
            }
        }
        assert_eq!(admin_tasks().len(), 9);
        let groups = {
            let app = {
                let mut app = test_app();
                app.admin_tasks = admin_tasks();
                app
            };
            app.recipe_groups()
        };
        assert_eq!(
            groups
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["Power Management", "System"]
        );
    }

    #[test]
    fn system_detail_keeps_live_fields_and_copy_all() {
        let app = test_app();
        let rows = app.system_detail_rows();
        assert_eq!(rows[0].0, "OS");
        assert_eq!(rows[1].0, "Host");
        assert_eq!(rows[2].0, "Kernel");
        assert_eq!(rows[3].0, "Uptime");
        assert_eq!(rows[4].0, "Shell");
        assert_eq!(rows[5].0, "DE/WM");
        assert_eq!(rows[6].0, "Package Manager");
        assert_eq!(rows[7].0, "App Directory");
        assert_eq!(rows[1].1, "testhost");
        assert_eq!(rows[4].1, "/bin/bash");
        let src = production_main();
        let system = src
            .split("fn system_info_page")
            .nth(1)
            .and_then(|rest| rest.split("fn modal_count_line").next())
            .expect("system_info_page");
        assert!(system.contains("Copy all"));
        assert!(system.contains("copy_system_details"));
        assert!(!system.contains("command_output"));
        assert_eq!(greeting_for_hour(7), "Good morning");
        assert_eq!(greeting_for_hour(13), "Good afternoon");
        assert_eq!(greeting_for_hour(21), "Good evening");
        assert_eq!(greeting_for_hour(2), "Good evening");
    }

    #[test]
    fn apps_and_recipes_reserve_dock_padding() {
        let mut app = test_app();
        assert!(app.dock_can_appear());
        app.switch_page(Page::Recipes);
        assert!(app.dock_can_appear());
        app.switch_page(Page::Home);
        assert!(!app.dock_can_appear());
        app.switch_page(Page::System);
        assert!(!app.dock_can_appear());
        let src = production_main();
        assert!(src.contains("bottom: if self.dock_can_appear() { 120 } else { 40 }"));
    }

    #[test]
    fn recent_runs_record_session_history_without_fake_rows() {
        let mut app = test_app();
        assert!(app.recent_runs.is_empty());
        app.install_selected.insert(0);
        app.record_recent_run();
        assert_eq!(app.recent_runs.len(), 1);
        assert_eq!(app.recent_runs[0].kind, "Install");
        assert!(app.recent_runs[0].summary.contains("Firefox"));

        app.page = Page::Recipes;
        app.admin_selected.insert(0);
        app.record_recent_run();
        assert_eq!(app.recent_runs[0].kind, "Recipe");
        assert_eq!(app.recent_runs.len(), 2);
    }
}
