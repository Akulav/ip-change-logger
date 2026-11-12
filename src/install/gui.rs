//! The two bits of UI the install model needs, in plain egui so they fit any screen:
//! the install prompt (Linux/macOS, started outside the install folder) and the update button.

use super::{update, Offer};
use eframe::egui::{self, RichText};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------
// Install prompt
// ---------------------------------------------------------------------------------------------

pub enum PromptResult {
    /// Still showing.
    Open,
    /// Installed (or updated) and the installed copy started: close this window.
    Close,
    /// "Just run it": carry on with this copy.
    JustRun,
}

pub struct InstallPrompt {
    offer: Offer,
    dir: String,
    error: Option<String>,
}

impl InstallPrompt {
    pub fn new(offer: Offer) -> Option<Self> {
        let dir = match &offer {
            Offer::None => return None,
            Offer::Install { suggested } => suggested.display().to_string(),
            Offer::Update { root, .. } => root.display().to_string(),
        };
        Some(Self { offer, dir, error: None })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, what_is_stored: &str) -> PromptResult {
        let mut result = PromptResult::Open;
        ui.add_space((ui.available_height() * 0.5 - 170.0).max(16.0));
        ui.vertical_centered(|ui| {
            egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(24)).show(ui, |ui| {
                ui.set_width(520.0_f32.min(ui.available_width()));
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    let app = super::APP_NAME;
                    let updating = matches!(self.offer, Offer::Update { .. });
                    ui.label(
                        RichText::new(format!("Version {}  ·  made by Akulav", super::CURRENT_VERSION))
                            .size(12.0)
                            .weak(),
                    );
                    let title = if updating { format!("Update {app}") } else { format!("Install {app}") };
                    ui.label(RichText::new(title).size(20.0).strong());
                    ui.add_space(8.0);
                    match &self.offer {
                        Offer::Update { root, installed } => {
                            let from = installed
                                .as_deref()
                                .map(|v| format!("version {v}"))
                                .unwrap_or_else(|| "an older version".into());
                            ui.label(format!(
                                "{app} {from} is installed in {}. Update it to {}? {what_is_stored} stay where they are.",
                                root.display(),
                                super::CURRENT_VERSION
                            ));
                        }
                        _ => {
                            ui.label(format!(
                                "Choose a folder. {app} and everything it saves ({what_is_stored}) go there, and updates keep using it."
                            ));
                            ui.add_space(10.0);
                            ui.label(RichText::new("Install folder").strong());
                            if ui.add(egui::TextEdit::singleline(&mut self.dir).desired_width(f32::INFINITY)).changed()
                            {
                                self.error = None;
                            }
                            let extras = if cfg!(target_os = "macos") {
                                format!("Creates {app}.app in this folder.")
                            } else {
                                format!("Also adds an app-menu entry and the {} command.", super::APP_ID)
                            };
                            ui.label(RichText::new(extras).size(12.0).weak());
                        }
                    }
                    if let Some(err) = &self.error {
                        ui.add_space(6.0);
                        ui.colored_label(egui::Color32::from_rgb(0xf0, 0x6a, 0x6a), err);
                    }
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        let label = if updating { "Update" } else { "Install" };
                        if ui.button(RichText::new(label).strong()).clicked() {
                            let r = match &self.offer {
                                Offer::Update { root, installed } => {
                                    let from = installed.clone().unwrap_or_default();
                                    let args: Vec<&str> = if from.is_empty() {
                                        Vec::new()
                                    } else {
                                        vec!["--updated-from", from.as_str()]
                                    };
                                    super::install_and_launch(root, &args)
                                }
                                _ => super::parse_install_dir(&self.dir)
                                    .and_then(|d| super::install_and_launch(&d, &["--installed"])),
                            };
                            match r {
                                Ok(()) => result = PromptResult::Close,
                                Err(e) => self.error = Some(e),
                            }
                        }
                        if ui.button("Just run it").on_hover_text("Run this copy without installing it").clicked() {
                            result = PromptResult::JustRun;
                        }
                    });
                });
            });
        });
        result
    }
}

// ---------------------------------------------------------------------------------------------
// Updates
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum State {
    Idle,
    Checking,
    UpToDate,
    Available(update::UpdateInfo),
    Downloading { done: u64, total: u64 },
    Ready(update::UpdateInfo, PathBuf),
    Error(String),
}

/// Checks GitHub 10 s after start and every 6 hours, downloads into the install folder on request
/// and applies the update with one more click. Only installed release builds update themselves.
pub struct Updater {
    state: Arc<Mutex<State>>,
    started: Instant,
    last_check: Option<Instant>,
}

impl Updater {
    pub fn new() -> Self {
        Self { state: Arc::new(Mutex::new(State::Idle)), started: Instant::now(), last_check: None }
    }

    fn enabled() -> bool {
        !cfg!(debug_assertions)
            && update::this_os_asset().is_some()
            && super::program_root().is_some_and(|r| super::is_install_root(&r))
    }

    fn get(&self) -> State {
        self.state.lock().map(|s| s.clone()).unwrap_or(State::Idle)
    }

    fn set(state: &Arc<Mutex<State>>, s: State) {
        if let Ok(mut g) = state.lock() {
            *g = s;
        }
    }

    pub fn check_now(&mut self, ctx: &egui::Context) {
        if !Self::enabled() || matches!(self.get(), State::Checking | State::Downloading { .. }) {
            return;
        }
        self.last_check = Some(Instant::now());
        Self::set(&self.state, State::Checking);
        let (state, ctx) = (self.state.clone(), ctx.clone());
        std::thread::spawn(move || {
            let s = match update::check() {
                Ok(Some(info)) => State::Available(info),
                Ok(None) => State::UpToDate,
                Err(e) => State::Error(e),
            };
            Self::set(&state, s);
            ctx.request_repaint();
        });
    }

    /// Call every frame (cheap).
    pub fn tick(&mut self, ctx: &egui::Context) {
        let due = match self.last_check {
            None => self.started.elapsed() >= Duration::from_secs(10),
            Some(t) => t.elapsed() >= Duration::from_secs(6 * 3600),
        };
        if due && Self::enabled() && matches!(self.get(), State::Idle | State::UpToDate | State::Error(_)) {
            self.check_now(ctx);
        }
    }

    fn download(&mut self, ctx: &egui::Context, info: update::UpdateInfo) {
        Self::set(&self.state, State::Downloading { done: 0, total: 0 });
        let (state, ctx) = (self.state.clone(), ctx.clone());
        std::thread::spawn(move || {
            let progress = |done: u64, total: u64| {
                Self::set(&state, State::Downloading { done, total });
                ctx.request_repaint();
            };
            let s = match super::download_update(&info, &progress) {
                Ok(path) => State::Ready(info, path),
                Err(e) => State::Error(e),
            };
            Self::set(&state, s);
            ctx.request_repaint();
        });
    }

    /// A compact update control for a footer or toolbar. Closes the window when an update is applied.
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        match self.get() {
            State::Available(info) => {
                if ui
                    .button(RichText::new(format!("Update to {}", info.version)).strong())
                    .on_hover_text("Download the new version from GitHub (verified with SHA-256)")
                    .clicked()
                {
                    self.download(&ctx, info);
                }
            }
            State::Downloading { done, total } => {
                let pct = (done * 100).checked_div(total).unwrap_or(0);
                ui.label(RichText::new(format!("Downloading update… {pct}%")).size(12.0));
            }
            State::Ready(info, path) => {
                if ui.button(RichText::new("Restart & update").strong()).clicked() {
                    match super::apply_update(&path, &info.sha256) {
                        Ok(()) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
                        Err(e) => Self::set(&self.state, State::Error(e)),
                    }
                }
            }
            State::Error(e) => {
                if ui
                    .small_button("Retry update check")
                    .on_hover_text(format!("Update check failed: {e}"))
                    .clicked()
                {
                    self.check_now(&ctx);
                }
            }
            State::Checking => {
                ui.label(RichText::new("Checking for updates…").size(12.0).weak());
            }
            State::Idle | State::UpToDate => {
                if Self::enabled() && ui.small_button("Check for updates").clicked() {
                    self.check_now(&ctx);
                }
            }
        }
    }
}
