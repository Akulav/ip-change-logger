//! `--demo`: sample data for screenshots, using RFC 5737 documentation
//! addresses so no real IP ever ends up in the docs.

use crate::config::{Paths, Settings};
use crate::history::Entry;
use crate::net::intel::IpIntel;
use chrono::{Duration, Local};

pub fn paths() -> Result<Paths, String> {
    let dir = std::env::temp_dir().join(format!("iptrail-demo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let paths = Paths::in_dir(&dir);

    let now = Local::now();
    let home = || IpIntel {
        country: Some("Netherlands".into()),
        country_code: Some("NL".into()),
        region: Some("North Holland".into()),
        city: Some("Amsterdam".into()),
        timezone: Some("Europe/Amsterdam".into()),
        isp: Some("Example Broadband B.V.".into()),
        org: Some("Example Broadband".into()),
        asn: Some("AS64500 Example Broadband B.V.".into()),
        reverse_dns: Some("cpe-198-51-100-24.example.net".into()),
        latitude: Some(52.3740),
        longitude: Some(4.8897),
        proxy: Some(false),
        hosting: Some(false),
        mobile: Some(false),
        tor: Some(false),
        attacker: Some(false),
        abuser: Some(false),
        source: "ip-api.com".into(),
    };
    let vpn = IpIntel {
        country: Some("Switzerland".into()),
        country_code: Some("CH".into()),
        region: Some("Zurich".into()),
        city: Some("Zurich".into()),
        timezone: Some("Europe/Zurich".into()),
        isp: Some("Example VPN AG".into()),
        org: Some("Example VPN".into()),
        asn: Some("AS64511 Example VPN AG".into()),
        reverse_dns: Some("node-17.vpn.example.com".into()),
        latitude: Some(47.3769),
        longitude: Some(8.5417),
        proxy: Some(true),
        hosting: Some(true),
        mobile: Some(false),
        tor: Some(false),
        attacker: Some(false),
        abuser: Some(false),
        source: "ip-api.com".into(),
    };
    let entries = [
        (Duration::days(2) + Duration::hours(3), "198.51.100.24", None, Some(home())),
        (Duration::hours(26), "203.0.113.77", Some("198.51.100.24"), Some(vpn.clone())),
        (Duration::hours(25), "198.51.100.24", Some("203.0.113.77"), Some(home())),
        (Duration::minutes(42), "203.0.113.18", Some("198.51.100.24"), Some(vpn)),
    ];

    let mut lines = String::new();
    for (ago, ip, prev, intel) in entries {
        let entry = Entry {
            at: now - ago,
            ip: ip.parse().expect("valid demo ip"),
            previous: prev.map(|p| p.parse().expect("valid demo ip")),
            intel,
        };
        lines.push_str(&serde_json::to_string(&entry).map_err(|e| e.to_string())?);
        lines.push('\n');
    }
    std::fs::write(&paths.history, lines).map_err(|e| e.to_string())?;
    Ok(paths)
}

/// Demo mode never touches the network.
pub fn settings() -> Settings {
    Settings {
        auto_start: false,
        notifications: false,
        ip_intel: false,
        threat_feeds: false,
        ..Default::default()
    }
}
