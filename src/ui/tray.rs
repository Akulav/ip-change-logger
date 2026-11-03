//! System tray icon (Windows).
//!
//! Tray callbacks run on the UI thread's message loop. They only forward
//! messages; the app handles them in `App::logic`, which eframe calls even
//! while the window is hidden.

use std::sync::mpsc::{self, Receiver};
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

pub enum TrayMsg {
    Show,
    CheckNow,
    ToggleMonitoring,
    Quit,
}

pub struct Tray {
    icon: TrayIcon,
    toggle: MenuItem,
    rx: Receiver<TrayMsg>,
    tooltip: String,
    running: Option<bool>,
}

impl Tray {
    pub fn new(ctx: &eframe::egui::Context) -> Result<Self, String> {
        let show = MenuItem::new("Show IPTrail", true, None);
        let check = MenuItem::new("Check now", true, None);
        let toggle = MenuItem::new("Pause monitoring", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[
            &show,
            &PredefinedMenuItem::separator(),
            &check,
            &toggle,
            &PredefinedMenuItem::separator(),
            &quit,
        ])
        .map_err(|e| e.to_string())?;

        let icon = TrayIconBuilder::new()
            .with_icon(icon()?)
            .with_tooltip("IPTrail")
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|e| e.to_string())?;

        let (tx, rx) = mpsc::channel();
        let (show_id, check_id, toggle_id, quit_id) =
            (show.id().clone(), check.id().clone(), toggle.id().clone(), quit.id().clone());
        let menu_tx = tx.clone();
        let menu_ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let msg = match event.id {
                id if id == show_id => TrayMsg::Show,
                id if id == check_id => TrayMsg::CheckNow,
                id if id == toggle_id => TrayMsg::ToggleMonitoring,
                id if id == quit_id => TrayMsg::Quit,
                _ => return,
            };
            let _ = menu_tx.send(msg);
            menu_ctx.request_repaint();
        }));

        let icon_ctx = ctx.clone();
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            let open = matches!(
                event,
                TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
                    | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. }
            );
            if open {
                let _ = tx.send(TrayMsg::Show);
                icon_ctx.request_repaint();
            }
        }));

        Ok(Self { icon, toggle, rx, tooltip: String::new(), running: None })
    }

    pub fn poll(&self) -> Option<TrayMsg> {
        self.rx.try_recv().ok()
    }

    /// Keeps tooltip and menu text in sync with the monitor (cheap when unchanged).
    pub fn sync(&mut self, tooltip: String, running: bool) {
        if tooltip != self.tooltip {
            let _ = self.icon.set_tooltip(Some(&tooltip));
            self.tooltip = tooltip;
        }
        if self.running != Some(running) {
            self.toggle.set_text(if running { "Pause monitoring" } else { "Resume monitoring" });
            self.running = Some(running);
        }
    }
}

fn icon() -> Result<tray_icon::Icon, String> {
    let img = image::load_from_memory(include_bytes!("../../assets/icon-32.png"))
        .map_err(|e| e.to_string())?
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).map_err(|e| e.to_string())
}
