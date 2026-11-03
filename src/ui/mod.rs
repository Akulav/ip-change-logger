//! The desktop window.

mod theme;
#[cfg(windows)]
mod tray;
mod widgets;

use crate::config::{Paths, Settings, MAX_INTERVAL_SECS, MIN_INTERVAL_SECS};
use crate::history::Entry;
use crate::monitor::{Command, Event, Monitor, State};
use crate::net::intel::{IpIntel, Risk};
use chrono::{DateTime, Local};
use eframe::egui::{self, Color32, FontId, Layout, RichText, ScrollArea, Ui, ViewportCommand};
use std::path::Path;
use std::time::{Duration, Instant};
use theme::{AMBER, CYAN, FAINT, GREEN, INDIGO, MUTED, RED, TEXT};
use widgets::{
    caption, card, chip, ghost_button, kv_row, pill, primary_button, section_title, signal_row, stat, Glyph,
};

pub const APP_NAME: &str = "IPTrail";
const REPO_URL: &str = "https://github.com/Akulav/ip-change-logger";

pub fn run(paths: Paths, settings: Settings) -> eframe::Result {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../../assets/icon-256.png")).unwrap_or_default();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id("iptrail")
            .with_inner_size([1000.0, 720.0])
            .with_min_inner_size([780.0, 560.0])
            .with_icon(icon),
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(|cc| Ok(Box::new(App::new(cc, paths, settings)))))
}

struct App {
    monitor: Monitor,
    paths: Paths,
    settings: Settings,
    logo: Option<egui::TextureHandle>,
    settings_open: bool,
    log_path_edit: String,
    confirm_clear: bool,
    toast: Option<(String, Instant)>,
    #[cfg(windows)]
    tray: Option<tray::Tray>,
    quitting: bool,
    was_minimized: bool,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, paths: Paths, settings: Settings) -> Self {
        theme::apply(&cc.egui_ctx);
        let logo = load_logo(&cc.egui_ctx);

        let ctx = cc.egui_ctx.clone();
        let monitor = Monitor::spawn(paths.clone(), settings.clone(), move |event| {
            if let Event::Error(e) = &event {
                eprintln!("iptrail: {e}");
            }
            ctx.request_repaint();
        });

        #[cfg(windows)]
        let tray = match tray::Tray::new(&cc.egui_ctx) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("iptrail: tray icon unavailable: {e}");
                None
            }
        };

        Self {
            log_path_edit: settings.log_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            monitor,
            paths,
            settings,
            logo,
            settings_open: false,
            confirm_clear: false,
            toast: None,
            #[cfg(windows)]
            tray,
            quitting: false,
            was_minimized: false,
        }
    }

    fn has_tray(&self) -> bool {
        #[cfg(windows)]
        return self.tray.is_some();
        #[cfg(not(windows))]
        return false;
    }

    fn apply_settings(&mut self) {
        self.settings = self.settings.clone().sanitized();
        if let Err(e) = self.settings.save(&self.paths) {
            self.notify_toast(format!("Could not save settings: {e}"));
        }
        self.monitor.send(Command::UpdateSettings(self.settings.clone()));
    }

    fn notify_toast(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
    }

    fn show_window(ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
    }

    fn hide_window(ctx: &egui::Context) {
        ctx.send_viewport_cmd(ViewportCommand::Visible(false));
    }
}

fn load_logo(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory(include_bytes!("../../assets/icon-64.png")).ok()?.into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture("logo", color, egui::TextureOptions::LINEAR))
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        if let Some(tray) = &mut self.tray {
            while let Some(msg) = tray.poll() {
                match msg {
                    tray::TrayMsg::Show => Self::show_window(ctx),
                    tray::TrayMsg::CheckNow => self.monitor.send(Command::CheckNow),
                    tray::TrayMsg::ToggleMonitoring => {
                        let running = self.monitor.state().running;
                        self.monitor.send(if running { Command::Pause } else { Command::Start });
                    }
                    tray::TrayMsg::Quit => {
                        self.quitting = true;
                        Self::show_window(ctx);
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                }
            }
            let (tooltip, running) = {
                let s = self.monitor.state();
                let ip = s.current.map(|ip| ip.to_string()).unwrap_or_else(|| "no IP yet".into());
                let status = if s.running { "monitoring" } else { "paused" };
                (format!("IPTrail — {ip} ({status})"), s.running)
            };
            tray.sync(tooltip, running);
        }

        let tray = self.has_tray();
        let minimized = ctx.input(|i| i.viewport().minimized).unwrap_or(false);
        if minimized && !self.was_minimized && tray && self.settings.minimize_to_tray {
            Self::hide_window(ctx);
        }
        self.was_minimized = minimized;

        if ctx.input(|i| i.viewport().close_requested()) {
            if tray && self.settings.close_to_tray && !self.quitting {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                Self::hide_window(ctx);
            } else {
                self.monitor.send(Command::Shutdown);
            }
        }
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        // Countdown and relative times tick once per second.
        ctx.request_repaint_after(Duration::from_secs(1));

        egui::Panel::top("top")
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 16,
                bottom: 8,
            }))
            .show(ui, |ui| self.top_bar(ui));

        egui::Panel::bottom("footer")
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 6,
                bottom: 12,
            }))
            .show(ui, |ui| self.footer(ui));

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::BG).inner_margin(egui::Margin {
                left: 24,
                right: 24,
                top: 8,
                bottom: 8,
            }))
            .show(ui, |ui| self.dashboard(ui));

        self.settings_window(&ctx);
        self.clear_confirmation(&ctx);
        self.toast_overlay(&ctx);
    }

    fn on_exit(&mut self) {
        self.monitor.send(Command::Shutdown);
    }
}

// ---------------------------------------------------------------------------
// Sections

impl App {
    fn top_bar(&mut self, ui: &mut Ui) {
        let (running, checking) = {
            let s = self.monitor.state();
            (s.running, s.checking)
        };
        ui.horizontal(|ui| {
            if let Some(logo) = &self.logo {
                ui.add(egui::Image::new(logo).fit_to_exact_size(egui::vec2(34.0, 34.0)));
            }
            ui.add_space(4.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(RichText::new(APP_NAME).font(FontId::new(21.0, theme::strong())).color(TEXT));
                ui.label(RichText::new("Public IP monitor").size(12.0).color(MUTED));
            });
            ui.add_space(8.0);
            if checking {
                pill(ui, "Checking…", CYAN);
            } else if running {
                pill(ui, "Monitoring", GREEN);
            } else {
                pill(ui, "Paused", AMBER);
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ghost_button(ui, "⚙  Settings").clicked() {
                    self.settings_open = true;
                }
                if ghost_button(ui, "⟳  Check now").on_hover_text("Check the public IP immediately").clicked()
                {
                    self.monitor.send(Command::CheckNow);
                }
                let (label, color, cmd) = if running {
                    ("Pause", AMBER, Command::Pause)
                } else {
                    ("Start monitoring", GREEN, Command::Start)
                };
                if primary_button(ui, label, color).clicked() {
                    self.monitor.send(cmd);
                }
            });
        });
        ui.add_space(6.0);
    }

    fn dashboard(&mut self, ui: &mut Ui) {
        let mut copy: Option<String> = None;
        let mut open_folder = false;
        let mut ask_clear = false;

        {
            let s = self.monitor.state();
            ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                hero(ui, &s, &mut copy);
                ui.add_space(12.0);
                stats_row(ui, &s);
                ui.add_space(12.0);

                let wide = ui.available_width() > 860.0;
                if wide {
                    ui.columns(2, |cols| {
                        signals_card(&mut cols[0], s.intel.as_ref(), s.current.is_some());
                        cols[0].add_space(12.0);
                        details_card(&mut cols[0], s.intel.as_ref());
                        history_card(&mut cols[1], &s, &mut open_folder, &mut ask_clear, &mut copy);
                    });
                } else {
                    signals_card(ui, s.intel.as_ref(), s.current.is_some());
                    ui.add_space(12.0);
                    details_card(ui, s.intel.as_ref());
                    ui.add_space(12.0);
                    history_card(ui, &s, &mut open_folder, &mut ask_clear, &mut copy);
                }
            });
        }

        if let Some(text) = copy {
            ui.ctx().copy_text(text.clone());
            self.notify_toast(format!("Copied {text}"));
        }
        if open_folder {
            let log = self.settings.log_file(&self.paths);
            let dir = log.parent().unwrap_or(self.paths.data_dir()).to_path_buf();
            if let Err(e) = open_path(&dir) {
                self.notify_toast(format!("Could not open folder: {e}"));
            }
        }
        if ask_clear {
            self.confirm_clear = true;
        }
    }

    fn footer(&mut self, ui: &mut Ui) {
        let s = self.monitor.state();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            if let Some(err) = &s.last_error {
                ui.label(RichText::new(format!("⚠ {err}")).size(12.0).color(RED));
            } else if let Some(w) = s.warnings.first() {
                ui.label(RichText::new(format!("⚠ {w}")).size(12.0).color(AMBER))
                    .on_hover_text(s.warnings.join("\n"));
            } else if let Some(p) = s.provider {
                ui.label(RichText::new(format!("● Connected via {p}")).size(12.0).color(MUTED));
            }

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.hyperlink_to(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).size(12.0),
                    REPO_URL,
                );
                ui.label(
                    RichText::new("Data: ip-api.com · ipwho.is · Tor Project · blocklist.de · FireHOL")
                        .size(12.0)
                        .color(FAINT),
                );
            });
        });
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let mut changed = false;
        let tray = self.has_tray();
        egui::Window::new(RichText::new("Settings").font(FontId::new(17.0, theme::strong())))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                let st = &mut self.settings;

                caption(ui, "Monitoring");
                ui.horizontal(|ui| {
                    ui.label("Check every");
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut st.interval_secs, MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS)
                                .logarithmic(true)
                                .suffix(" s"),
                        )
                        .drag_stopped();
                });
                changed |= ui.checkbox(&mut st.auto_start, "Start monitoring when IPTrail opens").changed();
                changed |= ui.checkbox(&mut st.notifications, "Desktop notification when the IP changes").changed();

                ui.add_space(10.0);
                caption(ui, "Intelligence (no API key needed)");
                changed |= ui
                    .checkbox(&mut st.ip_intel, "Look up location, network and proxy/VPN status")
                    .on_hover_text("ip-api.com, falling back to ipwho.is")
                    .changed();
                changed |= ui
                    .checkbox(&mut st.threat_feeds, "Check Tor and abuse blocklists")
                    .on_hover_text("Tor Project exit list, blocklist.de and FireHOL level 1 — downloaded and cached locally")
                    .changed();

                if tray {
                    ui.add_space(10.0);
                    caption(ui, "System tray");
                    changed |= ui.checkbox(&mut st.minimize_to_tray, "Minimize to tray").changed();
                    changed |= ui.checkbox(&mut st.close_to_tray, "Close button hides to tray").changed();
                }

                ui.add_space(10.0);
                caption(ui, "Change log file");
                ui.horizontal(|ui| {
                    let hint = self.paths.default_log.display().to_string();
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.log_path_edit)
                            .hint_text(hint)
                            .desired_width(ui.available_width() - 70.0),
                    );
                    if resp.lost_focus() {
                        let trimmed = self.log_path_edit.trim();
                        st.log_path = (!trimmed.is_empty()).then(|| trimmed.into());
                        changed = true;
                    }
                    if ui.button("Reset").clicked() {
                        self.log_path_edit.clear();
                        st.log_path = None;
                        changed = true;
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.link("Open data folder").clicked() {
                        let _ = open_path(self.paths.data_dir());
                    }
                    ui.label(RichText::new("·").color(FAINT));
                    ui.hyperlink_to("Project page", REPO_URL);
                });
            });
        if changed {
            self.apply_settings();
        }
        self.settings_open = open;
    }

    fn clear_confirmation(&mut self, ctx: &egui::Context) {
        if !self.confirm_clear {
            return;
        }
        let modal = egui::Modal::new(egui::Id::unique("confirm-clear")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(RichText::new("Clear history?").font(FontId::new(17.0, theme::strong())));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Removes every recorded change from IPTrail. The plain-text log file is kept.")
                    .color(MUTED),
            );
            ui.add_space(12.0);
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, "Clear", RED).clicked() {
                    self.monitor.send(Command::ClearHistory);
                    self.confirm_clear = false;
                }
                if ghost_button(ui, "Cancel").clicked() {
                    self.confirm_clear = false;
                }
            });
        });
        if modal.should_close() {
            self.confirm_clear = false;
        }
    }

    fn toast_overlay(&mut self, ctx: &egui::Context) {
        let Some((text, at)) = &self.toast else { return };
        if at.elapsed() > Duration::from_millis(2200) {
            self.toast = None;
            return;
        }
        egui::Area::new(egui::Id::unique("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -48.0))
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(theme::SURFACE_HI)
                    .stroke(egui::Stroke::new(1.0, theme::tint(CYAN, 0.5)))
                    .corner_radius(10)
                    .inner_margin(egui::Margin::symmetric(14, 8))
                    .show(ui, |ui| ui.label(RichText::new(text.as_str()).color(TEXT)));
            });
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}

// ---------------------------------------------------------------------------
// Dashboard pieces (pure functions of the monitor state)

fn hero(ui: &mut Ui, s: &State, copy: &mut Option<String>) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                caption(ui, "Current public IP");
                ui.add_space(2.0);
                ui.horizontal(|ui| match s.current {
                    Some(ip) => {
                        ui.label(
                            RichText::new(ip.to_string()).font(FontId::monospace(40.0)).color(Color32::WHITE),
                        );
                        if ui.button("📋").on_hover_text("Copy to clipboard").clicked() {
                            *copy = Some(ip.to_string());
                        }
                    }
                    None if s.checking || s.running => {
                        ui.spinner();
                        ui.label(RichText::new("Looking up your address…").size(20.0).color(MUTED));
                    }
                    None => {
                        ui.label(RichText::new("Not checked yet").size(26.0).color(MUTED));
                    }
                });
                if let Some(intel) = &s.intel {
                    ui.horizontal_wrapped(|ui| {
                        if let Some(cc) = &intel.country_code {
                            chip(ui, cc, CYAN);
                        }
                        if let Some(place) = intel.place() {
                            ui.label(RichText::new(place).color(TEXT));
                        }
                        if let Some(net) = intel.network() {
                            ui.label(RichText::new(format!("· {net}")).color(MUTED));
                        }
                    });
                }
                if let Some(at) = s.last_change {
                    ui.label(
                        RichText::new(format!("Held since {} ({})", at.format("%b %e, %H:%M"), ago(at)))
                            .size(12.5)
                            .color(FAINT),
                    );
                }
            });

            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                risk_badge(ui, s.intel.as_ref());
            });
        });
    });
}

fn risk_badge(ui: &mut Ui, intel: Option<&IpIntel>) {
    let risk = intel.map(IpIntel::risk).unwrap_or(Risk::Unknown);
    let (title, sub, color, icon) = match risk {
        Risk::Clean => ("Clean", "No proxy, Tor or abuse signals", GREEN, Glyph::Check),
        Risk::Masked => ("Masked", "VPN, proxy, Tor or data-center address", INDIGO, Glyph::Shield),
        Risk::Threat => ("Flagged", "Listed on an attack/abuse blocklist", RED, Glyph::Alert),
        Risk::Unknown => ("Unknown", "No intelligence yet", FAINT, Glyph::Question),
    };
    egui::Frame::new()
        .fill(theme::tint(color, 0.10))
        .stroke(egui::Stroke::new(1.0, theme::tint(color, 0.40)))
        .corner_radius(12)
        .inner_margin(egui::Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_min_width(190.0);
            ui.horizontal(|ui| {
                widgets::glyph(ui, icon, color, 34.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.label(RichText::new(title).font(FontId::new(18.0, theme::strong())).color(color));
                    ui.label(RichText::new(sub).size(11.5).color(MUTED));
                });
            });
        });
}

fn stats_row(ui: &mut Ui, s: &State) {
    let entries = s.history.entries();
    let today = Local::now().date_naive();
    let changes_today = entries.iter().filter(|e| e.at.date_naive() == today && e.previous.is_some()).count();

    let next = match (s.running, s.checking, s.next_check) {
        (_, true, _) => "checking now".to_string(),
        (true, false, Some(at)) => {
            format!("next in {}", short_duration(at.saturating_duration_since(Instant::now())))
        }
        (true, false, None) => "starting…".to_string(),
        (false, ..) => "monitoring paused".to_string(),
    };

    ui.columns(3, |cols| {
        let (prev, prev_sub) = match s.previous {
            Some(p) => (
                p.to_string(),
                entries.last().map(|e| format!("until {}", e.at.format("%b %e, %H:%M"))).unwrap_or_default(),
            ),
            None => ("—".to_string(), "no change recorded yet".to_string()),
        };
        stat(&mut cols[0], "Previous IP", &prev, &prev_sub, s.previous.is_some());
        let last = s.last_check.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_else(|| "—".into());
        stat(&mut cols[1], "Last check", &last, &next, false);
        let total = entries.iter().filter(|e| e.previous.is_some()).count();
        stat(
            &mut cols[2],
            "Changes",
            &total.to_string(),
            &format!(
                "{changes_today} today · {}",
                plural(s.checks, "check this session", "checks this session")
            ),
            false,
        );
    });
}

fn signals_card(ui: &mut Ui, intel: Option<&IpIntel>, have_ip: bool) {
    card(ui, |ui| {
        section_title(ui, "🛡", "Anonymity & threat signals");
        ui.add_space(8.0);
        let get = |f: fn(&IpIntel) -> Option<bool>| intel.and_then(f);
        egui::Grid::new("signals").num_columns(2).spacing([12.0, 9.0]).striped(false).show(ui, |ui| {
            signal_row(ui, "Anonymous", "Tor or proxy/VPN detected", get(IpIntel::anonymous), INDIGO);
            signal_row(ui, "Threat", "Listed as attacker or abuser", get(IpIntel::threat), RED);
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        egui::Grid::new("signal-details").num_columns(2).spacing([12.0, 9.0]).show(ui, |ui| {
            signal_row(ui, "Proxy / VPN", "ip-api.com proxy, VPN or Tor detection", get(|i| i.proxy), INDIGO);
            signal_row(ui, "Tor exit node", "Tor Project bulk exit list", get(|i| i.tor), INDIGO);
            signal_row(ui, "Data center", "Hosting / cloud provider range", get(|i| i.hosting), INDIGO);
            signal_row(ui, "Mobile network", "Cellular carrier", get(|i| i.mobile), CYAN);
            signal_row(
                ui,
                "Known attacker",
                "blocklist.de — attacks reported in the last 48 h",
                get(|i| i.attacker),
                RED,
            );
            signal_row(
                ui,
                "Known abuser",
                "FireHOL level 1 (Spamhaus DROP, DShield, Feodo…)",
                get(|i| i.abuser),
                RED,
            );
        });
        if intel.is_none() && have_ip {
            ui.add_space(6.0);
            ui.label(
                RichText::new("Intelligence lookups are disabled or still loading.").size(12.0).color(FAINT),
            );
        }
    });
}

fn details_card(ui: &mut Ui, intel: Option<&IpIntel>) {
    card(ui, |ui| {
        section_title(ui, "🌐", "Network details");
        ui.add_space(8.0);
        let i = intel.cloned().unwrap_or_default();
        let coords = match (i.latitude, i.longitude) {
            (Some(lat), Some(lon)) => Some(format!("{lat:.4}, {lon:.4}")),
            _ => None,
        };
        egui::Grid::new("details").num_columns(2).spacing([16.0, 7.0]).show(ui, |ui| {
            kv_row(ui, "Location", i.place().as_deref());
            kv_row(ui, "ISP", i.isp.as_deref());
            kv_row(ui, "Organization", i.org.as_deref());
            kv_row(ui, "ASN", i.asn.as_deref());
            kv_row(ui, "Reverse DNS", i.reverse_dns.as_deref());
            kv_row(ui, "Time zone", i.timezone.as_deref());
            kv_row(ui, "Coordinates", coords.as_deref());
            kv_row(ui, "Source", (!i.source.is_empty()).then_some(i.source.as_str()));
        });
    });
}

fn history_card(
    ui: &mut Ui,
    s: &State,
    open_folder: &mut bool,
    ask_clear: &mut bool,
    copy: &mut Option<String>,
) {
    card(ui, |ui| {
        ui.horizontal(|ui| {
            section_title(ui, "🕘", "Change history");
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("🗑").on_hover_text("Clear history").clicked() {
                    *ask_clear = true;
                }
                if ui.small_button("📁").on_hover_text("Open log folder").clicked() {
                    *open_folder = true;
                }
            });
        });
        ui.add_space(6.0);

        let entries = s.history.entries();
        if entries.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("🛰").size(28.0).color(FAINT));
                ui.label(RichText::new("Every address change will appear here.").color(MUTED));
            });
            ui.add_space(20.0);
            return;
        }

        ScrollArea::vertical().id_salt("history").max_height(470.0).auto_shrink([false, true]).show(
            ui,
            |ui| {
                for (idx, entry) in entries.iter().rev().enumerate() {
                    history_row(ui, entry, idx == 0, copy);
                }
            },
        );
    });
}

fn history_row(ui: &mut Ui, e: &Entry, latest: bool, copy: &mut Option<String>) {
    let accent = if latest { CYAN } else { theme::BORDER };
    let resp = egui::Frame::new()
        .fill(if latest { theme::tint(CYAN, 0.06) } else { Color32::TRANSPARENT })
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 36.0), egui::Sense::hover());
                ui.painter().circle_filled(egui::pos2(rect.center().x, rect.top() + 9.0), 4.5, accent);
                if !latest {
                    ui.painter().circle_stroke(
                        egui::pos2(rect.center().x, rect.top() + 9.0),
                        4.5,
                        (1.0, FAINT),
                    );
                }
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(e.ip.to_string()).font(FontId::monospace(15.0)).color(TEXT));
                        if e.previous.is_none() {
                            chip(ui, "first seen", FAINT);
                        }
                        if let Some(intel) = &e.intel {
                            for flag in intel.flag_labels() {
                                let color = if matches!(flag, "attacker" | "abuser") { RED } else { INDIGO };
                                chip(ui, flag, color);
                            }
                        }
                    });
                    let mut sub = e.at.format("%a %b %e %Y, %H:%M:%S").to_string();
                    if let Some(place) = e.intel.as_ref().and_then(IpIntel::place) {
                        sub.push_str(&format!("  ·  {place}"));
                    }
                    ui.add(egui::Label::new(RichText::new(sub).size(12.0).color(MUTED)).truncate());
                });
            });
        })
        .response;
    let resp = resp.interact(egui::Sense::click());
    if resp.double_clicked() {
        *copy = Some(e.ip.to_string());
    }
    resp.on_hover_text(e.log_line() + "\n\nDouble-click to copy");
}

// ---------------------------------------------------------------------------
// Helpers

fn plural(n: u64, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn ago(at: DateTime<Local>) -> String {
    let secs = (Local::now() - at).num_seconds().max(0) as u64;
    format!("{} ago", short_duration(Duration::from_secs(secs)))
}

fn short_duration(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 => format!("{}m {}s", s / 60, s % 60),
        3600..=86_399 => format!("{}h {}m", s / 3600, (s % 3600) / 60),
        _ => format!("{}d {}h", s / 86_400, (s % 86_400) / 3600),
    }
}

fn open_path(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(windows)]
    let mut cmd = std::process::Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut cmd = std::process::Command::new("open");
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut cmd = std::process::Command::new("xdg-open");
    cmd.arg(path).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_are_compact() {
        assert_eq!(short_duration(Duration::from_secs(5)), "5s");
        assert_eq!(short_duration(Duration::from_secs(125)), "2m 5s");
        assert_eq!(short_duration(Duration::from_secs(7_260)), "2h 1m");
        assert_eq!(short_duration(Duration::from_secs(90_000)), "1d 1h");
    }
}
