//! Public threat feeds, downloaded on demand and cached on disk.
//!
//! These take the place of ipdata.co's paid threat API:
//!
//! | Flag       | Feed                                                         | Refresh |
//! |------------|--------------------------------------------------------------|---------|
//! | `tor`      | Tor Project bulk exit list                                   | 1 h     |
//! | `attacker` | blocklist.de — IPs reported for attacks in the last 48 hours | 6 h     |
//! | `abuser`   | FireHOL level 1 — Spamhaus DROP, DShield, Feodo, …           | 12 h    |

use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

struct FeedSpec {
    file: &'static str,
    url: &'static str,
    ttl: Duration,
}

const TOR: FeedSpec = FeedSpec {
    file: "tor-exits.txt",
    url: "https://check.torproject.org/torbulkexitlist",
    ttl: Duration::from_secs(60 * 60),
};
const ATTACKERS: FeedSpec = FeedSpec {
    file: "blocklist-de.txt",
    url: "https://lists.blocklist.de/lists/all.txt",
    ttl: Duration::from_secs(6 * 60 * 60),
};
const ABUSERS: FeedSpec = FeedSpec {
    file: "firehol-level1.netset",
    url: "https://raw.githubusercontent.com/firehol/blocklist-ipsets/master/firehol_level1.netset",
    ttl: Duration::from_secs(12 * 60 * 60),
};

/// A set of single addresses plus IPv4 CIDR ranges.
#[derive(Default)]
pub struct IpSet {
    singles: HashSet<IpAddr>,
    v4_nets: Vec<(u32, u32)>, // (network, mask)
}

impl IpSet {
    pub fn parse(text: &str) -> Self {
        let mut set = IpSet::default();
        for line in text.lines() {
            let line = line.split(['#', ';']).next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            if let Some((addr, len)) = line.split_once('/') {
                if let (Ok(addr), Ok(len)) = (addr.parse::<Ipv4Addr>(), len.parse::<u32>()) {
                    if len <= 32 {
                        let mask = if len == 0 { 0 } else { u32::MAX << (32 - len) };
                        set.v4_nets.push((u32::from(addr) & mask, mask));
                    }
                }
            } else if let Ok(ip) = line.parse::<IpAddr>() {
                set.singles.insert(ip);
            }
        }
        set
    }

    pub fn contains(&self, ip: IpAddr) -> bool {
        if self.singles.contains(&ip) {
            return true;
        }
        match ip {
            IpAddr::V4(v4) => {
                let n = u32::from(v4);
                self.v4_nets.iter().any(|&(net, mask)| n & mask == net)
            }
            IpAddr::V6(_) => false,
        }
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.singles.len() + self.v4_nets.len()
    }
}

struct Feed {
    spec: &'static FeedSpec,
    set: Option<IpSet>,
    fetched_at: Option<SystemTime>,
}

impl Feed {
    fn new(spec: &'static FeedSpec) -> Self {
        Self { spec, set: None, fetched_at: None }
    }

    fn is_fresh(&self) -> bool {
        self.fetched_at.and_then(|t| t.elapsed().ok()).is_some_and(|age| age < self.spec.ttl)
    }

    /// Makes sure the feed is loaded and not older than its TTL. Falls back to
    /// a stale copy (memory or disk) when the network is unavailable.
    fn refresh(&mut self, agent: &ureq::Agent, cache_dir: &Path) -> Result<(), String> {
        if self.is_fresh() {
            return Ok(());
        }
        let path = cache_dir.join(self.spec.file);

        // A recent copy on disk saves a download after restarting the app.
        if let Ok(modified) = std::fs::metadata(&path).and_then(|m| m.modified()) {
            let fresh = modified.elapsed().is_ok_and(|age| age < self.spec.ttl);
            if fresh || self.set.is_none() {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    self.set = Some(IpSet::parse(&text));
                    self.fetched_at = Some(modified);
                    if fresh {
                        return Ok(());
                    }
                }
            }
        }

        match download(agent, self.spec.url) {
            Ok(text) => {
                let _ = std::fs::create_dir_all(cache_dir);
                let _ = std::fs::write(&path, &text);
                self.set = Some(IpSet::parse(&text));
                self.fetched_at = Some(SystemTime::now());
                Ok(())
            }
            Err(e) if self.set.is_some() => Err(format!("{} is stale: {e}", self.spec.file)),
            Err(e) => Err(format!("{} unavailable: {e}", self.spec.file)),
        }
    }

    fn check(&self, ip: IpAddr) -> Option<bool> {
        self.set.as_ref().map(|s| s.contains(ip))
    }
}

fn download(agent: &ureq::Agent, url: &str) -> Result<String, String> {
    agent
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(16 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FeedVerdict {
    pub tor: Option<bool>,
    pub attacker: Option<bool>,
    pub abuser: Option<bool>,
}

pub struct ThreatFeeds {
    cache_dir: PathBuf,
    tor: Feed,
    attackers: Feed,
    abusers: Feed,
}

impl ThreatFeeds {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            tor: Feed::new(&TOR),
            attackers: Feed::new(&ATTACKERS),
            abusers: Feed::new(&ABUSERS),
        }
    }

    /// Checks `ip` against every feed. Returns the verdict plus any non-fatal
    /// problems (a feed that could not be refreshed is reported, not fatal).
    pub fn check(&mut self, agent: &ureq::Agent, ip: IpAddr) -> (FeedVerdict, Vec<String>) {
        let mut problems = Vec::new();
        for feed in [&mut self.tor, &mut self.attackers, &mut self.abusers] {
            if let Err(e) = feed.refresh(agent, &self.cache_dir) {
                problems.push(e);
            }
        }
        let verdict = FeedVerdict {
            tor: self.tor.check(ip),
            attacker: self.attackers.check(ip),
            abuser: self.abusers.check(ip),
        };
        (verdict, problems)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_singles_and_cidrs() {
        let set = IpSet::parse("# comment\n1.2.3.4\n10.0.0.0/8 ; inline\n2001:db8::1\n\ngarbage\n");
        assert_eq!(set.len(), 3);
        assert!(set.contains("1.2.3.4".parse().unwrap()));
        assert!(set.contains("10.200.3.4".parse().unwrap()));
        assert!(!set.contains("11.0.0.1".parse().unwrap()));
        assert!(set.contains("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn zero_prefix_matches_everything() {
        let set = IpSet::parse("0.0.0.0/0");
        assert!(set.contains("8.8.8.8".parse().unwrap()));
    }
}
