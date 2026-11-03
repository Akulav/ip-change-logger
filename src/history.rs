//! Persistent record of IP changes.
//!
//! Two files are written:
//! * `history.jsonl` — one JSON object per change, used to restore the UI.
//! * `ip-changes.log` — a human-readable line per change (the successor of the
//!   1.x `ip.txt` on the desktop).

use crate::net::intel::IpIntel;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub at: DateTime<Local>,
    pub ip: IpAddr,
    /// The address seen before this one, if any.
    pub previous: Option<IpAddr>,
    pub intel: Option<IpIntel>,
}

impl Entry {
    /// One line for the plain-text log.
    pub fn log_line(&self) -> String {
        let mut line = format!("{}  {:<15}", self.at.format("%Y-%m-%d %H:%M:%S %:z"), self.ip);
        match self.previous {
            Some(prev) => line.push_str(&format!("  (was {prev})")),
            None => line.push_str("  (first seen)"),
        }
        if let Some(intel) = &self.intel {
            if let Some(place) = intel.place() {
                line.push_str(&format!("  {place}"));
            }
            if let Some(net) = intel.network() {
                line.push_str(&format!("  · {net}"));
            }
            let flags = intel.flag_labels();
            if !flags.is_empty() {
                line.push_str(&format!("  [{}]", flags.join(", ")));
            }
        }
        line
    }
}

pub struct History {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl History {
    pub fn load(path: PathBuf) -> Self {
        let entries = std::fs::read_to_string(&path)
            .map(|text| {
                text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
                    .collect()
            })
            .unwrap_or_default();
        Self { path, entries }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn last(&self) -> Option<&Entry> {
        self.entries.last()
    }

    pub fn push(&mut self, entry: Entry) -> std::io::Result<()> {
        let line = serde_json::to_string(&entry).map_err(std::io::Error::other)?;
        self.entries.push(entry);
        append_line(&self.path, &line)
    }

    /// Updates the intel of the newest entry (used when a lookup succeeds late).
    pub fn update_last_intel(&mut self, intel: IpIntel) -> std::io::Result<()> {
        if let Some(last) = self.entries.last_mut() {
            last.intel = Some(intel);
            self.rewrite()?;
        }
        Ok(())
    }

    pub fn clear(&mut self) -> std::io::Result<()> {
        self.entries.clear();
        match std::fs::remove_file(&self.path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }

    fn rewrite(&self) -> std::io::Result<()> {
        let mut out = String::new();
        for e in &self.entries {
            out.push_str(&serde_json::to_string(e).map_err(std::io::Error::other)?);
            out.push('\n');
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("jsonl.tmp");
        std::fs::write(&tmp, out)?;
        std::fs::rename(tmp, &self.path)
    }
}

pub fn append_line(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(f, "{line}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_disk() {
        let dir = std::env::temp_dir().join(format!("iptrail-test-{}", std::process::id()));
        let path = dir.join("history.jsonl");
        let _ = std::fs::remove_dir_all(&dir);

        let mut h = History::load(path.clone());
        assert!(h.entries().is_empty());
        h.push(Entry { at: Local::now(), ip: "198.51.100.1".parse().unwrap(), previous: None, intel: None })
            .unwrap();
        h.push(Entry {
            at: Local::now(),
            ip: "203.0.113.9".parse().unwrap(),
            previous: Some("198.51.100.1".parse().unwrap()),
            intel: None,
        })
        .unwrap();
        h.update_last_intel(IpIntel { city: Some("Oslo".into()), ..Default::default() }).unwrap();

        let h2 = History::load(path);
        assert_eq!(h2.entries().len(), 2);
        assert_eq!(h2.last().unwrap().intel.as_ref().unwrap().city.as_deref(), Some("Oslo"));
        assert!(h2.last().unwrap().log_line().contains("(was 198.51.100.1)"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
