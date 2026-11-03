//! Background worker that polls the public IP and records changes.
//!
//! The worker owns all network I/O. The UI (or the headless runner) talks to it
//! through [`Command`]s and reads the shared [`State`].

use crate::config::{Paths, Settings};
use crate::history::{self, Entry, History};
use crate::net::{self, feeds::ThreatFeeds, intel::IpIntel, public_ip};
use chrono::{DateTime, Local};
use std::net::IpAddr;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

pub enum Command {
    Start,
    Pause,
    CheckNow,
    UpdateSettings(Settings),
    ClearHistory,
    Shutdown,
}

/// What the worker reports to whoever is listening.
pub enum Event {
    /// Something in [`State`] changed (repaint).
    Updated,
    /// A new address was recorded.
    Changed(Box<Entry>),
    /// A check failed.
    Error(String),
}

pub struct State {
    pub running: bool,
    pub checking: bool,
    pub current: Option<IpAddr>,
    pub intel: Option<IpIntel>,
    pub previous: Option<IpAddr>,
    pub last_change: Option<DateTime<Local>>,
    pub last_check: Option<DateTime<Local>>,
    pub next_check: Option<Instant>,
    pub provider: Option<&'static str>,
    pub last_error: Option<String>,
    pub warnings: Vec<String>,
    pub checks: u64,
    pub history: History,
}

#[derive(Clone)]
pub struct Monitor {
    tx: Sender<Command>,
    state: Arc<Mutex<State>>,
}

impl Monitor {
    pub fn spawn(paths: Paths, settings: Settings, on_event: impl Fn(Event) + Send + 'static) -> Self {
        let history = History::load(paths.history.clone());
        let last = history.last().cloned();
        let state = Arc::new(Mutex::new(State {
            running: settings.auto_start,
            checking: false,
            current: last.as_ref().map(|e| e.ip),
            intel: last.as_ref().and_then(|e| e.intel.clone()),
            previous: last.as_ref().and_then(|e| e.previous),
            last_change: last.as_ref().map(|e| e.at),
            last_check: None,
            next_check: None,
            provider: None,
            last_error: None,
            warnings: Vec::new(),
            checks: 0,
            history,
        }));

        let (tx, rx) = mpsc::channel();
        let worker = Worker {
            state: state.clone(),
            settings,
            paths,
            agent: net::agent(),
            feeds: None,
            on_event: Box::new(on_event),
        };
        std::thread::Builder::new()
            .name("iptrail-monitor".into())
            .spawn(move || worker.run(rx))
            .expect("spawn monitor thread");

        Self { tx, state }
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }

    pub fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

struct Worker {
    state: Arc<Mutex<State>>,
    settings: Settings,
    paths: Paths,
    agent: ureq::Agent,
    feeds: Option<ThreatFeeds>,
    on_event: Box<dyn Fn(Event) + Send>,
}

impl Worker {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn run(mut self, rx: mpsc::Receiver<Command>) {
        // Check right away when auto-starting.
        let mut due = self.state().running.then(Instant::now);

        loop {
            let wait = match due {
                Some(at) => at.saturating_duration_since(Instant::now()),
                None => Duration::from_secs(3600),
            };
            match rx.recv_timeout(wait) {
                Ok(Command::Start) => {
                    self.state().running = true;
                    due = Some(Instant::now());
                }
                Ok(Command::Pause) => {
                    let mut s = self.state();
                    s.running = false;
                    s.next_check = None;
                    drop(s);
                    due = None;
                    (self.on_event)(Event::Updated);
                }
                Ok(Command::CheckNow) => due = Some(Instant::now()),
                Ok(Command::UpdateSettings(new)) => {
                    let interval_changed = new.interval_secs != self.settings.interval_secs;
                    self.settings = new;
                    if interval_changed && self.state().running {
                        let next = Instant::now() + self.interval();
                        self.state().next_check = Some(next);
                        due = Some(next);
                    }
                    (self.on_event)(Event::Updated);
                }
                Ok(Command::ClearHistory) => {
                    let mut s = self.state();
                    if let Err(e) = s.history.clear() {
                        s.last_error = Some(format!("Could not clear history: {e}"));
                    }
                    drop(s);
                    (self.on_event)(Event::Updated);
                }
                Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {
                    self.check();
                    due = self.state().running.then(|| Instant::now() + self.interval());
                    self.state().next_check = due;
                    (self.on_event)(Event::Updated);
                }
            }
        }
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(self.settings.interval_secs)
    }

    fn check(&mut self) {
        self.state().checking = true;
        (self.on_event)(Event::Updated);

        let result = public_ip::fetch(&self.agent);
        let now = Local::now();

        let (ip, known, needs_intel) = {
            let mut s = self.state();
            s.checking = false;
            s.last_check = Some(now);
            s.checks += 1;
            match result {
                Ok(lookup) => {
                    s.provider = Some(lookup.provider);
                    s.last_error = None;
                    (lookup.ip, s.current, s.intel.is_none())
                }
                Err(e) => {
                    s.last_error = Some(e.clone());
                    drop(s);
                    (self.on_event)(Event::Error(e));
                    return;
                }
            }
        };

        if known == Some(ip) {
            // Same address. Retry intel if an earlier lookup failed.
            if needs_intel && self.wants_intel() {
                if let Some(intel) = self.gather_intel(ip) {
                    let mut s = self.state();
                    s.intel = Some(intel.clone());
                    if s.history.last().is_some_and(|e| e.ip == ip) {
                        let _ = s.history.update_last_intel(intel);
                    }
                }
            }
            return;
        }

        let intel = if self.wants_intel() { self.gather_intel(ip) } else { None };
        let entry = Entry { at: now, ip, previous: known, intel: intel.clone() };

        let log_file = self.settings.log_file(&self.paths);
        let mut s = self.state();
        s.previous = known;
        s.current = Some(ip);
        s.intel = intel;
        s.last_change = Some(now);
        let mut io_errors = Vec::new();
        if let Err(e) = s.history.push(entry.clone()) {
            io_errors.push(format!("history: {e}"));
        }
        if let Err(e) = history::append_line(&log_file, &entry.log_line()) {
            io_errors.push(format!("{}: {e}", log_file.display()));
        }
        if !io_errors.is_empty() {
            s.last_error = Some(format!("Could not write log ({})", io_errors.join("; ")));
        }
        drop(s);

        // The very first address ever seen is not a "change" worth a popup.
        if self.settings.notifications && known.is_some() {
            crate::notify::ip_changed(&entry);
        }
        (self.on_event)(Event::Changed(Box::new(entry)));
    }

    fn wants_intel(&self) -> bool {
        self.settings.ip_intel || self.settings.threat_feeds
    }

    /// Location lookup and threat-feed check. Returns `None` only if nothing at all could be learned.
    fn gather_intel(&mut self, ip: IpAddr) -> Option<IpIntel> {
        let mut warnings = Vec::new();

        let mut intel = if self.settings.ip_intel {
            match net::intel::lookup(&self.agent, ip) {
                Ok(i) => Some(i),
                Err(e) => {
                    warnings.push(format!("IP lookup failed: {e}"));
                    None
                }
            }
        } else {
            None
        };

        if self.settings.threat_feeds {
            let cache_dir = self.paths.cache_dir.clone();
            let feeds = self.feeds.get_or_insert_with(|| ThreatFeeds::new(cache_dir));
            let (verdict, problems) = feeds.check(&self.agent, ip);
            warnings.extend(problems);
            if verdict.tor.is_some() || verdict.attacker.is_some() || verdict.abuser.is_some() {
                let i = intel
                    .get_or_insert_with(|| IpIntel { source: "threat feeds".into(), ..Default::default() });
                i.tor = verdict.tor;
                i.attacker = verdict.attacker;
                i.abuser = verdict.abuser;
            }
        }

        self.state().warnings = warnings;
        intel
    }
}
