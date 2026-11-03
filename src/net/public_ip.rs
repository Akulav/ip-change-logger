//! Discovering the machine's public IPv4 address.
//!
//! Several independent "what is my IP" services are tried in order, so one
//! service being down or rate limited never looks like an IP change.

use std::net::IpAddr;

/// Plain-text IPv4 echo services. All are free, keyless and HTTPS.
pub const PROVIDERS: &[(&str, &str)] = &[
    ("ipify", "https://api.ipify.org"),
    ("icanhazip", "https://ipv4.icanhazip.com"),
    ("AWS checkip", "https://checkip.amazonaws.com"),
    ("ident.me", "https://v4.ident.me"),
];

#[derive(Debug, Clone)]
pub struct Lookup {
    pub ip: IpAddr,
    pub provider: &'static str,
}

pub fn fetch(agent: &ureq::Agent) -> Result<Lookup, String> {
    let mut errors = Vec::new();
    for &(name, url) in PROVIDERS {
        match fetch_one(agent, url) {
            Ok(ip) => return Ok(Lookup { ip, provider: name }),
            Err(e) => errors.push(format!("{name}: {e}")),
        }
    }
    Err(format!("all IP providers failed ({})", errors.join("; ")))
}

fn fetch_one(agent: &ureq::Agent, url: &str) -> Result<IpAddr, String> {
    let body = agent
        .get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .with_config()
        .limit(1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse(&body)
}

fn parse(body: &str) -> Result<IpAddr, String> {
    body.trim().parse::<IpAddr>().map_err(|_| format!("unexpected response {:?}", truncate(body.trim(), 40)))
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((i, _)) => &s[..i],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_responses() {
        assert_eq!(parse("203.0.113.7\n").unwrap().to_string(), "203.0.113.7");
        assert!(parse("<html>nope</html>").is_err());
        assert!(parse("").is_err());
    }
}
