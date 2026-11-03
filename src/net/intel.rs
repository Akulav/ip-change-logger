//! IP intelligence: location, network owner and anonymity signals.
//!
//! Replaces the ipdata.co integration of IPTrail 1.x with free services that
//! need no API key:
//!
//! * [ip-api.com](https://ip-api.com) — geolocation, ISP/ASN, reverse DNS and
//!   `proxy` / `hosting` / `mobile` flags (free tier is HTTP-only, 45 req/min).
//! * [ipwho.is](https://ipwho.is) — HTTPS fallback for location and network.
//!
//! Tor / attacker / abuser flags come from [`super::feeds`].

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct IpIntel {
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub timezone: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub asn: Option<String>,
    pub reverse_dns: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,

    /// Known proxy, VPN or Tor exit (ip-api.com).
    pub proxy: Option<bool>,
    /// Hosting provider / data center range (ip-api.com).
    pub hosting: Option<bool>,
    /// Cellular network (ip-api.com).
    pub mobile: Option<bool>,
    /// Listed as a Tor exit node (Tor Project).
    pub tor: Option<bool>,
    /// Reported for attacks in the last 48 hours (blocklist.de).
    pub attacker: Option<bool>,
    /// On a curated abuse / hijacked-network list (FireHOL level 1).
    pub abuser: Option<bool>,

    /// Which service produced the location data.
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    Unknown,
    Clean,
    /// Anonymised (proxy / VPN / Tor / data center) but not on an abuse list.
    Masked,
    Threat,
}

impl IpIntel {
    /// Traffic from this address is anonymised (Tor or proxy/VPN).
    pub fn anonymous(&self) -> Option<bool> {
        any_known(&[self.tor, self.proxy])
    }

    /// Address appears on an attack or abuse list.
    pub fn threat(&self) -> Option<bool> {
        any_known(&[self.attacker, self.abuser])
    }

    pub fn risk(&self) -> Risk {
        match (self.threat(), self.anonymous(), self.hosting) {
            (Some(true), _, _) => Risk::Threat,
            (_, Some(true), _) | (_, _, Some(true)) => Risk::Masked,
            (None, None, None) => Risk::Unknown,
            _ => Risk::Clean,
        }
    }

    /// "City, Region, Country" with empty parts skipped.
    pub fn place(&self) -> Option<String> {
        let mut parts: Vec<&str> = Vec::new();
        for p in [&self.city, &self.region, &self.country].into_iter().flatten() {
            let p = p.trim();
            if !p.is_empty() && !parts.contains(&p) {
                parts.push(p);
            }
        }
        (!parts.is_empty()).then(|| parts.join(", "))
    }

    /// Best short description of who operates the address.
    pub fn network(&self) -> Option<String> {
        let name = self.isp.as_deref().or(self.org.as_deref())?.trim();
        if name.is_empty() {
            return None;
        }
        Some(match self.asn.as_deref().and_then(|a| a.split_whitespace().next()) {
            Some(asn) => format!("{name} ({asn})"),
            None => name.to_string(),
        })
    }

    /// Short labels for every positive flag, for logs and notifications.
    pub fn flag_labels(&self) -> Vec<&'static str> {
        [
            (self.proxy, "proxy/VPN"),
            (self.tor, "Tor"),
            (self.hosting, "data center"),
            (self.mobile, "mobile"),
            (self.attacker, "attacker"),
            (self.abuser, "abuser"),
        ]
        .into_iter()
        .filter_map(|(v, label)| (v == Some(true)).then_some(label))
        .collect()
    }
}

fn any_known(values: &[Option<bool>]) -> Option<bool> {
    if values.contains(&Some(true)) {
        Some(true)
    } else if values.iter().any(Option::is_some) {
        Some(false)
    } else {
        None
    }
}

pub fn lookup(agent: &ureq::Agent, ip: IpAddr) -> Result<IpIntel, String> {
    match ip_api(agent, ip) {
        Ok(intel) => Ok(intel),
        Err(primary) => {
            ipwhois(agent, ip).map_err(|fallback| format!("ip-api.com: {primary}; ipwho.is: {fallback}"))
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IpApiResponse {
    status: String,
    message: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    region_name: Option<String>,
    city: Option<String>,
    timezone: Option<String>,
    isp: Option<String>,
    org: Option<String>,
    #[serde(rename = "as")]
    asn: Option<String>,
    reverse: Option<String>,
    lat: Option<f64>,
    lon: Option<f64>,
    mobile: Option<bool>,
    proxy: Option<bool>,
    hosting: Option<bool>,
}

fn ip_api(agent: &ureq::Agent, ip: IpAddr) -> Result<IpIntel, String> {
    const FIELDS: &str = "status,message,country,countryCode,regionName,city,timezone,isp,org,as,reverse,lat,lon,mobile,proxy,hosting";
    let url = format!("http://ip-api.com/json/{ip}?fields={FIELDS}");
    let r: IpApiResponse = agent
        .get(&url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    if r.status != "success" {
        return Err(r.message.unwrap_or_else(|| "lookup failed".into()));
    }
    Ok(IpIntel {
        country: non_empty(r.country),
        country_code: non_empty(r.country_code),
        region: non_empty(r.region_name),
        city: non_empty(r.city),
        timezone: non_empty(r.timezone),
        isp: non_empty(r.isp),
        org: non_empty(r.org),
        asn: non_empty(r.asn),
        reverse_dns: non_empty(r.reverse),
        latitude: r.lat,
        longitude: r.lon,
        proxy: r.proxy,
        hosting: r.hosting,
        mobile: r.mobile,
        source: "ip-api.com".into(),
        ..Default::default()
    })
}

#[derive(Deserialize)]
struct IpWhoIsResponse {
    success: bool,
    message: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    region: Option<String>,
    city: Option<String>,
    latitude: Option<f64>,
    longitude: Option<f64>,
    connection: Option<IpWhoIsConnection>,
    timezone: Option<IpWhoIsTimezone>,
}

#[derive(Deserialize)]
struct IpWhoIsConnection {
    asn: Option<u64>,
    org: Option<String>,
    isp: Option<String>,
}

#[derive(Deserialize)]
struct IpWhoIsTimezone {
    id: Option<String>,
}

fn ipwhois(agent: &ureq::Agent, ip: IpAddr) -> Result<IpIntel, String> {
    let r: IpWhoIsResponse = agent
        .get(&format!("https://ipwho.is/{ip}"))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    if !r.success {
        return Err(r.message.unwrap_or_else(|| "lookup failed".into()));
    }
    let conn = r.connection;
    Ok(IpIntel {
        country: non_empty(r.country),
        country_code: non_empty(r.country_code),
        region: non_empty(r.region),
        city: non_empty(r.city),
        timezone: r.timezone.and_then(|t| non_empty(t.id)),
        isp: conn.as_ref().and_then(|c| non_empty(c.isp.clone())),
        org: conn.as_ref().and_then(|c| non_empty(c.org.clone())),
        asn: conn.as_ref().and_then(|c| c.asn).map(|n| format!("AS{n}")),
        latitude: r.latitude,
        longitude: r.longitude,
        source: "ipwho.is".into(),
        ..Default::default()
    })
}

fn non_empty(s: Option<String>) -> Option<String> {
    s.filter(|s| !s.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_levels() {
        let mut i = IpIntel::default();
        assert_eq!(i.risk(), Risk::Unknown);
        i.proxy = Some(false);
        i.tor = Some(false);
        i.hosting = Some(false);
        assert_eq!(i.risk(), Risk::Clean);
        i.hosting = Some(true);
        assert_eq!(i.risk(), Risk::Masked);
        i.abuser = Some(true);
        assert_eq!(i.risk(), Risk::Threat);
        assert_eq!(i.flag_labels(), vec!["data center", "abuser"]);
    }

    #[test]
    fn place_skips_duplicates_and_blanks() {
        let i = IpIntel {
            city: Some("Singapore".into()),
            region: Some(" ".into()),
            country: Some("Singapore".into()),
            ..Default::default()
        };
        assert_eq!(i.place().as_deref(), Some("Singapore"));
    }

    #[test]
    fn parses_ip_api_payload() {
        let r: IpApiResponse = serde_json::from_str(
            r#"{"status":"success","country":"Australia","countryCode":"AU","regionName":"Queensland",
                "city":"South Brisbane","timezone":"Australia/Brisbane","isp":"Cloudflare, Inc","org":"APNIC",
                "as":"AS13335 Cloudflare, Inc.","reverse":"one.one.one.one","lat":-27.4,"lon":153.0,
                "mobile":false,"proxy":false,"hosting":true}"#,
        )
        .unwrap();
        assert_eq!(r.asn.as_deref(), Some("AS13335 Cloudflare, Inc."));
        assert_eq!(r.hosting, Some(true));
    }
}
