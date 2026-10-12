mod catalog;
mod cursor;
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
    FontFamily, FontId, Frame, Id, Key, Layout, Order, Painter, Panel, Pos2, Rect, RichText,
    ScrollArea, Sense, Stroke, StrokeKind, TextEdit, TextureHandle, TextureOptions, Ui, pos2, vec2,
};

use catalog::{
    AdminTask, AppEntry, BaseDirSource, CatalogLoad, Task, admin_tasks, find_base_dir, load_apps,
};
use cursor::CursorTheme;
use runner::{RunnerMessage, run_tasks, tasks_need_privileges};
use system::{
    PackageManager, RemovableApp, RemovableSource, command_output, detect_package_manager,
    distro_name, env_or_unknown, scan_removable_apps, strip_ansi, uptime_compact, uptime_prose,
};
use theme::{Palette, ThemeFamily, ThemeMode};
use validate::{Action, is_exec_name, is_label, zeroize_string};

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
    Cursor,
    System,
}

impl Page {
    fn title(self) -> &'static str {
        match self {
            Page::Home => "Home",
            Page::Apps => "Apps",
            Page::Recipes => "Recipes",
            Page::Cursor => "Cursor",
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
const TOPBAR_HEIGHT: f32 = 60.0;
const CARD_HEIGHT: f32 = 178.0;
const CARD_ICON: f32 = 40.0;
const CARD_GAP: f32 = 14.0;
const CARD_RADIUS: f32 = 0.0;
const CHECK_SIZE: f32 = 22.0;
const SEARCH_HEIGHT: f32 = 48.0;
const DOCK_MIN_WIDTH: f32 = 520.0;
const RUN_DRAWER_WIDTH: f32 = 460.0;
const DOCK_RADIUS: f32 = 0.0;
const CONTENT_TWO_COL: f32 = 720.0;
const INTENT_HEIGHT: f32 = 190.0;
const GLANCE_HEIGHT: f32 = 84.0;
const RECIPE_ROW_HEIGHT: f32 = 84.0;
const CURSOR_CARD_HEIGHT: f32 = 306.0;
const TITLE_SIZE: f32 = 44.0;
const LAPTOP_POWER_LABELS: &[&str] = &["TLP (Laptops)", "Powertop"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Glyph {
    /// Same mark as the matching top-bar tab.
    Tab(Page),
    Bolt,
    Battery,
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
    cursor_selected: Option<CursorTheme>,
    direct_cursor: Option<String>,
    install_search: String,
    install_category: Option<String>,
    remove_search: String,
    distro_name: String,
    package_manager: PackageManager,
    kernel: String,
    de_wm: String,
    recent_runs: Vec<RecentRun>,
    log: String,
    password: String,
    show_password_modal: bool,
    show_run_complete_modal: bool,
    is_running: bool,
    tx: Option<Sender<RunnerMessage>>,
    rx: Option<Receiver<RunnerMessage>>,
    log_revision: u64,
    run_drawer_open: bool,
    run_items: Vec<RunItem>,
    icons: HashMap<&'static str, TextureHandle>,
    family: ThemeFamily,
    theme: ThemeMode,
    remove_apps: Vec<RemovableApp>,
    remove_scan: RemoveScanState,
    remove_scan_rx: Option<Receiver<Result<Vec<RemovableApp>, String>>>,
    toast: Option<(String, std::time::Instant)>,
    hardware: system::HardwareSnapshot,
}

impl ToolboxApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let preference = theme::load_theme();
        theme::apply_theme(&cc.egui_ctx, preference.family, preference.mode);
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
        if discovery.source == Some(BaseDirSource::WorkingDirectory) {
            log = format!(
                "[WARN] App catalog loaded from the current working directory ({}). Only run Toolbox from a checkout you trust; helper scripts in that folder can call sudo.",
                base_dir.display()
            );
        }
        if let Some(error) = &catalog.error {
            if log == "Process logs will appear here..." {
                log = format!("[ERROR] {error}");
            } else {
                log.push('\n');
                log.push_str("[ERROR] ");
                log.push_str(error);
            }
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
            cursor_selected: None,
            direct_cursor: None,
            install_search: String::new(),
            install_category: None,
            remove_search: String::new(),
            distro_name,
            package_manager,
            kernel: command_output("uname", &["-r"]),
            de_wm: env_or_unknown(&["XDG_CURRENT_DESKTOP", "DESKTOP_SESSION"]),
            recent_runs: Vec::new(),
            log,
            password: String::new(),
            show_password_modal: false,
            show_run_complete_modal: false,
            is_running: false,
            tx: None,
            rx: None,
            log_revision: 0,
            run_drawer_open: false,
            run_items: Vec::new(),
            icons,
            family: preference.family,
            theme: preference.mode,
            remove_apps: Vec::new(),
            remove_scan: RemoveScanState::Idle,
            remove_scan_rx: None,
            toast: None,
            hardware: system::HardwareSnapshot::default(),
        }
    }

    fn palette(&self) -> Palette {
        Palette::from(self.family, self.theme)
    }

    fn persist_theme(&mut self, ctx: &Context) {
        theme::apply_theme(ctx, self.family, self.theme);
        if let Err(error) = theme::save_theme(self.family, self.theme) {
            if self.log == "Process logs will appear here..." {
                self.log.clear();
            }
            self.log.push_str(&format!(
                "\n[WARN] Could not save theme preference: {error}"
            ));
            self.log_revision = self.log_revision.saturating_add(1);
        }
    }

    /// Picks one of the five skins: four dark families, or the white Light skin.
    fn set_skin(&mut self, ctx: &Context, family: ThemeFamily, mode: ThemeMode) {
        if self.family == family && self.theme == mode {
            return;
        }
        self.family = family;
        self.theme = mode;
        self.persist_theme(ctx);
    }

    fn show_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), std::time::Instant::now()));
    }

    fn switch_page(&mut self, page: Page) {
        if page == Page::System && self.page != Page::System {
            self.hardware = system::HardwareSnapshot::read();
        }
        self.page = page;
    }

    fn staged_count(&self, mode: AppsMode) -> usize {
        match mode {
            AppsMode::Install => self.install_selected.len(),
            AppsMode::Remove => self.remove_selected.len(),
        }
    }

    fn total_staged_count(&self) -> usize {
        self.staged_count(AppsMode::Install)
            + self.staged_count(AppsMode::Remove)
            + self.admin_selected.len()
            + usize::from(self.cursor_selected.is_some())
    }

    fn dock_visible(&self) -> bool {
        !self.is_running && self.total_staged_count() > 0
    }

    fn dock_count(&self) -> usize {
        self.total_staged_count()
    }

    fn dock_names(&self) -> Vec<String> {
        let mut names = self.install_selected_names();
        names.extend(self.remove_selected_names());
        if let Some(theme) = self.cursor_selected {
            names.push(theme.label().to_owned());
        }
        names.extend(self.admin_selected_names());
        names
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
        let mut indices: Vec<usize> = self.admin_selected.iter().copied().collect();
        indices.sort_unstable();
        indices
            .into_iter()
            .filter_map(|index| {
                self.admin_tasks
                    .get(index)
                    .map(|task| recipe_display_label(&task.label).to_owned())
            })
            .collect()
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
                match entry.try_command(&self.base_dir, Action::Install) {
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

    fn push_cursor_task(&self, tasks: &mut Vec<Task>, errors: &mut Vec<String>) {
        let Some(theme) = self.cursor_selected else {
            return;
        };
        match theme.try_task(&self.base_dir) {
            Ok(task) => tasks.push(task),
            Err(error) => errors.push(format!("{}: {error}", theme.label())),
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
        self.push_install_tasks(&mut tasks, &mut errors);
        self.push_remove_tasks(&mut tasks, &mut errors);
        self.push_cursor_task(&mut tasks, &mut errors);
        self.push_admin_tasks(&mut tasks, &mut errors);
        if errors.is_empty() {
            Ok(tasks)
        } else {
            Err(errors)
        }
    }

    fn clear_all_selections(&mut self) {
        self.install_selected.clear();
        self.remove_selected.clear();
        self.admin_selected.clear();
        self.cursor_selected = None;
    }

    fn clear_dock_selections(&mut self) {
        self.clear_all_selections();
    }

    fn preselect_admin_labels(&mut self, labels: &[&str]) {
        self.admin_selected.clear();
        for (index, task) in self.admin_tasks.iter().enumerate() {
            if labels.contains(&task.label.as_str()) {
                self.admin_selected.insert(index);
            }
        }
    }

    fn toggle_cursor(&mut self, theme: CursorTheme) {
        if self.selection_locked() {
            return;
        }
        if self.cursor_selected == Some(theme) {
            self.cursor_selected = None;
        } else {
            self.cursor_selected = Some(theme);
        }
    }

    fn go_install_apps(&mut self) {
        self.apps_mode = AppsMode::Install;
        self.switch_page(Page::Apps);
    }

    fn go_fresh_setup(&mut self) {
        self.switch_page(Page::Recipes);
    }

    fn go_laptop_power(&mut self) {
        self.preselect_admin_labels(LAPTOP_POWER_LABELS);
        self.show_toast("Added the battery recipes to your queue");
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
        let package_manager = self.package_manager;
        let (tx, rx) = mpsc::channel();
        self.remove_scan_rx = Some(rx);
        let repaint = ctx.clone();
        thread::spawn(move || {
            let result = scan_removable_apps(package_manager);
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
        self.direct_cursor = None;

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

        let password = std::mem::take(&mut self.password);
        self.start_tasks(ctx, tasks, password);
    }

    fn apply_cursor_now(&mut self, ctx: &Context, theme: CursorTheme) {
        if self.is_running {
            return;
        }
        let task = match theme.try_task(&self.base_dir) {
            Ok(task) => task,
            Err(error) => {
                self.append_log(&format!("[ERROR] {error}"));
                return;
            }
        };
        self.direct_cursor = Some(theme.label().to_owned());
        self.start_tasks(ctx, vec![task], String::new());
    }

    fn start_tasks(&mut self, ctx: &Context, tasks: Vec<Task>, password: String) {
        let (tx, rx) = mpsc::channel();
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

        let base_dir = self.base_dir.clone();
        let repaint = ctx.clone();
        thread::spawn(move || {
            run_tasks(tasks, password, tx, base_dir);
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
            self.finish_run();
        }
    }

    fn finish_run(&mut self) {
        if let Some(label) = self.direct_cursor.take() {
            self.show_toast(format!(
                "Cursor changed to {}",
                cursor_short_name_for(&label)
            ));
            self.recent_runs.insert(
                0,
                RecentRun {
                    kind: "Cursor".to_owned(),
                    summary: label,
                },
            );
            self.recent_runs.truncate(8);
        } else {
            self.record_recent_run();
            self.clear_all_selections();
        }
        self.is_running = false;
        zeroize_string(&mut self.password);
        self.tx = None;
        self.rx = None;
        self.log.push_str("\n\n[DONE] All tasks finished.");
        self.log_revision = self.log_revision.saturating_add(1);
        self.show_run_complete_modal = true;
    }

    fn record_recent_run(&mut self) {
        let mut entries = Vec::new();
        let install = self.install_selected_names();
        if !install.is_empty() {
            entries.push(("Install", install));
        }
        let remove = self.remove_selected_names();
        if !remove.is_empty() {
            entries.push(("Remove", remove));
        }
        if let Some(theme) = self.cursor_selected {
            entries.push(("Cursor", vec![theme.label().to_owned()]));
        }
        let recipes = self.admin_selected_names();
        if !recipes.is_empty() {
            entries.push(("Recipe", recipes));
        }
        // Insert in reverse so Install appears above Remove/Recipe for a mixed run.
        for (kind, names) in entries.into_iter().rev() {
            self.recent_runs.insert(
                0,
                RecentRun {
                    kind: kind.to_owned(),
                    summary: truncated_names(&names, 48),
                },
            );
        }
        self.recent_runs.truncate(8);
    }

    fn topbar(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let mut next_family = None;
        let mut next_light = false;
        let mut open_queue = false;
        let queue_count = if self.dock_visible() {
            self.dock_count()
        } else {
            0
        };
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let compact = ui.available_width() < 1100.0;
            match self.icons.get("toolbox-logo") {
                Some(logo) => {
                    let (rect, _) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
                    ui.painter().image(
                        logo.id(),
                        rect,
                        Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }
                None => toolbox_icon(ui, 30.0, &palette),
            }
            if !compact {
                ui.add_space(10.0);
                ui.label(
                    RichText::new("Toolbox")
                        .font(FontId::proportional(20.0))
                        .color(palette.text)
                        .strong(),
                );
            }
            ui.add_space(if compact { 12.0 } else { 36.0 });
            for page in [
                Page::Home,
                Page::Apps,
                Page::Recipes,
                Page::Cursor,
                Page::System,
            ] {
                let badge = if page == Page::Apps {
                    self.install_selected.len() + self.remove_selected.len()
                } else {
                    0
                };
                if tab_button(ui, page, self.page == page, badge, compact, &palette).clicked() {
                    self.switch_page(page);
                    if page == Page::Apps && self.apps_mode == AppsMode::Remove {
                        let ctx = ui.ctx().clone();
                        self.start_remove_scan(&ctx);
                    }
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                if light_swatch(ui, self.theme == ThemeMode::Light, &palette).clicked() {
                    next_light = true;
                }
                for family in ThemeFamily::ALL.into_iter().rev().filter(|_| !compact) {
                    let selected = self.theme == ThemeMode::Dark && family == self.family;
                    if family_swatch(ui, family, selected, &palette).clicked() {
                        next_family = Some(family);
                    }
                }
                if queue_count > 0 {
                    ui.add_space(8.0);
                    if queue_button(ui, queue_count, &palette).clicked() {
                        open_queue = true;
                    }
                }
            });
        });
        let ctx = ui.ctx().clone();
        if next_light {
            self.set_skin(&ctx, ThemeFamily::Blue, ThemeMode::Light);
        }
        if let Some(family) = next_family {
            self.set_skin(&ctx, family, ThemeMode::Dark);
        }
        if open_queue {
            self.show_password_modal = true;
        }
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
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("Apps")
                        .font(FontId::proportional(TITLE_SIZE))
                        .color(palette.text)
                        .strong(),
                );
                ui.label(
                    RichText::new(match self.apps_mode {
                        AppsMode::Install => {
                            "Tick the apps you want. Nothing installs until you press Run."
                        }
                        AppsMode::Remove => "Tick the apps you want to take off this computer.",
                    })
                    .color(palette.muted),
                );
            });
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
            AppsMode::Install => "Search apps by name…",
            AppsMode::Remove => "Search installed apps by name…",
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
            AppCard {
                width,
                selected,
                label: &label,
                source: Some(source),
                description: &notes,
                icon: icon.as_ref(),
                palette: &palette,
            },
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
            AppCard {
                width,
                selected,
                label: &label,
                source: Some(source),
                description: &detail,
                icon: icon.as_ref(),
                palette: &palette,
            },
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
        format!("{} is ready.", self.distro_name)
    }

    fn glance_metrics(&self) -> [(&'static str, String); 4] {
        [
            ("Operating system", self.distro_name.clone()),
            ("Kernel", self.kernel.clone()),
            ("Running for", uptime_compact()),
            ("Installs with", self.package_manager.to_string()),
        ]
    }

    fn home_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ScrollArea::vertical()
            .id_salt("home_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("What would you like to do?")
                        .font(FontId::proportional(TITLE_SIZE))
                        .color(palette.text)
                        .strong(),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
                    ui.painter().rect_filled(dot, 0.0, palette.accent);
                    ui.add_space(4.0);
                    ui.label(RichText::new(self.ready_line()).color(palette.muted));
                });
                ui.add_space(28.0);

                let intents = [
                    (
                        Glyph::Tab(Page::Apps),
                        "Install apps",
                        "Browse the catalog and pick what you need.",
                        "Browse apps",
                    ),
                    (
                        Glyph::Tab(Page::Recipes),
                        "Set up a new computer",
                        "One-click recipes for updates, Bluetooth and essentials.",
                        "See recipes",
                    ),
                    (
                        Glyph::Battery,
                        "Make your battery last",
                        "TLP and Powertop tuned for laptops.",
                        "Add to queue",
                    ),
                ];
                let columns = intent_columns(ui);
                let width = tile_width_gap(ui, columns, 22.0);
                let mut chosen = None;
                for (row_index, row) in intents.chunks(columns).enumerate() {
                    if row_index > 0 {
                        ui.add_space(22.0);
                    }
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 22.0;
                        for (offset, (glyph, title, body, cta)) in row.iter().enumerate() {
                            let index = row_index * columns + offset;
                            if intent_card(ui, width, *glyph, title, body, cta, &palette).clicked()
                            {
                                chosen = Some(index);
                            }
                        }
                    });
                }
                match chosen {
                    Some(0) => self.go_install_apps(),
                    Some(1) => self.go_fresh_setup(),
                    Some(2) => self.go_laptop_power(),
                    _ => {}
                }

                ui.add_space(36.0);
                section_eyebrow(ui, "Your computer", &palette);
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

                ui.add_space(22.0);
                section_eyebrow(ui, "Recent activity", &palette);
                ui.add_space(10.0);
                self.recent_runs_card(ui, &palette);
            });
    }

    fn recent_runs_card(&self, ui: &mut Ui, palette: &Palette) {
        Frame::new()
            .stroke(Stroke::new(1.0_f32, palette.border))
            .inner_margin(egui::Margin::symmetric(18, 16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if self.recent_runs.is_empty() {
                    ui.label(
                        RichText::new(
                            "Nothing yet. Pick some apps or a recipe and they will show up here.",
                        )
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
        ui.label(
            RichText::new(
                "Ready-made fixes and setups. Switch on the ones you want, then run them together.",
            )
            .color(palette.muted),
        );
        ui.add_space(22.0);

        let mut order: Vec<usize> = (0..self.admin_tasks.len()).collect();
        order.sort_by_key(|index| (recipe_list_rank(&self.admin_tasks[*index].label), *index));
        ScrollArea::vertical()
            .id_salt("recipes_cards")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::new()
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing.y = 0.0;
                        for (row, index) in order.iter().enumerate() {
                            self.recipe_row(ui, *index, row > 0);
                        }
                    });
            });
    }

    fn recipe_row(&mut self, ui: &mut Ui, index: usize, divider: bool) {
        let task = &self.admin_tasks[index];
        let label = task.label.clone();
        let selected = self.admin_selected.contains(&index);
        let palette = self.palette();
        let locked = self.selection_locked();
        if recipe_row(ui, selected, divider, locked, &label, &palette) && !locked {
            if selected {
                self.admin_selected.remove(&index);
            } else {
                self.admin_selected.insert(index);
            }
        }
    }

    fn cursor_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        ui.label(
            RichText::new("Mouse cursor")
                .font(FontId::proportional(TITLE_SIZE))
                .color(palette.text)
                .strong(),
        );
        ui.label(
            RichText::new(
                "Choose a look. Use it right away, or add it to your queue with other changes.",
            )
            .color(palette.muted),
        );
        ui.add_space(22.0);

        let themes = CursorTheme::ALL;
        ScrollArea::vertical()
            .id_salt("cursor_cards")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let columns = cursor_columns(ui);
                self.card_rows(ui, columns, themes.len(), |app, ui, index, width| {
                    app.cursor_tile(ui, themes[index], width);
                });
            });
    }

    fn cursor_tile(&mut self, ui: &mut Ui, theme: CursorTheme, width: f32) {
        let selected = self.cursor_selected == Some(theme);
        let locked = self.selection_locked();
        let palette = self.palette();
        let preview = self.icons.get(theme.preview_key()).cloned();
        let (stage, apply) = cursor_card(
            ui,
            width,
            theme,
            selected,
            locked,
            preview.as_ref(),
            &palette,
        );
        if stage {
            self.toggle_cursor(theme);
        }
        if apply {
            let ctx = ui.ctx().clone();
            self.apply_cursor_now(&ctx, theme);
        }
    }

    fn system_info_page(&mut self, ui: &mut Ui) {
        let palette = self.palette();
        let hw = self.hardware.clone();
        let percent = |used: u64, total: u64| -> f32 {
            if total == 0 {
                0.0
            } else {
                (used as f32 / total as f32).clamp(0.0, 1.0)
            }
        };
        let cards: [SystemCard; 4] = [
            SystemCard {
                title: "System",
                lines: vec![
                    self.distro_name.clone(),
                    format!("Kernel {}", self.kernel),
                    format!("Desktop: {}", self.de_wm),
                    format!("Installs with {}", self.package_manager),
                ],
                bar: None,
            },
            SystemCard {
                title: "Processor",
                lines: vec![
                    hw.cpu_model.clone(),
                    format!("{} cores", hw.cpu_cores),
                    format!("Busy about {:.0}%", hw.cpu_load * 100.0),
                ],
                bar: Some(hw.cpu_load),
            },
            SystemCard {
                title: "Memory",
                lines: vec![
                    format!(
                        "{} of {} in use",
                        system::format_kb(hw.mem_used_kb),
                        system::format_kb(hw.mem_total_kb)
                    ),
                    format!(
                        "Swap: {} of {}",
                        system::format_kb(hw.swap_used_kb),
                        system::format_kb(hw.swap_total_kb)
                    ),
                ],
                bar: Some(percent(hw.mem_used_kb, hw.mem_total_kb)),
            },
            SystemCard {
                title: "Storage",
                lines: vec![
                    format!(
                        "{} of {} used",
                        system::format_kb(hw.disk_used_kb),
                        system::format_kb(hw.disk_total_kb)
                    ),
                    format!("Filesystem: {}", hw.disk_fs),
                    format!("Running for {}", uptime_prose()),
                ],
                bar: Some(percent(hw.disk_used_kb, hw.disk_total_kb)),
            },
        ];
        ScrollArea::vertical()
            .id_salt("system_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(
                    RichText::new("This computer")
                        .font(FontId::proportional(TITLE_SIZE))
                        .color(palette.text)
                        .strong(),
                );
                ui.add_space(22.0);
                let columns = if ui.available_width() < 640.0 { 1 } else { 2 };
                let width = tile_width(ui, columns);
                for row in cards.chunks(columns) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        for card in row {
                            system_card(ui, width, card, &palette);
                        }
                    });
                    ui.add_space(CARD_GAP);
                }
            });
    }

    fn queue_items(&self) -> Vec<(&'static str, String, QueueKey)> {
        let mut items = Vec::new();
        let mut install: Vec<usize> = self.install_selected.iter().copied().collect();
        install.sort_by_key(|index| self.apps.get(*index).map(|app| app.label.clone()));
        for index in install {
            if let Some(app) = self.apps.get(index) {
                items.push(("Install", app.label.clone(), QueueKey::Install(index)));
            }
        }
        let mut remove: Vec<usize> = self.remove_selected.iter().copied().collect();
        remove.sort_by_key(|index| self.remove_apps.get(*index).map(|app| app.label.clone()));
        for index in remove {
            if let Some(app) = self.remove_apps.get(index) {
                items.push(("Remove", app.label.clone(), QueueKey::Remove(index)));
            }
        }
        if let Some(theme) = self.cursor_selected {
            items.push(("Apply", theme.label().to_owned(), QueueKey::Cursor));
        }
        let mut admin: Vec<usize> = self.admin_selected.iter().copied().collect();
        admin.sort_unstable();
        for index in admin {
            if let Some(task) = self.admin_tasks.get(index) {
                items.push((
                    "Run",
                    recipe_title(&task.label).to_owned(),
                    QueueKey::Admin(index),
                ));
            }
        }
        items
    }

    fn unstage(&mut self, key: QueueKey) {
        match key {
            QueueKey::Install(index) => {
                self.install_selected.remove(&index);
            }
            QueueKey::Remove(index) => {
                self.remove_selected.remove(&index);
            }
            QueueKey::Cursor => self.cursor_selected = None,
            QueueKey::Admin(index) => {
                self.admin_selected.remove(&index);
            }
        }
    }

    /// Floating pill at the bottom centre that summarises the queue.
    fn run_dock(&mut self, ctx: &Context) {
        if !self.dock_visible() || self.show_password_modal {
            return;
        }

        let palette = self.palette();
        let count = self.dock_count();
        let names = truncated_names(&self.dock_names(), 56);
        let mut review = false;

        Area::new(Id::new("run_dock"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -24.0])
            .order(Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.void)
                    .stroke(Stroke::new(1.0_f32, palette.accent))
                    .corner_radius(DOCK_RADIUS)
                    .inner_margin(egui::Margin::symmetric(22, 14))
                    .shadow(egui::Shadow {
                        offset: [0, 12],
                        blur: 40,
                        spread: 0,
                        color: Color32::from_black_alpha(0x77),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(DOCK_MIN_WIDTH);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 16.0;
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 5.0;
                                    ui.label(
                                        RichText::new(count.to_string())
                                            .font(FontId::proportional(18.0))
                                            .color(palette.accent)
                                            .strong(),
                                    );
                                    ui.label(
                                        RichText::new(if count == 1 {
                                            "task queued"
                                        } else {
                                            "tasks queued"
                                        })
                                        .font(FontId::proportional(18.0))
                                        .color(palette.text)
                                        .strong(),
                                    );
                                });
                                ui.label(RichText::new(names).color(palette.muted));
                            });
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if cta_button(ui, "Review & run →", true, &palette).clicked() {
                                    review = true;
                                }
                            });
                        });
                    });
            });

        if review {
            self.show_password_modal = true;
        }
    }

    /// Right-hand progress drawer over a dimmed scrim.
    fn run_drawer_overlay(&mut self, ctx: &Context) {
        if !self.run_drawer_open {
            return;
        }
        let palette = self.palette();
        let screen = ctx.content_rect();
        let done_count = self
            .run_items
            .iter()
            .filter(|item| matches!(item.status, TaskStatus::Success | TaskStatus::Failed))
            .count();
        let total = self.run_items.len().max(1);
        let failed = self
            .run_items
            .iter()
            .any(|item| item.status == TaskStatus::Failed);
        let finished = !self.is_running;
        let mut close = false;

        Area::new(Id::new("run_scrim"))
            .fixed_pos(screen.min)
            .order(Order::Foreground)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(screen.size(), Sense::click_and_drag());
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(0x99));
            });

        Area::new(Id::new("run_drawer"))
            .fixed_pos(pos2(screen.right() - RUN_DRAWER_WIDTH, screen.top()))
            .order(Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.void)
                    .stroke(Stroke::new(1.0_f32, palette.border_strong))
                    .inner_margin(egui::Margin::same(28))
                    .show(ui, |ui| {
                        ui.set_width(RUN_DRAWER_WIDTH - 56.0);
                        ui.set_min_height(screen.height() - 56.0);
                        ui.label(
                            RichText::new(if !finished {
                                "Working on it…"
                            } else if failed {
                                "Finished with problems"
                            } else {
                                "All done"
                            })
                            .font(FontId::proportional(30.0))
                            .color(palette.text)
                            .strong(),
                        );
                        ui.label(
                            RichText::new(if !finished {
                                "Please keep this window open."
                            } else if failed {
                                "Some tasks failed. Check the details below."
                            } else {
                                "Everything finished without problems."
                            })
                            .color(palette.muted),
                        );
                        ui.add_space(18.0);

                        let (bar, _) =
                            ui.allocate_exact_size(vec2(ui.available_width(), 6.0), Sense::hover());
                        ui.painter().rect_filled(bar, 0.0, palette.border);
                        let fraction = if finished {
                            1.0
                        } else {
                            done_count as f32 / total as f32
                        };
                        ui.painter().rect_filled(
                            Rect::from_min_size(bar.min, vec2(bar.width() * fraction, 6.0)),
                            0.0,
                            palette.accent,
                        );
                        ui.add_space(18.0);

                        ScrollArea::vertical()
                            .id_salt("run_task_status")
                            .max_height(220.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                for (index, item) in self.run_items.iter().enumerate() {
                                    run_job_row(ui, index > 0, item, &palette);
                                }
                            });
                        ui.add_space(14.0);

                        section_eyebrow(ui, "Details", &palette);
                        ui.add_space(6.0);
                        let log_height = (ui.available_height() - 76.0).max(80.0);
                        let showing_placeholder = self.log == "Process logs will appear here...";
                        Frame::new()
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .inner_margin(8.0)
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
                                                .font(FontId::monospace(12.0))
                                                .color(if showing_placeholder {
                                                    palette.muted
                                                } else {
                                                    palette.log_text
                                                }),
                                        );
                                    });
                            });
                        ui.add_space(14.0);
                        let label = if finished { "Close" } else { "Running…" };
                        let button = ui.add_enabled(
                            finished,
                            Button::new(RichText::new(label).color(palette.cta_text).strong())
                                .fill(palette.cta_fill)
                                .stroke(Stroke::new(1.0_f32, palette.cta_fill))
                                .corner_radius(0.0)
                                .min_size(vec2(ui.available_width(), 44.0)),
                        );
                        if button.clicked() {
                            close = true;
                        }
                    });
            });

        if close {
            self.run_drawer_open = false;
            self.acknowledge_run_complete();
        }
    }

    fn staged_needs_sudo(&self) -> bool {
        match self.selected_tasks() {
            Ok(tasks) if !tasks.is_empty() => tasks_need_privileges(&tasks),
            _ => true,
        }
    }

    /// Expanded queue panel: review, optional password, then run.
    fn sudo_modal(&mut self, ctx: &Context) {
        if !self.show_password_modal {
            return;
        }
        let palette = self.palette();

        if self.total_staged_count() == 0 {
            self.show_password_modal = false;
            return;
        }
        if ctx.input(|input| input.key_pressed(Key::Escape)) {
            zeroize_string(&mut self.password);
            self.show_password_modal = false;
            return;
        }

        let items = self.queue_items();
        let toolbox = self.remove_includes_toolbox();
        let needs_sudo = self.staged_needs_sudo();
        let count = items.len();
        let mut close = false;
        let mut clear = false;
        let mut run = false;
        let mut removed = None;

        Area::new(Id::new("queue_panel"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -24.0])
            .order(Order::Foreground)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.void)
                    .stroke(Stroke::new(1.0_f32, palette.accent))
                    .inner_margin(egui::Margin::symmetric(22, 18))
                    .shadow(egui::Shadow {
                        offset: [0, 12],
                        blur: 40,
                        spread: 0,
                        color: Color32::from_black_alpha(0x77),
                    })
                    .show(ui, |ui| {
                        ui.set_width(DOCK_MIN_WIDTH.max(520.0));
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Ready to run: {count} {}",
                                    if count == 1 { "task" } else { "tasks" }
                                ))
                                .font(FontId::proportional(20.0))
                                .color(palette.text)
                                .strong(),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ghost_button(ui, "✕", &palette).clicked() {
                                    close = true;
                                }
                            });
                        });
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
                        ui.add_space(10.0);
                        Frame::new()
                            .stroke(Stroke::new(1.0_f32, palette.border))
                            .show(ui, |ui| {
                                ScrollArea::vertical()
                                    .id_salt("sudo_name_list")
                                    .max_height(200.0)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.spacing_mut().item_spacing.y = 0.0;
                                        for (verb, name, key) in &items {
                                            if queue_item_row(ui, verb, name, &palette) {
                                                removed = Some(*key);
                                            }
                                        }
                                    });
                            });
                        ui.add_space(12.0);
                        if needs_sudo {
                            ui.label(
                                RichText::new("Your password (needed to change the system)")
                                    .color(palette.muted),
                            );
                            ui.add_space(4.0);
                            let response = ui.add(
                                TextEdit::singleline(&mut self.password)
                                    .password(true)
                                    .desired_width(ui.available_width())
                                    .text_color(palette.text)
                                    .background_color(palette.input_fill),
                            );
                            if response.lost_focus()
                                && ui.input(|input| input.key_pressed(Key::Enter))
                                && !self.password.is_empty()
                            {
                                run = true;
                            }
                        } else {
                            ui.label(
                                RichText::new("These tasks run without a password.")
                                    .color(palette.muted),
                            );
                            if ui.input(|input| input.key_pressed(Key::Enter)) {
                                run = true;
                            }
                        }
                        ui.add_space(12.0);
                        ui.horizontal(|ui| {
                            if ghost_button(ui, "Clear all", &palette).clicked() {
                                clear = true;
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let ready = !needs_sudo || !self.password.is_empty();
                                if ui
                                    .add_enabled(
                                        ready,
                                        Button::new(
                                            RichText::new(format!("Run {count} tasks →"))
                                                .color(palette.cta_text)
                                                .strong(),
                                        )
                                        .fill(palette.cta_fill)
                                        .stroke(Stroke::new(1.0_f32, palette.cta_fill))
                                        .corner_radius(0.0)
                                        .min_size(vec2(160.0, 40.0)),
                                    )
                                    .clicked()
                                {
                                    run = true;
                                }
                            });
                        });
                    });
            });

        if let Some(key) = removed {
            self.unstage(key);
        }
        if clear {
            zeroize_string(&mut self.password);
            self.clear_dock_selections();
            self.show_password_modal = false;
        } else if close {
            zeroize_string(&mut self.password);
            self.show_password_modal = false;
        } else if run {
            self.show_password_modal = false;
            self.run_selected(ctx);
        }
    }

    fn acknowledge_run_complete(&mut self) {
        self.show_run_complete_modal = false;
    }

    fn toast_overlay(&mut self, ctx: &Context) {
        let Some((message, started)) = &self.toast else {
            return;
        };
        let elapsed = started.elapsed();
        if elapsed >= std::time::Duration::from_millis(2200) {
            self.toast = None;
            return;
        }
        let palette = self.palette();
        let message = message.clone();
        Area::new(Id::new("toast"))
            .anchor(Align2::RIGHT_TOP, [-24.0, TOPBAR_HEIGHT + 16.0])
            .order(Order::Tooltip)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(palette.accent)
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new(message).color(palette.accent_on).strong());
                    });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueueKey {
    Install(usize),
    Remove(usize),
    Cursor,
    Admin(usize),
}

impl eframe::App for ToolboxApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.drain_runner_messages();
        self.drain_remove_scan();

        let palette = self.palette();
        Panel::top("topbar")
            .resizable(false)
            .exact_size(TOPBAR_HEIGHT)
            .frame(
                Frame::new()
                    .fill(palette.void)
                    .stroke(Stroke::new(1.0_f32, palette.border))
                    .inner_margin(egui::Margin::symmetric(28, 0)),
            )
            .show_inside(ui, |ui| self.topbar(ui));

        CentralPanel::default()
            .frame(Frame::new().fill(palette.void).inner_margin(egui::Margin {
                left: 44,
                right: 44,
                top: 36,
                bottom: 120,
            }))
            .show_inside(ui, |ui| match self.page {
                Page::Home => self.home_page(ui),
                Page::Apps => self.apps_page(ui),
                Page::Recipes => self.recipes_page(ui),
                Page::Cursor => self.cursor_page(ui),
                Page::System => self.system_info_page(ui),
            });

        let ctx = ui.ctx().clone();
        self.run_dock(&ctx);
        self.sudo_modal(&ctx);
        self.run_drawer_overlay(&ctx);
        self.toast_overlay(&ctx);
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

/// Decodes the app's own logo (not an app tile): `assets/brand/toolbox-logo.png`.
fn decode_brand_logo() -> Option<crate::logos::Raster> {
    let pixmap =
        tiny_skia::Pixmap::decode_png(include_bytes!("../assets/brand/toolbox-logo.png")).ok()?;
    Some(crate::logos::Raster {
        size: [pixmap.width() as usize, pixmap.height() as usize],
        rgba: pixmap.data().to_vec(),
    })
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
    if let Some(raster) = decode_brand_logo() {
        icons.insert(
            "toolbox-logo",
            ctx.load_texture(
                "toolbox-logo",
                crate::logos::color_image_from_raster(&raster),
                TextureOptions::LINEAR,
            ),
        );
    }
    for theme in CursorTheme::ALL {
        let svg = theme.preview_markup();
        if let Some(raster) = crate::logos::rasterize_svg_markup(&svg, 256) {
            icons.insert(
                theme.preview_key(),
                ctx.load_texture(
                    format!("cursor-{}", theme.preview_key()),
                    crate::logos::color_image_from_raster(&raster),
                    TextureOptions::LINEAR,
                ),
            );
        }
    }
    icons
}

fn tab_button(
    ui: &mut Ui,
    page: Page,
    selected: bool,
    badge: usize,
    compact: bool,
    palette: &Palette,
) -> egui::Response {
    let show_label = !compact || selected;
    let font = FontId::proportional(15.0);
    let color = if selected {
        palette.text
    } else {
        palette.muted
    };
    let galley = ui
        .painter()
        .layout_no_wrap(page.title().to_owned(), font, color);
    let badge_width = if badge > 0 { 26.0 } else { 0.0 };
    let width = if show_label {
        16.0 + 18.0 + 8.0 + galley.size().x + badge_width + 16.0
    } else {
        12.0 + 18.0 + badge_width + 12.0
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, TOPBAR_HEIGHT), Sense::click());
    if response.hovered() && !selected {
        ui.painter().rect_filled(rect, 0.0, palette.surface_hover);
    }
    let icon_rect = Rect::from_center_size(
        pos2(
            rect.left() + if show_label { 25.0 } else { 21.0 },
            rect.center().y,
        ),
        vec2(18.0, 18.0),
    );
    paint_page_symbol(
        ui.painter(),
        icon_rect,
        page,
        if selected {
            palette.accent
        } else {
            palette.muted
        },
    );
    let text_pos = pos2(
        icon_rect.right() + if show_label { 8.0 } else { 0.0 },
        rect.center().y - galley.size().y / 2.0,
    );
    let text_right = text_pos.x + galley.size().x;
    if show_label {
        ui.painter().galley(text_pos, galley, color);
    }
    if badge > 0 {
        let badge_rect = Rect::from_center_size(
            pos2(text_right + 8.0 + 9.0, rect.center().y),
            vec2(18.0, 18.0),
        );
        ui.painter().rect_filled(badge_rect, 0.0, palette.accent);
        ui.painter().text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            badge.to_string(),
            FontId::proportional(11.0),
            palette.accent_on,
        );
    }
    if selected {
        ui.painter().rect_filled(
            Rect::from_min_max(pos2(rect.left(), rect.bottom() - 2.0), rect.right_bottom()),
            0.0,
            palette.accent,
        );
    }
    response
}

fn queue_button(ui: &mut Ui, count: usize, palette: &Palette) -> egui::Response {
    ui.add(
        Button::new(
            RichText::new(format!("Queue: {count} →"))
                .color(palette.accent)
                .strong(),
        )
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, palette.accent))
        .corner_radius(0.0)
        .min_size(vec2(0.0, 34.0)),
    )
}

fn run_job_row(ui: &mut Ui, divider: bool, item: &RunItem, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::hover());
    if divider {
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0_f32, palette.border),
        );
    }
    let badge = Rect::from_center_size(pos2(rect.left() + 10.0, rect.center().y), vec2(16.0, 16.0));
    match item.status {
        TaskStatus::Success => {
            ui.painter().rect_filled(badge, 0.0, palette.accent);
        }
        TaskStatus::Failed => {
            ui.painter().rect_filled(badge, 0.0, palette.danger);
        }
        TaskStatus::Running => {
            ui.painter().rect_stroke(
                badge,
                0.0,
                Stroke::new(1.5_f32, palette.accent),
                StrokeKind::Inside,
            );
        }
        TaskStatus::Queued => {
            ui.painter().rect_stroke(
                badge,
                0.0,
                Stroke::new(1.0_f32, palette.border_strong),
                StrokeKind::Inside,
            );
        }
    }
    ui.painter().text(
        pos2(rect.left() + 28.0, rect.center().y),
        Align2::LEFT_CENTER,
        &item.description,
        FontId::proportional(14.5),
        palette.text,
    );
    let label = match item.status {
        TaskStatus::Queued => "Waiting",
        TaskStatus::Running => "Working…",
        TaskStatus::Success => "Done",
        TaskStatus::Failed => "Failed",
    };
    ui.painter().text(
        pos2(rect.right(), rect.center().y),
        Align2::RIGHT_CENTER,
        label,
        FontId::proportional(13.0),
        task_status_color(item.status, palette),
    );
}

/// One queue line. Returns true when the remove button was clicked.
fn queue_item_row(ui: &mut Ui, verb: &str, name: &str, palette: &Palette) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 38.0), Sense::hover());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, palette.surface_hover);
    }
    ui.painter().text(
        pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        verb,
        FontId::proportional(13.0),
        palette.accent,
    );
    ui.painter().text(
        pos2(rect.left() + 84.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(14.5),
        palette.text,
    );
    let x_rect =
        Rect::from_center_size(pos2(rect.right() - 20.0, rect.center().y), vec2(28.0, 28.0));
    let x = ui.interact(
        x_rect,
        ui.id().with(("unstage", verb, name)),
        Sense::click(),
    );
    ui.painter().text(
        x_rect.center(),
        Align2::CENTER_CENTER,
        "✕",
        FontId::proportional(13.0),
        if x.hovered() {
            palette.text
        } else {
            palette.muted
        },
    );
    x.clicked()
}

fn family_swatch(
    ui: &mut Ui,
    family: ThemeFamily,
    selected: bool,
    chrome: &Palette,
) -> egui::Response {
    let skin = Palette::from(family, ThemeMode::Dark);
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    ui.painter().rect_filled(rect, 0.0, skin.accent);
    paint_swatch_ring(ui, rect, selected, response.hovered(), chrome);
    response.on_hover_text(format!("{} appearance", family.label()))
}

/// The white Light skin, last in the swatch row.
fn light_swatch(ui: &mut Ui, selected: bool, chrome: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
    ui.painter().rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, chrome.border_strong),
        StrokeKind::Inside,
    );
    paint_swatch_ring(ui, rect, selected, response.hovered(), chrome);
    response.on_hover_text("Light appearance")
}

fn paint_swatch_ring(ui: &Ui, rect: Rect, selected: bool, hovered: bool, chrome: &Palette) {
    if selected || hovered {
        ui.painter().rect_stroke(
            rect.expand(3.0),
            0.0,
            Stroke::new(
                if selected { 1.5_f32 } else { 1.0_f32 },
                if selected { chrome.text } else { chrome.muted },
            ),
            StrokeKind::Outside,
        );
    }
}

fn mode_segment(ui: &mut Ui, mode: AppsMode, palette: &Palette) -> Option<AppsMode> {
    let mut clicked = None;
    // Right-to-left parent: add Uninstall first so Install reads on the left.
    ui.spacing_mut().item_spacing.x = 0.0;
    if segment_button(ui, "Uninstall", mode == AppsMode::Remove, palette).clicked() {
        clicked = Some(AppsMode::Remove);
    }
    if segment_button(ui, "Install", mode == AppsMode::Install, palette).clicked() {
        clicked = Some(AppsMode::Install);
    }
    clicked
}

fn segment_button(ui: &mut Ui, label: &str, selected: bool, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(104.0, 38.0), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, 0.0, palette.accent);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, palette.surface_hover);
    }
    ui.painter().rect_stroke(
        rect,
        0.0,
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
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        if selected {
            palette.accent_on
        } else {
            palette.muted
        },
    );
    response
}

fn filter_chip(ui: &mut Ui, label: &str, selected: bool, palette: &Palette) -> egui::Response {
    let width = (label.len() as f32 * 7.6 + 30.0).clamp(48.0, 150.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 34.0), Sense::click());
    let fill = if selected {
        palette.accent
    } else if response.hovered() {
        palette.surface_hover
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    if !selected {
        ui.painter().rect_stroke(
            rect,
            0.0,
            Stroke::new(1.0_f32, palette.border),
            StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.5),
        if selected {
            palette.accent_on
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
        .corner_radius(0.0)
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

/// Selected tiles wash and outline with the family accent (Orange Ember is
/// `#F97316`). Caution peach is reserved for chips, never selection chrome.
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
    if selected {
        painter.rect_filled(rect, 0.0, palette.accent);
        let center = rect.center();
        let a = pos2(rect.left() + 6.0, center.y);
        let b = pos2(rect.left() + 9.5, rect.bottom() - 6.5);
        let c = pos2(rect.right() - 6.0, rect.top() + 6.5);
        painter.line_segment([a, b], Stroke::new(2.0_f32, palette.accent_on));
        painter.line_segment([b, c], Stroke::new(2.0_f32, palette.accent_on));
    } else {
        painter.rect_stroke(
            rect,
            0.0,
            Stroke::new(1.2_f32, palette.border_strong),
            StrokeKind::Inside,
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
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, palette.border),
        StrokeKind::Inside,
    );
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

struct AppCard<'a> {
    width: f32,
    selected: bool,
    label: &'a str,
    source: Option<&'a str>,
    description: &'a str,
    icon: Option<&'a (TextureHandle, bool)>,
    palette: &'a Palette,
}

fn app_card(ui: &mut Ui, card: AppCard<'_>) -> egui::Response {
    let AppCard {
        width,
        selected,
        label,
        source,
        description,
        icon,
        palette,
    } = card;
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
        pos2(text_left, rect.min.y + 74.0),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(17.0),
        palette.text,
    );

    let galley = painter.layout(
        description.to_owned(),
        FontId::proportional(12.5),
        palette.muted,
        text_width,
    );
    let desc_pos = pos2(text_left, rect.min.y + 92.0);
    let clip = Rect::from_min_size(desc_pos, vec2(text_width, 32.0));
    painter
        .with_clip_rect(clip)
        .galley(desc_pos, galley, palette.muted);

    if let Some(source) = source {
        paint_source_pill(
            painter,
            pos2(text_left, rect.bottom() - 18.0),
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
    tile_width_gap(ui, columns, CARD_GAP)
}

fn tile_width_gap(ui: &Ui, columns: usize, gap: f32) -> f32 {
    let columns = columns.max(1) as f32;
    ((ui.available_width() - gap * (columns - 1.0)) / columns).max(220.0)
}

fn section_eyebrow(ui: &mut Ui, label: &str, palette: &Palette) {
    ui.label(
        RichText::new(label.to_ascii_uppercase())
            .font(FontId::proportional(11.0))
            .color(palette.muted),
    );
}

fn intent_card(
    ui: &mut Ui,
    width: f32,
    glyph: Glyph,
    title: &str,
    body: &str,
    cta: &str,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, INTENT_HEIGHT), Sense::click());
    paint_surface_card(ui.painter(), rect, response.hovered(), palette);
    for corner in [
        rect.left_top() + vec2(-8.0, -5.0),
        rect.right_top() + vec2(8.0, -5.0),
        rect.left_bottom() + vec2(-8.0, 5.0),
        rect.right_bottom() + vec2(8.0, 5.0),
    ] {
        ui.painter().text(
            corner,
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(14.0),
            palette.muted,
        );
    }

    let well = Rect::from_min_size(rect.min + vec2(24.0, 24.0), vec2(44.0, 44.0));
    ui.painter().rect_stroke(
        well,
        0.0,
        Stroke::new(1.0_f32, palette.border_strong),
        StrokeKind::Inside,
    );
    paint_glyph(ui.painter(), well.shrink(12.0), glyph, palette.accent);

    let painter = ui.painter();
    let text_left = rect.min.x + 24.0;
    let text_width = (rect.width() - 48.0).max(24.0);
    painter.text(
        pos2(text_left, rect.min.y + 82.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(22.0),
        palette.text,
    );
    let galley = painter.layout(
        body.to_owned(),
        FontId::proportional(13.5),
        palette.muted,
        text_width,
    );
    painter
        .with_clip_rect(Rect::from_min_size(
            pos2(text_left, rect.min.y + 114.0),
            vec2(text_width, 38.0),
        ))
        .galley(pos2(text_left, rect.min.y + 114.0), galley, palette.muted);
    painter.text(
        pos2(text_left, rect.bottom() - 18.0),
        Align2::LEFT_CENTER,
        format!("{cta} →"),
        FontId::proportional(14.0),
        palette.accent,
    );
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
        pos2(rect.min.x + 16.0, rect.min.y + 14.0),
        Align2::LEFT_TOP,
        label,
        FontId::proportional(11.0),
        palette.muted,
    );
    let value_pos = pos2(rect.min.x + 16.0, rect.min.y + 34.0);
    let galley = painter.layout(
        value.to_owned(),
        FontId::proportional(22.0),
        palette.text,
        (rect.width() - 32.0).max(24.0),
    );
    painter
        .with_clip_rect(Rect::from_min_size(
            value_pos,
            vec2((rect.width() - 32.0).max(24.0), 34.0),
        ))
        .galley(value_pos, galley, palette.text);
    response
}

struct SystemCard {
    title: &'static str,
    lines: Vec<String>,
    bar: Option<f32>,
}

fn system_card(ui: &mut Ui, width: f32, card: &SystemCard, palette: &Palette) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 168.0), Sense::hover());
    paint_surface_card(ui.painter(), rect, false, palette);
    let painter = ui.painter();
    painter.text(
        rect.min + vec2(22.0, 18.0),
        Align2::LEFT_TOP,
        card.title,
        FontId::proportional(22.0),
        palette.text,
    );
    for (index, line) in card.lines.iter().enumerate() {
        let pos = rect.min + vec2(22.0, 54.0 + index as f32 * 22.0);
        let galley = painter.layout(
            line.clone(),
            FontId::proportional(14.0),
            if index == 0 {
                palette.text
            } else {
                palette.muted
            },
            (rect.width() - 44.0).max(24.0),
        );
        painter
            .with_clip_rect(Rect::from_min_size(pos, vec2(rect.width() - 44.0, 22.0)))
            .galley(pos, galley, palette.text);
    }
    if let Some(fraction) = card.bar {
        let bar = Rect::from_min_size(
            pos2(rect.left() + 22.0, rect.bottom() - 22.0),
            vec2(rect.width() - 44.0, 6.0),
        );
        painter.rect_filled(bar, 0.0, palette.border);
        painter.rect_filled(
            Rect::from_min_size(bar.min, vec2(bar.width() * fraction, 6.0)),
            0.0,
            palette.accent,
        );
    }
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

fn recipe_row(
    ui: &mut Ui,
    selected: bool,
    divider: bool,
    locked: bool,
    label: &str,
    palette: &Palette,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), RECIPE_ROW_HEIGHT),
        Sense::hover(),
    );
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, palette.surface_hover);
    }
    if divider {
        ui.painter().line_segment(
            [rect.left_top(), rect.right_top()],
            Stroke::new(1.0_f32, palette.border),
        );
    }
    let well = Rect::from_min_size(
        pos2(rect.left() + 22.0, rect.center().y - 20.0),
        vec2(40.0, 40.0),
    );
    ui.painter().rect_stroke(
        well,
        0.0,
        Stroke::new(1.0_f32, palette.border_strong),
        StrokeKind::Inside,
    );
    paint_glyph(ui.painter(), well.shrink(11.0), Glyph::Bolt, palette.accent);

    let text_left = well.right() + 18.0;
    let button = vec2(124.0, 38.0);
    let text_width = (rect.right() - 22.0 - button.x - 16.0 - text_left).max(40.0);
    let painter = ui.painter();
    painter.text(
        pos2(text_left, rect.center().y),
        Align2::LEFT_BOTTOM,
        recipe_title(label),
        FontId::proportional(19.0),
        palette.text,
    );
    let galley = painter.layout(
        recipe_promise(label).to_owned(),
        FontId::proportional(13.5),
        palette.muted,
        text_width,
    );
    let desc_pos = pos2(text_left, rect.center().y + 4.0);
    painter
        .with_clip_rect(Rect::from_min_size(desc_pos, vec2(text_width, 24.0)))
        .galley(desc_pos, galley, palette.muted);

    let button_rect = Rect::from_min_size(
        pos2(
            rect.right() - 22.0 - button.x,
            rect.center().y - button.y / 2.0,
        ),
        button,
    );
    let (caption, fill, text, stroke) = if selected {
        ("Added ✓", palette.accent, palette.accent_on, palette.accent)
    } else {
        (
            "Add to queue",
            Color32::TRANSPARENT,
            palette.text,
            palette.border_strong,
        )
    };
    ui.push_id((label, "recipe-row"), |ui| {
        ui.add_enabled_ui(!locked, |ui| {
            ui.put(
                button_rect,
                Button::new(RichText::new(caption).color(text).strong())
                    .fill(fill)
                    .stroke(Stroke::new(1.0_f32, stroke))
                    .corner_radius(0.0)
                    .min_size(button),
            )
            .clicked()
        })
        .inner
    })
    .inner
}

fn recipe_title(label: &str) -> &'static str {
    match label {
        "Update System" => "Update everything",
        "Enable Bluetooth" => "Turn Bluetooth on",
        "Disable Bluetooth" => "Turn Bluetooth off",
        "TLP (Laptops)" => "Make your battery last longer",
        "Powertop" => "Cooler, quieter laptop",
        "SWAP Fix" => "Fix slowdowns when memory is full",
        "nala (rank mirrors) - Debian only" => "Faster, friendlier installs",
        "Fastfetch" => "Show system info in the terminal",
        "Stacer" => "System cleaner and monitor",
        _ => "Admin playbook",
    }
}

fn recipe_list_rank(label: &str) -> u8 {
    match label {
        "Update System" => 0,
        "Enable Bluetooth" => 1,
        "Disable Bluetooth" => 2,
        "TLP (Laptops)" => 3,
        "Powertop" => 4,
        "SWAP Fix" => 5,
        "nala (rank mirrors) - Debian only" => 6,
        "Fastfetch" => 7,
        "Stacer" => 8,
        _ => 100,
    }
}

fn cursor_short_name(theme: CursorTheme) -> &'static str {
    match theme {
        CursorTheme::Classic => "Bibata Classic",
        CursorTheme::Ice => "Bibata Ice",
        CursorTheme::Amber => "Bibata Amber",
        CursorTheme::Plasma => "KDE Plasma",
    }
}

fn cursor_short_name_for(label: &str) -> String {
    CursorTheme::ALL
        .into_iter()
        .find(|theme| theme.label() == label)
        .map_or_else(
            || label.to_owned(),
            |theme| cursor_short_name(theme).to_owned(),
        )
}

fn cursor_columns(ui: &Ui) -> usize {
    ((ui.available_width() / 250.0) as usize).clamp(1, 4)
}

fn cursor_card(
    ui: &mut Ui,
    width: f32,
    theme: CursorTheme,
    selected: bool,
    locked: bool,
    preview: Option<&TextureHandle>,
    palette: &Palette,
) -> (bool, bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, CURSOR_CARD_HEIGHT), Sense::hover());
    paint_card_background(ui.painter(), rect, selected, palette);

    let stage = Rect::from_min_size(rect.min + vec2(16.0, 16.0), vec2(width - 32.0, 120.0));
    ui.painter().rect_stroke(
        stage,
        0.0,
        Stroke::new(1.0_f32, palette.border),
        StrokeKind::Inside,
    );
    if let Some(icon) = preview {
        ui.painter().image(
            icon.id(),
            Rect::from_center_size(stage.center(), vec2(64.0, 64.0)),
            Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
            Color32::WHITE,
        );
    }
    ui.painter().text(
        pos2(rect.min.x + 16.0, stage.bottom() + 14.0),
        Align2::LEFT_TOP,
        cursor_short_name(theme),
        FontId::proportional(21.0),
        palette.text,
    );

    let button = vec2(width - 32.0, 44.0);
    let apply_rect = Rect::from_min_size(pos2(rect.left() + 16.0, stage.bottom() + 52.0), button);
    let stage_rect =
        Rect::from_min_size(pos2(apply_rect.left(), apply_rect.bottom() + 8.0), button);
    let apply_clicked = ui
        .push_id((theme.directory(), "apply"), |ui| {
            ui.add_enabled_ui(!locked, |ui| {
                ui.put(apply_rect, cursor_apply_button(theme, button, palette))
                    .clicked()
            })
            .inner
        })
        .inner;
    let stage_clicked = ui
        .push_id((theme.directory(), "stage"), |ui| {
            ui.add_enabled_ui(!locked, |ui| {
                ui.put(stage_rect, cursor_stage_button(selected, button, palette))
                    .clicked()
            })
            .inner
        })
        .inner;
    (stage_clicked, apply_clicked)
}

fn cursor_stage_button(selected: bool, size: egui::Vec2, palette: &Palette) -> Button<'_> {
    let (label, fill, text, stroke) = if selected {
        (
            "In queue ✓",
            palette.accent,
            palette.accent_on,
            palette.accent,
        )
    } else {
        (
            "Add to queue",
            Color32::TRANSPARENT,
            palette.text,
            palette.border_strong,
        )
    };
    Button::new(RichText::new(label).color(text).strong())
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, stroke))
        .corner_radius(0.0)
        .min_size(size)
}

fn cursor_apply_button(theme: CursorTheme, size: egui::Vec2, palette: &Palette) -> Button<'_> {
    Button::new(
        RichText::new(format!("Use {} now", cursor_short_name(theme)))
            .color(palette.cta_text)
            .strong(),
    )
    .fill(palette.cta_fill)
    .stroke(Stroke::new(1.0_f32, palette.cta_fill))
    .corner_radius(0.0)
    .min_size(size)
}

fn recipe_display_label(label: &str) -> &str {
    label.strip_suffix(" - Debian only").unwrap_or(label)
}

fn recipe_promise(label: &str) -> &'static str {
    match label {
        "Enable Bluetooth" => "Enable Bluetooth and start it at boot.",
        "Disable Bluetooth" => "Disable Bluetooth to save power.",
        "TLP (Laptops)" => "Installs TLP to tune power use automatically.",
        "Powertop" => "Installs Powertop to cut idle power and heat.",
        "Update System" => "Install the latest updates for your system and apps.",
        "nala (rank mirrors) - Debian only" => "Installs Nala, a nicer front end for apt.",
        "Stacer" => "Installs Stacer to clear clutter and watch usage.",
        "SWAP Fix" => "Adds a swap fix for smoother multitasking.",
        "Fastfetch" => "Installs Fastfetch.",
        _ => "Admin playbook for this machine.",
    }
}

fn paint_kind_chip(ui: &mut Ui, kind: &str, palette: &Palette) {
    let width = (kind.len() as f32 * 7.2 + 16.0).clamp(56.0, 100.0);
    let (rect, _) = ui.allocate_exact_size(vec2(width, 22.0), Sense::hover());
    let (fill, text) = match kind {
        "Recipe" => (palette.caution, palette.caution_on),
        _ => (palette.elevated, palette.accent),
    };
    ui.painter().rect_filled(rect, 0.0, fill);
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
        Glyph::Tab(page) => paint_page_symbol(painter, rect, page, color),
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
        Glyph::Battery => {
            let body = Rect::from_min_max(
                pos2(rect.left(), rect.top() + 3.0),
                pos2(rect.right() - 3.0, rect.bottom() - 3.0),
            );
            painter.rect_stroke(body, 0.0, stroke, StrokeKind::Inside);
            painter.rect_filled(
                Rect::from_center_size(pos2(rect.right() - 1.0, rect.center().y), vec2(2.5, 6.0)),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_max(
                    body.min + vec2(3.0, 3.0),
                    pos2(body.center().x + 2.0, body.max.y - 3.0),
                ),
                0.0,
                color,
            );
        }
    }
}

fn cta_button(ui: &mut Ui, label: &str, enabled: bool, palette: &Palette) -> egui::Response {
    ui.add_enabled(
        enabled,
        Button::new(RichText::new(label).color(palette.cta_text).strong())
            .fill(palette.cta_fill)
            .stroke(Stroke::new(1.0_f32, palette.cta_fill))
            .corner_radius(0.0)
            .min_size(vec2(148.0, 36.0)),
    )
}

fn ghost_button(ui: &mut Ui, label: &str, palette: &Palette) -> egui::Response {
    let width = (label.len() as f32 * 7.4 + 20.0).clamp(64.0, 120.0);
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, palette.surface_hover);
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
    entry.try_command(base_dir, Action::Remove)
}

fn paint_source_pill(painter: &Painter, left_center: Pos2, label: &str, palette: &Palette) {
    let text = source_pill_label(label);
    let width = (text.len() as f32 * 7.2 + 16.0).clamp(56.0, 140.0);
    let rect = Rect::from_center_size(
        pos2(left_center.x + width / 2.0, left_center.y),
        vec2(width, 20.0),
    );
    painter.rect_filled(rect, 0.0, palette.elevated);
    painter.rect_stroke(
        rect,
        0.0,
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
        Page::Cursor => {
            // Northwest arrow in the same 22px box and 1.7 stroke as the other rail marks.
            let tip = pos2(rect.left() + 2.0, rect.top() + 2.0);
            let heel = pos2(rect.left() + 9.5, rect.bottom() - 2.0);
            let notch = pos2(rect.left() + 12.1, rect.top() + 12.1);
            let wing = pos2(rect.right() - 2.0, rect.top() + 9.5);
            painter.add(egui::Shape::closed_line(
                vec![tip, heel, notch, wing],
                stroke,
            ));
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
        "Bottles" | "bottles" | "com.usebottles.bottles" => "bottles",
        "Boxes" | "boxes" | "gnome-boxes" | "org.gnome.Boxes" => "boxes",
        "Brave Browser" | "brave" | "com.brave.Browser" => "brave",
        "Brave Origin" | "brave-origin" | "brave-origin-bin" => "brave-origin",
        "Discord" | "discord" | "com.discordapp.Discord" => "discord",
        "Dolphin Emulator" | "dolphin-emu" | "org.DolphinEmu.dolphin-emu" => "dolphin-emu",
        "Extension Manager" | "extension-manager" | "com.mattjakeman.ExtensionManager" => {
            "extension-manager"
        }
        "Firefox" | "firefox" => "firefox",
        "Flatseal" | "flatseal" | "com.github.tchx84.Flatseal" => "flatseal",
        "Gear Lever" | "gearlever" | "gear-lever" | "it.mijorus.gearlever" => "gear-lever",
        "GIMP" | "gimp" | "org.gimp.GIMP" => "gimp",
        "Google Chrome" | "google-chrome" | "com.google.Chrome" => "google-chrome",
        "GParted" | "gparted" => "gparted",
        "Heroic" | "heroic" | "com.heroicgameslauncher.hgl" => "heroic",
        "htop" => "htop",
        "LibreOffice" | "libreoffice" => "libreoffice",
        "LibreWolf" | "librewolf" | "io.gitlab.librewolf-community" => "librewolf",
        "LocalSend" | "localsend" | "org.localsend.localsend_app" => "localsend",
        "Lutris" | "lutris" | "net.lutris.Lutris" => "lutris",
        "Microsoft Edge" | "microsoft-edge" | "com.microsoft.Edge" => "microsoft-edge",
        "Mission Center"
        | "missioncenter"
        | "mission-center"
        | "io.missioncenter.MissionCenter" => "mission-center",
        "mpv" => "mpv",
        "Obsidian" | "obsidian" | "md.obsidian.Obsidian" => "obsidian",
        "OBS Studio" | "obs" | "obs-studio" | "com.obsproject.Studio" => "obs-studio",
        "OnlyOffice"
        | "onlyoffice"
        | "onlyoffice-desktopeditors"
        | "org.onlyoffice.desktopeditors" => "onlyoffice",
        "Opera" | "opera" | "com.opera.Opera" => "opera",
        "PPSSPP" | "ppsspp" | "org.ppsspp.PPSSPP" => "ppsspp",
        "Prism Launcher"
        | "prismlauncher"
        | "prism-launcher"
        | "org.prismlauncher.PrismLauncher" => "prism-launcher",
        "Proton VPN" | "proton-vpn" | "protonvpn-app" | "com.protonvpn.www" => "proton-vpn",
        "ProtonPlus" | "protonplus" | "com.vysp3r.ProtonPlus" => "protonplus",
        "ProtonUp-Qt" | "protonup-qt" | "net.davidotek.pupgui2" => "protonup-qt",
        "PyCharm Community" | "pycharm-community" | "com.jetbrains.PyCharm-Community" => {
            "pycharm-community"
        }
        "qBittorrent" | "qbittorrent" => "qbittorrent",
        "RetroArch" | "retroarch" | "org.libretro.RetroArch" => "retroarch",
        "Signal" | "signal" | "signal-desktop" | "org.signal.Signal" => "signal",
        "Slack" | "slack" | "com.slack.Slack" => "slack",
        "Sober" | "sober" | "org.vinegarhq.Sober" => "sober",
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
            cursor_selected: None,
            direct_cursor: None,
            install_search: String::new(),
            install_category: None,
            remove_search: String::new(),
            distro_name: "Debian".to_owned(),
            package_manager: PackageManager::Apt,
            kernel: "6.12.0-test".to_owned(),
            de_wm: "sway".to_owned(),
            recent_runs: Vec::new(),
            log: String::new(),
            password: String::new(),
            show_password_modal: false,
            show_run_complete_modal: false,
            is_running: false,
            tx: None,
            rx: None,
            log_revision: 0,
            run_drawer_open: false,
            run_items: Vec::new(),
            icons: HashMap::new(),
            family: ThemeFamily::Teal,
            theme: ThemeMode::Dark,
            remove_apps: vec![
                sample_removable("htop", "htop", false),
                sample_removable("Toolbox", "linux-it-guy-toolbox", true),
            ],
            remove_scan: RemoveScanState::Ready,
            remove_scan_rx: None,
            toast: None,
            hardware: system::HardwareSnapshot::default(),
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
    fn run_queue_includes_all_staged_selections() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.cursor_selected = Some(CursorTheme::Classic);
        app.admin_selected.insert(0);

        assert_eq!(app.total_staged_count(), 4);
        let verbs: Vec<&str> = app.queue_items().iter().map(|item| item.0).collect();
        assert_eq!(verbs, ["Install", "Remove", "Apply", "Run"]);
        assert_eq!(
            app.dock_names(),
            vec![
                "Firefox".to_owned(),
                "htop".to_owned(),
                "Bibata Modern Classic".to_owned(),
                "Update System".to_owned(),
            ]
        );
        let tasks = app.selected_tasks().expect("combined tasks");
        assert_eq!(tasks.len(), 4);
        assert_eq!(tasks[0].description, "Installing Firefox");
        assert_eq!(tasks[0].command.last().map(String::as_str), Some("install"));
        assert_eq!(tasks[1].description, "Removing htop");
        assert_eq!(tasks[1].command.last().map(String::as_str), Some("remove"));
        assert_eq!(tasks[2].description, "Applying Bibata Modern Classic");
        assert_eq!(
            tasks[2].command.last().map(String::as_str),
            Some("Bibata-Modern-Classic")
        );
        assert!(!crate::runner::command_needs_privileges(&tasks[2].command));
        assert_eq!(tasks[3].description, "Running Update System");
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.cursor_selected, Some(CursorTheme::Classic));
        assert!(app.admin_selected.contains(&0));
    }

    #[test]
    fn review_lists_all_staged_names_grouped() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.install_selected.insert(1);
        app.remove_selected.insert(0);
        app.cursor_selected = Some(CursorTheme::Ice);
        app.admin_selected.insert(0);

        let items: Vec<(&str, String)> = app
            .queue_items()
            .into_iter()
            .map(|(verb, name, _)| (verb, name))
            .collect();
        assert_eq!(
            items,
            vec![
                ("Install", "Firefox".to_owned()),
                ("Install", "VLC".to_owned()),
                ("Remove", "htop".to_owned()),
                ("Apply", "Bibata Modern Ice".to_owned()),
                ("Run", "Update everything".to_owned()),
            ]
        );
        let (_, _, key) = app.queue_items().remove(0);
        app.unstage(key);
        assert_eq!(app.install_selected_names(), vec!["VLC".to_owned()]);
        assert!(app.remove_includes_toolbox().is_none());
        app.remove_selected.insert(1);
        assert!(app.remove_includes_toolbox().is_some());
    }

    #[test]
    fn review_panel_lists_names_before_password() {
        let src = production_main();
        let modal = src
            .split("fn sudo_modal")
            .nth(1)
            .and_then(|rest| rest.split("fn acknowledge_run_complete").next())
            .expect("sudo_modal body");
        assert!(modal.contains("Ready to run"));
        assert!(modal.contains("queue_items"));
        assert!(modal.contains("sudo_name_list"));
        assert!(modal.contains("Clear all"));
        let name_list = modal.find("sudo_name_list").expect("name list");
        let password = modal.find("Your password").expect("password label");
        assert!(
            name_list < password,
            "queued names must appear before the password field"
        );
    }

    #[test]
    fn switching_apps_mode_does_not_clear_other_selection() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.cursor_selected = Some(CursorTheme::Amber);
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
        assert_eq!(app.cursor_selected, Some(CursorTheme::Amber));
        assert_eq!(app.staged_count(AppsMode::Install), 1);
        assert_eq!(app.staged_count(AppsMode::Remove), 1);
        assert_eq!(app.total_staged_count(), 4);
        assert!(app.dock_visible());

        app.apps_mode = AppsMode::Install;
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.dock_count(), 4);

        app.switch_page(Page::Home);
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.cursor_selected, Some(CursorTheme::Amber));
        assert!(app.dock_visible());
    }

    #[test]
    fn dock_clear_clears_all_staged_selections() {
        let mut app = test_app();
        app.apps_mode = AppsMode::Install;
        app.install_selected.insert(0);
        app.install_selected.insert(1);
        app.remove_selected.insert(0);
        app.cursor_selected = Some(CursorTheme::Ice);
        app.admin_selected.insert(0);

        assert_eq!(app.dock_count(), 5);
        assert!(app.dock_visible());
        app.clear_dock_selections();
        assert!(app.install_selected.is_empty());
        assert!(app.remove_selected.is_empty());
        assert!(app.cursor_selected.is_none());
        assert!(app.admin_selected.is_empty());
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
        assert!(app.dock_visible());
        // Selections on another mode still surface the dock on every page.
        app.switch_page(Page::Apps);
        app.apps_mode = AppsMode::Install;
        app.install_selected.clear();
        app.remove_selected.insert(0);
        assert!(app.dock_visible());
        assert_eq!(app.dock_count(), 1);
    }

    #[test]
    fn run_freezes_selections_while_running_then_clears_on_finish() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.cursor_selected = Some(CursorTheme::Classic);
        app.admin_selected.insert(0);
        app.is_running = true;
        assert!(app.selection_locked());
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.cursor_selected, Some(CursorTheme::Classic));
        assert!(app.admin_selected.contains(&0));

        app.finish_run();
        assert!(!app.is_running);
        assert!(app.install_selected.is_empty());
        assert!(app.remove_selected.is_empty());
        assert!(app.cursor_selected.is_none());
        assert!(app.admin_selected.is_empty());
        assert!(!app.dock_visible());
        assert!(app.show_run_complete_modal);
        assert_eq!(app.recent_runs.len(), 4);
        assert!(
            app.recent_runs
                .iter()
                .any(|run| run.kind == "Cursor" && run.summary.contains("Classic"))
        );

        app.acknowledge_run_complete();
        assert!(!app.show_run_complete_modal);
    }

    #[test]
    fn direct_cursor_apply_keeps_staged_install_and_remove() {
        let mut app = test_app();
        app.install_selected.insert(0);
        app.remove_selected.insert(0);
        app.cursor_selected = Some(CursorTheme::Ice);
        app.direct_cursor = Some("Bibata Modern Amber".to_owned());

        app.finish_run();
        assert!(app.install_selected.contains(&0));
        assert!(app.remove_selected.contains(&0));
        assert_eq!(app.cursor_selected, Some(CursorTheme::Ice));
        assert!(app.direct_cursor.is_none());
        assert_eq!(app.recent_runs.len(), 1);
        assert_eq!(app.recent_runs[0].kind, "Cursor");
        assert_eq!(app.recent_runs[0].summary, "Bibata Modern Amber");
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
    fn catalog_labels_resolve_to_icon_keys() {
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
            "Brave Origin",
            "Brave Browser",
            "Zen",
        ] {
            assert_ne!(icon_key(label), "", "{label}");
        }
        assert_eq!(icon_key("Brave Origin"), "brave-origin");
        assert_eq!(icon_key("Brave Browser"), "brave");
        assert_eq!(icon_key("Zen"), "zen-browser");
        assert_eq!(icon_key("ProtonPlus"), "protonplus");
        assert_eq!(icon_key("com.vysp3r.ProtonPlus"), "protonplus");
        assert_eq!(icon_key("LocalSend"), "localsend");
        assert_eq!(icon_key("Mission Center"), "mission-center");
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
    fn chrome_uses_apps_mode_and_top_tabs() {
        let src = production_main();
        assert!(src.contains("enum AppsMode"));
        assert!(src.contains("Page::Apps"));
        assert!(src.contains("Page::Cursor"));
        assert!(!src.contains("Page::Install"));
        assert!(!src.contains("Page::Remove"));
        assert!(!src.contains("Page::Administration"));
        assert!(!src.contains("Supported Distros"));
        assert!(!src.contains("Select All"));
        assert!(src.contains("Review & run"));
        assert!(src.contains("run_dock"));
        assert_eq!(TOPBAR_HEIGHT, 60.0);
        assert_eq!(CARD_HEIGHT, 178.0);
        assert_eq!(TITLE_SIZE, 44.0);
        assert_eq!(INTENT_HEIGHT, 190.0);
        assert_eq!(GLANCE_HEIGHT, 84.0);
        assert_eq!(CARD_RADIUS, 0.0);
        assert_eq!(DOCK_MIN_WIDTH, 520.0);
        assert!(src.contains("tasks queued"));
        assert!(src.contains("fn topbar"));
        assert!(src.contains("fn home_page"));
        assert!(src.contains("fn recipes_page"));
        assert!(!src.contains("fn stub_page"));
        assert!(!src.contains("Home is coming"));
        assert!(!src.contains("Recipes is coming"));
    }

    #[test]
    fn skin_swatches_pick_family_or_white_light() {
        let src = production_main();
        assert!(src.contains("fn set_skin"));
        let bar = src
            .split("fn topbar(")
            .nth(1)
            .and_then(|rest| rest.split("fn apps_page").next())
            .expect("topbar");
        assert!(bar.contains("light_swatch"));
        assert!(bar.contains("family_swatch"));
        let swatch = src
            .split("fn family_swatch")
            .nth(1)
            .and_then(|rest| rest.split("fn mode_segment").next())
            .expect("family_swatch");
        assert!(swatch.contains("skin.accent"));
        assert!(swatch.contains("Color32::WHITE"));
        assert!(!swatch.contains("caution"));
        assert!(!swatch.contains("CAUTION"));
    }

    #[test]
    fn apps_chrome_paints_family_accent_and_keeps_caution_off_cta() {
        let src = production_main();
        let card = src
            .split("fn paint_card_background")
            .nth(1)
            .and_then(|rest| rest.split("fn paint_check").next())
            .expect("paint_card_background");
        assert!(card.contains("palette.tile_selected"));
        assert!(card.contains("palette.accent_dim"));
        assert!(card.contains("palette.accent"));
        assert!(!card.contains("caution"));
        let check = src
            .split("fn paint_check")
            .nth(1)
            .and_then(|rest| rest.split("fn paint_app_icon").next())
            .expect("paint_check");
        assert!(check.contains("palette.accent"));
        assert!(!check.contains("caution"));
        let rail = src
            .split("fn tab_button")
            .nth(1)
            .and_then(|rest| rest.split("fn queue_button").next())
            .expect("tab_button");
        assert!(rail.contains("palette.accent"));
        assert!(!rail.contains("caution"));
        let filter = src
            .split("fn filter_chip")
            .nth(1)
            .and_then(|rest| rest.split("fn search_field").next())
            .expect("filter_chip");
        assert!(filter.contains("palette.accent"));
        assert!(!filter.contains("caution"));
        let cta = src
            .split("fn cta_button")
            .nth(1)
            .and_then(|rest| rest.split("fn ghost_button").next())
            .expect("cta_button");
        assert!(cta.contains("palette.cta_fill"));
        assert!(!cta.contains("caution"));
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
        assert!(app.admin_selected_names().is_empty());

        app.go_laptop_power();
        assert_eq!(app.page, Page::Recipes);
        assert!(app.toast.is_some());
        let selected = app.admin_selected_names();
        assert_eq!(
            selected,
            vec!["TLP (Laptops)".to_owned(), "Powertop".to_owned()]
        );
        assert_eq!(
            truncated_names(&app.dock_names(), 56),
            "TLP (Laptops), Powertop"
        );
    }

    #[test]
    fn home_glance_uses_live_fields_not_fake_cpu() {
        let app = test_app();
        let metrics = app.glance_metrics();
        assert_eq!(metrics[0].0, "Operating system");
        assert_eq!(metrics[1].0, "Kernel");
        assert_eq!(metrics[2].0, "Running for");
        assert_eq!(metrics[3].0, "Installs with");
        assert_eq!(metrics[0].1, "Debian");
        assert_eq!(metrics[1].1, "6.12.0-test");
        assert_eq!(metrics[3].1, "apt-get");
        let src = production_main();
        assert!(src.contains("Your computer"));
        assert!(!src.contains("\"CPU\""));
        assert!(src.contains("Nothing yet. Pick some apps or a recipe"));
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
        assert!(home.contains("One-click recipes for updates, Bluetooth and essentials."));
        assert!(home.contains("Browse the catalog and pick what you need."));
        assert!(home.contains("TLP and Powertop tuned for laptops."));
        assert!(home.contains("What would you like to do?"));
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
        assert_eq!(app.dock_count(), 3);
        let names: Vec<String> = app.queue_items().into_iter().map(|item| item.1).collect();
        assert_eq!(
            names,
            vec![
                "Firefox".to_owned(),
                "Update everything".to_owned(),
                "Show system info in the terminal".to_owned(),
            ]
        );

        let tasks = app.selected_tasks().expect("combined tasks");
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].description, "Installing Firefox");
        assert!(
            tasks[1..]
                .iter()
                .all(|task| task.description.starts_with("Running "))
        );
        assert!(app.install_selected.contains(&0));

        app.clear_dock_selections();
        assert!(app.admin_selected.is_empty());
        assert!(app.install_selected.is_empty());
        assert!(!app.dock_visible());

        app.admin_selected.insert(5);
        assert_eq!(
            app.admin_selected_names(),
            vec!["nala (rank mirrors)".to_owned()]
        );
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
        assert!(app.dock_visible());
        assert!(app.admin_selected.contains(&0));
    }

    #[test]
    fn every_admin_task_has_a_title_and_promise() {
        let tasks = admin_tasks();
        assert_eq!(tasks.len(), 9);
        for task in &tasks {
            assert_ne!(
                recipe_promise(&task.label),
                "Admin playbook for this machine.",
                "{}",
                task.label
            );
            assert_ne!(
                recipe_title(&task.label),
                "Admin playbook",
                "{}",
                task.label
            );
            assert_ne!(recipe_list_rank(&task.label), 100, "{}", task.label);
        }
        assert_eq!(recipe_title("Update System"), "Update everything");
        assert_eq!(
            recipe_promise("Enable Bluetooth"),
            "Enable Bluetooth and start it at boot."
        );
        assert_eq!(
            recipe_display_label("nala (rank mirrors) - Debian only"),
            "nala (rank mirrors)"
        );
    }

    #[test]
    fn system_page_shows_four_cards_from_live_data() {
        let src = production_main();
        let system = src
            .split("fn system_info_page")
            .nth(1)
            .and_then(|rest| rest.split("struct SystemCard").next())
            .expect("system_info_page");
        for title in ["System", "Processor", "Memory", "Storage"] {
            assert!(system.contains(&format!("title: \"{title}\"")), "{title}");
        }
        assert!(system.contains("This computer"));
        assert!(system.contains("HardwareSnapshot") || system.contains("hw."));
        assert!(!system.contains("command_output"));
        assert!(!system.contains("\"CPU\""));
    }

    #[test]
    fn every_page_reserves_dock_padding() {
        let src = production_main();
        assert!(src.contains("bottom: 120,"));
        let mut app = test_app();
        app.install_selected.insert(0);
        for page in [
            Page::Home,
            Page::Apps,
            Page::Recipes,
            Page::Cursor,
            Page::System,
        ] {
            app.switch_page(page);
            assert!(app.dock_visible(), "{page:?}");
        }
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

        app.admin_selected.insert(0);
        app.remove_selected.insert(0);
        app.record_recent_run();
        assert_eq!(app.recent_runs[0].kind, "Install");
        assert_eq!(app.recent_runs[1].kind, "Remove");
        assert_eq!(app.recent_runs[2].kind, "Recipe");
        assert_eq!(app.recent_runs.len(), 4);
    }
}
