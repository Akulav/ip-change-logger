//! Update check against this app's GitHub releases.
//!
//! Every release carries exactly one file per OS plus `SHA256SUMS.txt`. An update is only offered
//! when the release has a checksum for this OS's file; downloading and applying is done by
//! [`super::download_update`] / [`super::apply_update`].

use serde::Deserialize;

pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/Akulav/ip-change-logger/releases/latest";
pub const ASSET_WINDOWS: &str = "IPTrail-setup.exe";
pub const ASSET_LINUX: &str = "IPTrail-linux-x64";
pub const ASSET_MACOS: &str = "IPTrail-macos-arm64";
pub const ASSET_SUMS: &str = "SHA256SUMS.txt";

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateInfo {
    pub version: String,
    pub url: String,
    pub sha256: String,
}

pub fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_secs(120)))
        .user_agent(format!("{}/{}", super::APP_ID, super::CURRENT_VERSION))
        .build()
        .into()
}

/// The release file for this OS, or `None` where no build is published.
pub fn this_os_asset() -> Option<&'static str> {
    if cfg!(all(windows, target_arch = "x86_64")) {
        Some(ASSET_WINDOWS)
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(ASSET_LINUX)
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(ASSET_MACOS)
    } else {
        None
    }
}

/// Compare dotted versions numerically ("10.0.0" > "9.0.0"). Non-numeric parts compare as 0.
pub fn is_newer(remote: &str, current: &str) -> bool {
    let parse = |v: &str| -> Vec<u64> {
        v.trim_start_matches(['v', 'V']).split(['.', '-', '+']).map(|p| p.parse().unwrap_or(0)).collect()
    };
    let (a, b) = (parse(remote), parse(current));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).copied().unwrap_or(0), b.get(i).copied().unwrap_or(0));
        if x != y {
            return x > y;
        }
    }
    false
}

/// Find `name` in a `SHA256SUMS.txt` (`<hex>  <name>` or `<hex> *<name>`).
pub fn find_sum(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut it = l.split_whitespace();
        let hash = it.next()?;
        let file = it.next()?.trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

/// (version, file url, sums url) of `release` for `asset`, or `None` when the release isn't usable.
fn pick(release: &GhRelease, asset: &str) -> Option<(String, String, String)> {
    if release.draft || release.prerelease {
        return None;
    }
    let https = |a: &&GhAsset| a.browser_download_url.starts_with("https://");
    let file = release.assets.iter().filter(https).find(|a| a.name == asset)?;
    let sums = release.assets.iter().filter(https).find(|a| a.name == ASSET_SUMS)?;
    let version = release.tag_name.trim_start_matches(['v', 'V']).to_string();
    Some((version, file.browser_download_url.clone(), sums.browser_download_url.clone()))
}

/// Newer release for this OS, if any. Blocking: call it off the UI thread.
pub fn check() -> Result<Option<UpdateInfo>, String> {
    let Some(asset) = this_os_asset() else { return Ok(None) };
    let agent = agent();
    let text = match agent.get(LATEST_RELEASE_API).header("Accept", "application/vnd.github+json").call() {
        Ok(r) => r.into_body().read_to_string().map_err(|e| e.to_string())?,
        // No release published yet.
        Err(ureq::Error::StatusCode(404)) => return Ok(None),
        Err(e) => return Err(format!("could not reach GitHub: {e}")),
    };
    let release: GhRelease =
        serde_json::from_str(&text).map_err(|e| format!("unexpected reply from GitHub: {e}"))?;
    let Some((version, url, sums_url)) = pick(&release, asset) else { return Ok(None) };
    if !is_newer(&version, super::CURRENT_VERSION) {
        return Ok(None);
    }
    let sums = agent
        .get(&sums_url)
        .call()
        .map_err(|e| format!("could not download {ASSET_SUMS}: {e}"))?
        .into_body()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    let sha256 = find_sum(&sums, asset).ok_or(format!("release {version} has no checksum for {asset}"))?;
    Ok(Some(UpdateInfo { version, url, sha256 }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ordering() {
        assert!(is_newer("3.0.1", "3.0.0"));
        assert!(is_newer("3.1", "3.0.9"));
        assert!(!is_newer("2.5.1", "3.0.0"));
        assert!(!is_newer("3.0.0", "3.0.0"));
        assert!(is_newer("v4", "3.5.5"));
    }

    #[test]
    fn sums_and_assets() {
        let a = "a".repeat(64);
        let s = format!("{a}  {ASSET_WINDOWS}\n{} *{ASSET_LINUX}\n", "B".repeat(64));
        assert_eq!(find_sum(&s, ASSET_WINDOWS), Some(a));
        assert_eq!(find_sum(&s, ASSET_LINUX), Some("b".repeat(64)));
        assert_eq!(find_sum(&s, ASSET_MACOS), None);
        let json = format!(
            r#"{{"tag_name":"v3.1.0","assets":[
                {{"name":"{ASSET_WINDOWS}","browser_download_url":"https://x/setup.exe"}},
                {{"name":"{ASSET_SUMS}","browser_download_url":"https://x/sums"}}]}}"#
        );
        let r: GhRelease = serde_json::from_str(&json).unwrap();
        let (v, file, sums) = pick(&r, ASSET_WINDOWS).unwrap();
        assert_eq!(
            (v.as_str(), file.as_str(), sums.as_str()),
            ("3.1.0", "https://x/setup.exe", "https://x/sums")
        );
        assert!(pick(&r, ASSET_LINUX).is_none());
        let plain: GhRelease =
            serde_json::from_str(&json.replace("https://x/setup.exe", "http://x/setup.exe")).unwrap();
        assert!(pick(&plain, ASSET_WINDOWS).is_none(), "no plain-HTTP downloads");
    }
}
