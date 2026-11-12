//! User settings and on-disk locations.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MIN_INTERVAL_SECS: u64 = 5;
pub const MAX_INTERVAL_SECS: u64 = 3600;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Seconds between public IP checks.
    pub interval_secs: u64,
    /// Start monitoring as soon as the app launches.
    pub auto_start: bool,
    /// Show a desktop notification when the IP changes.
    pub notifications: bool,
    /// Look up location / network / proxy info for each new IP.
    pub ip_intel: bool,
    /// Check each new IP against public Tor and abuse blocklists.
    pub threat_feeds: bool,
    /// Hide to the system tray when minimized (Windows).
    pub minimize_to_tray: bool,
    /// Hide to the system tray instead of quitting when the window is closed (Windows).
    pub close_to_tray: bool,
    /// Custom location of the human-readable change log. `None` uses the data directory.
    pub log_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_secs: 15,
            auto_start: true,
            notifications: true,
            ip_intel: true,
            threat_feeds: true,
            minimize_to_tray: true,
            close_to_tray: false,
            log_path: None,
        }
    }
}

impl Settings {
    pub fn load(paths: &Paths) -> Self {
        std::fs::read_to_string(&paths.settings)
            .ok()
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .map(Settings::sanitized)
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> std::io::Result<()> {
        if let Some(dir) = paths.settings.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&paths.settings, json)
    }

    pub fn sanitized(mut self) -> Self {
        self.interval_secs = self.interval_secs.clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS);
        if self.log_path.as_ref().is_some_and(|p| p.as_os_str().is_empty()) {
            self.log_path = None;
        }
        self
    }

    pub fn log_file(&self, paths: &Paths) -> PathBuf {
        self.log_path.clone().unwrap_or_else(|| paths.default_log.clone())
    }
}

/// Every file IPTrail reads or writes.
#[derive(Debug, Clone)]
pub struct Paths {
    pub settings: PathBuf,
    pub history: PathBuf,
    pub default_log: PathBuf,
    pub cache_dir: PathBuf,
}

impl Paths {
    /// Everything lives in the folder IPTrail is installed in (next to the program when not
    /// installed); never in the OS's config / data / cache folders. See `crate::install`.
    pub fn discover() -> Self {
        Self::in_dir(&crate::install::data_root())
    }

    /// Keeps every file inside one directory (used by `--demo`).
    pub fn in_dir(base: &Path) -> Self {
        Self::new(base, base, &base.join("cache"))
    }

    fn new(config: &Path, data: &Path, cache: &Path) -> Self {
        Self {
            settings: config.join("settings.json"),
            history: data.join("history.jsonl"),
            default_log: data.join("ip-changes.log"),
            cache_dir: cache.to_path_buf(),
        }
    }

    pub fn data_dir(&self) -> &Path {
        self.history.parent().unwrap_or(Path::new("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_is_clamped() {
        let s = Settings { interval_secs: 1, ..Default::default() }.sanitized();
        assert_eq!(s.interval_secs, MIN_INTERVAL_SECS);
        let s = Settings { interval_secs: 99_999, ..Default::default() }.sanitized();
        assert_eq!(s.interval_secs, MAX_INTERVAL_SECS);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{ "interval_secs": 42 }"#).unwrap();
        assert_eq!(s.interval_secs, 42);
        assert!(s.notifications);
    }
}
