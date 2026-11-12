//! Where the app lives: installing, updating and removing it (same model as Akulav Launcher).
//!
//! **One file per OS does both install and update.**
//!
//! * **Windows** — `IPTrail-setup.exe` (Inno Setup, `installer/iptrail.iss`). The first run asks for
//!   the install folder; Inno remembers it, so a newer setup or the in-app update (the setup with
//!   `/UPDATE /DIR=<folder>`) installs into the same folder without asking.
//! * **Linux / macOS** — the bare binary. Started from anywhere, it offers to install itself into a
//!   folder the user picks (remembered in `<config dir>/iptrail/install-dir`); a newer download offers
//!   to update that folder. In-app updates swap the binary in place.
//!
//! **Files go there.** The install folder holds the program *and* everything it writes (settings,
//! history, change log, caches, a downloaded update). A [`MARKER`] file identifies an install
//! folder. Files left in the pre-3.1 locations (the OS's config / data folders) are moved in on the
//! first start of the installed copy, across drives too.

pub mod gui;
pub mod update;

use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------------------------
// This app
// ---------------------------------------------------------------------------------------------

pub const APP_NAME: &str = "IPTrail";
/// `.desktop` file, `~/.local/bin` command and the folder of the remembered install folder.
pub const APP_ID: &str = "iptrail";
/// The program inside the install folder.
#[cfg_attr(windows, allow(dead_code))]
pub const EXE_NAME: &str = if cfg!(windows) { "IPTrail.exe" } else { "iptrail" };
#[cfg_attr(windows, allow(dead_code))]
const BUNDLE_ID: &str = "io.github.akulav.iptrail";
#[cfg_attr(windows, allow(dead_code))]
const DESKTOP_KEYS: &str =
    "GenericName=Public IP monitor\nComment=Watch your public IP and log every change - made by Akulav\nCategories=Network;Utility;\nKeywords=ip;vpn;network;monitor;\n";
#[cfg_attr(windows, allow(dead_code))]
const ICON_PNG: &[u8] = include_bytes!("../../assets/icon-256.png");
/// Present in every install folder; holds the installed version.
pub const MARKER: &str = ".iptrail-install";
/// Everything the app keeps in its folder. Deleting data removes these and nothing else.
pub const DATA_ENTRIES: &[&str] = &["cache", "update", "history.jsonl", "ip-changes.log", "settings.json"];
/// `IPTRAIL_DATA_DIR` overrides the data folder, `IPTRAIL_PORTABLE` keeps data next to the program.
const ENV_DATA_DIR: &str = "IPTRAIL_DATA_DIR";
const ENV_PORTABLE: &str = "IPTRAIL_PORTABLE";

/// Files of versions before 3.1, in the OS's config / data folders: (file, name in the install folder).
fn legacy_files() -> Vec<(PathBuf, &'static str)> {
    let Some(d) = directories::ProjectDirs::from("io.github", "Akulav", "IPTrail") else { return Vec::new() };
    vec![
        (d.config_dir().join("settings.json"), "settings.json"),
        (d.data_dir().join("history.jsonl"), "history.jsonl"),
        (d.data_dir().join("ip-changes.log"), "ip-changes.log"),
    ]
}

/// Folders of versions before 3.1 that are removed once empty (and with "delete data").
fn legacy_dirs() -> Vec<PathBuf> {
    let Some(d) = directories::ProjectDirs::from("io.github", "Akulav", "IPTrail") else { return Vec::new() };
    vec![d.cache_dir().to_path_buf(), d.data_dir().to_path_buf(), d.config_dir().to_path_buf()]
}

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const NEW_SUFFIX: &str = ".new";
const OLD_SUFFIX: &str = ".old";

fn home_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.home_dir().to_path_buf())
}

#[cfg_attr(not(unix), allow(dead_code))]
fn config_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.config_dir().to_path_buf())
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn data_local_dir() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|b| b.data_local_dir().to_path_buf())
}

// ---------------------------------------------------------------------------------------------
// Locations
// ---------------------------------------------------------------------------------------------

/// `IPTrail.exe` → `IPTrail.old.exe`
pub(crate) fn sibling(exe: &Path, suffix: &str) -> PathBuf {
    let stem = exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| APP_ID.into());
    let name = match exe.extension() {
        Some(ext) => format!("{stem}{suffix}.{}", ext.to_string_lossy()),
        None => format!("{stem}{suffix}"),
    };
    exe.with_file_name(name)
}

/// Folder a program belongs to: its own folder, or for `X.app/Contents/MacOS/bin` the folder that
/// holds `X.app`.
pub fn root_of_exe(exe: &Path) -> Option<PathBuf> {
    let dir = exe.parent()?;
    let bundle = dir
        .parent()
        .filter(|_| dir.file_name().is_some_and(|n| n == "MacOS"))
        .filter(|contents| contents.file_name().is_some_and(|n| n == "Contents"))
        .and_then(Path::parent)
        .filter(|b| b.extension().is_some_and(|e| e == "app"));
    match bundle {
        Some(b) => b.parent().map(Path::to_path_buf),
        None => Some(dir.to_path_buf()),
    }
}

pub fn program_root() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| root_of_exe(&e))
}

pub fn is_install_root(dir: &Path) -> bool {
    dir.join(MARKER).is_file()
}

pub fn installed_version(root: &Path) -> Option<String> {
    let v = std::fs::read_to_string(root.join(MARKER)).ok()?;
    let v = v.trim();
    (!v.is_empty()).then(|| v.to_string())
}

pub(crate) fn write_marker(root: &Path) -> std::io::Result<()> {
    std::fs::write(root.join(MARKER), CURRENT_VERSION)
}

/// The install folder: the running copy's, otherwise the remembered one (Linux/macOS).
pub fn installed_root() -> Option<PathBuf> {
    program_root().filter(|r| is_install_root(r)).or_else(|| remembered_root().filter(|r| is_install_root(r)))
}

/// `portable.txt` next to the program, `--portable` or the portable env var: data next to it,
/// never offer to install.
pub fn portable() -> bool {
    std::env::var_os(ENV_PORTABLE).is_some()
        || program_root().is_some_and(|r| r.join("portable.txt").exists())
}

/// The data folder: always the folder the app lives in (the install folder when installed, else
/// next to the program). Never the OS's app-data folders.
pub fn data_root() -> PathBuf {
    if let Some(d) = std::env::var_os(ENV_DATA_DIR).filter(|d| !d.is_empty()) {
        return PathBuf::from(d);
    }
    let root = if portable() { program_root() } else { installed_root().or_else(program_root) };
    root.unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn same_path(a: &Path, b: &Path) -> bool {
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let (a, b) = (canon(a), canon(b));
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Check a folder typed into the install prompt. `~` expands to the home folder.
pub fn parse_install_dir(text: &str) -> Result<PathBuf, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("Choose a folder.".into());
    }
    let path = match text.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') => {
            home_dir().ok_or("Could not find your home folder.")?.join(rest.trim_start_matches('/'))
        }
        _ => PathBuf::from(text),
    };
    if !path.is_absolute() {
        return Err(format!("Use a full path, for example ~/Apps/{APP_NAME}."));
    }
    if path.parent().is_none() {
        return Err("Pick a folder, not the root of a drive.".into());
    }
    if path.is_file() {
        return Err("That's a file, not a folder.".into());
    }
    Ok(path)
}

// ---------------------------------------------------------------------------------------------
// Moving old data into the install folder
// ---------------------------------------------------------------------------------------------

fn remove_any(p: &Path) -> std::io::Result<()> {
    if p.is_dir() {
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    }
}

fn copy_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_all(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(src, dst).map(|_| ())
    }
}

/// Move a file or folder: a rename, or across drives a copy and then delete. A failed copy leaves
/// the original untouched.
pub(crate) fn move_entry(src: &Path, dst: &Path) -> std::io::Result<()> {
    if let Some(p) = dst.parent() {
        std::fs::create_dir_all(p)?;
    }
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    if let Err(e) = copy_all(src, dst) {
        let _ = remove_any(dst);
        return Err(e);
    }
    let _ = remove_any(src);
    Ok(())
}

fn migrate_legacy_data(root: &Path) -> Vec<String> {
    let mut notes = Vec::new();
    for (src, name) in legacy_files() {
        let dst = root.join(name);
        if !src.is_file() || dst.exists() || same_path(&src, &dst) {
            continue;
        }
        match move_entry(&src, &dst) {
            Ok(()) => notes.push(format!("moved {} to {}", src.display(), dst.display())),
            Err(e) => notes.push(format!("could not move {} ({e}); it was left there", src.display())),
        }
    }
    let dirs = legacy_dirs();
    if let Some((cache, rest)) = dirs.split_first() {
        // the old cache only held downloaded block lists: they are fetched again
        if !same_path(cache, root) && !root.starts_with(cache) {
            let _ = std::fs::remove_dir_all(cache);
        }
        for d in std::iter::once(cache).chain(rest) {
            // empty folders only (and their empty parents, e.g. `Akulav\IPTrail`)
            for p in d.ancestors().take(3) {
                if std::fs::remove_dir(p).is_err() {
                    break;
                }
            }
        }
    }
    notes
}

// ---------------------------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------------------------

/// Installing offered on Linux/macOS when the binary runs from outside its install folder.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(windows, allow(dead_code))]
pub enum Offer {
    #[default]
    None,
    Install {
        suggested: PathBuf,
    },
    Update {
        root: PathBuf,
        installed: Option<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct StartupInfo {
    pub installed: bool,
    pub updated_from: Option<String>,
    pub offer: Offer,
    pub notes: Vec<String>,
}

pub enum Startup {
    Run(StartupInfo),
    Exit,
}

/// Flags this module handles (`--portable`, `--here`, `--installed`, `--updated-from <v>`, `--uninstall`).
pub fn is_install_flag(arg: &str) -> bool {
    matches!(arg, "--portable" | "--here" | "--installed" | "--updated-from" | "--uninstall")
}

pub fn on_startup(args: &[String], gui: bool) -> Startup {
    let flag = |f: &str| args.iter().any(|a| a == f);
    if flag("--portable") {
        std::env::set_var(ENV_PORTABLE, "1");
    }
    let Ok(exe) = std::env::current_exe() else { return Startup::Run(StartupInfo::default()) };
    let _ = std::fs::remove_file(sibling(&exe, OLD_SUFFIX));
    let _ = std::fs::remove_file(sibling(&exe, NEW_SUFFIX));

    if flag("--uninstall") {
        platform::uninstall_interactive();
        return Startup::Exit;
    }
    let mut info = StartupInfo {
        installed: flag("--installed"),
        updated_from: args
            .iter()
            .position(|a| a == "--updated-from")
            .and_then(|i| args.get(i + 1))
            .filter(|v| !v.trim().is_empty() && !v.starts_with("--"))
            .cloned(),
        ..Default::default()
    };
    if let Some(root) = program_root().filter(|r| is_install_root(r)) {
        platform::on_installed_start(&root);
        let _ = std::fs::remove_dir_all(root.join("update"));
        info.notes = migrate_legacy_data(&root);
    } else if gui && !cfg!(debug_assertions) && !flag("--here") && !portable() {
        info.offer = platform::startup_offer();
    }
    Startup::Run(info)
}

// ---------------------------------------------------------------------------------------------
// Install, uninstall, update
// ---------------------------------------------------------------------------------------------

/// Install this binary into `root` (or update the copy there), then start it with `args`.
/// On success the caller must exit. Windows installs with the setup instead.
pub fn install_and_launch(root: &Path, args: &[&str]) -> Result<(), String> {
    platform::install_and_launch(root, args)
}

/// Delete [`DATA_ENTRIES`] in `dir` (never anything else), then `dir` itself if it is empty.
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn delete_data(dir: &Path) {
    for name in DATA_ENTRIES {
        let _ = remove_any(&dir.join(name));
    }
    for d in legacy_dirs() {
        let _ = std::fs::remove_dir_all(d);
    }
    let _ = std::fs::remove_dir(dir);
}

fn sha256_file(path: &Path) -> std::io::Result<String> {
    use sha2::Digest;
    use std::io::Read;
    let mut hasher = sha2::Sha256::new();
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Replace `target` with `source` even if `target` is the running executable. The previous file
/// stays as `.old` until the next start.
#[cfg_attr(windows, allow(dead_code))]
pub(crate) fn swap_in(source: &Path, target: &Path) -> Result<(), String> {
    let old = sibling(target, OLD_SUFFIX);
    let _ = std::fs::remove_file(&old);
    let had_target = target.exists();
    if had_target {
        std::fs::rename(target, &old)
            .map_err(|e| format!("could not move {} aside: {e}", target.display()))?;
    }
    if let Err(e) = std::fs::rename(source, target).or_else(|_| std::fs::copy(source, target).map(|_| ())) {
        if had_target {
            let _ = std::fs::rename(&old, target);
        }
        return Err(format!("could not write {}: {e}", target.display()));
    }
    let _ = std::fs::remove_file(source);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(target, std::fs::Permissions::from_mode(0o755));
    }
    Ok(())
}

fn looks_like_executable(bytes: &[u8]) -> bool {
    if cfg!(windows) {
        bytes.starts_with(b"MZ")
    } else if cfg!(target_os = "macos") {
        bytes.len() >= 4 && matches!(&bytes[..4], [0xcf, 0xfa, 0xed, 0xfe] | [0xca, 0xfe, 0xba, 0xbe])
    } else {
        bytes.starts_with(b"\x7fELF")
    }
}

/// Reject anything that isn't a program for this OS or doesn't match its published SHA-256.
pub fn verify_update(file: &Path, sha256: &str) -> Result<(), String> {
    let mut head = [0u8; 4];
    {
        use std::io::Read;
        let mut f = std::fs::File::open(file).map_err(|e| e.to_string())?;
        let _ = f.read(&mut head).map_err(|e| e.to_string())?;
    }
    if !looks_like_executable(&head) {
        let _ = std::fs::remove_file(file);
        return Err("the download is not a program for this system".into());
    }
    if !sha256_file(file).map_err(|e| e.to_string())?.eq_ignore_ascii_case(sha256) {
        let _ = std::fs::remove_file(file);
        return Err("the download did not match its SHA-256 checksum, so it was deleted".into());
    }
    Ok(())
}

/// Where a downloaded update waits: inside the app's own folder, never the temp folder.
fn staging_path() -> Result<PathBuf, String> {
    let dir = data_root().join("update");
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;
    Ok(dir.join(update::this_os_asset().unwrap_or("update")))
}

/// No release file is anywhere near this; refuse anything bigger.
const MAX_DOWNLOAD: u64 = 256 << 20;

/// Download and verify this OS's release file. `on_bytes(done, total)` reports progress.
pub fn download_update(info: &update::UpdateInfo, on_bytes: &dyn Fn(u64, u64)) -> Result<PathBuf, String> {
    use std::io::{Read, Write};
    let staged = staging_path()?;
    let _ = std::fs::remove_file(&staged);
    let resp = update::agent().get(&info.url).call().map_err(|e| format!("download failed: {e}"))?;
    let total = resp.body().content_length().unwrap_or(0);
    let mut reader = resp.into_body().into_reader();
    let mut file = std::fs::File::create(&staged).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("download failed: {e}"))?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > MAX_DOWNLOAD {
            drop(file);
            let _ = std::fs::remove_file(&staged);
            return Err("the update is unexpectedly large, so it was deleted".into());
        }
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        on_bytes(done, total.max(done));
    }
    file.flush().map_err(|e| e.to_string())?;
    drop(file);
    verify_update(&staged, &info.sha256)?;
    Ok(staged)
}

/// Apply a verified download: Windows runs the setup in update mode (same folder), Linux/macOS
/// swap the binary in place. Either way the app restarts; the caller must exit right away.
pub fn apply_update(staged: &Path, sha256: &str) -> Result<(), String> {
    verify_update(staged, sha256)?;
    platform::apply_update(staged)
}

fn remembered_root() -> Option<PathBuf> {
    platform::remembered_root()
}

// ---------------------------------------------------------------------------------------------
// Windows: Inno Setup
// ---------------------------------------------------------------------------------------------

#[cfg(windows)]
mod platform {
    use super::{installed_root, Offer, CURRENT_VERSION};
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// The app always runs from its install folder on Windows.
    pub fn remembered_root() -> Option<PathBuf> {
        None
    }

    pub fn startup_offer() -> Offer {
        Offer::None
    }

    pub fn on_installed_start(root: &Path) {
        if super::installed_version(root).as_deref() != Some(CURRENT_VERSION) {
            let _ = super::write_marker(root);
        }
    }

    pub fn install_and_launch(_root: &Path, _args: &[&str]) -> Result<(), String> {
        Err(format!("On Windows, install with {}.", super::update::ASSET_WINDOWS))
    }

    /// Run the setup silently in update mode into this copy's folder; it closes the app and
    /// starts the new version with `--updated-from <this version>`.
    pub fn apply_update(setup: &Path) -> Result<(), String> {
        let mut cmd = Command::new(setup);
        cmd.args(["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS", "/UPDATE"]);
        cmd.arg(format!("/FROM={CURRENT_VERSION}"));
        if let Some(root) = super::program_root().filter(|r| super::is_install_root(r)) {
            cmd.arg(format!("/DIR={}", root.display()));
        }
        cmd.spawn().map(|_| ()).map_err(|e| format!("could not start the setup: {e}"))
    }

    pub fn uninstall_interactive() {
        match installed_root().map(|r| r.join("unins000.exe")).filter(|u| u.is_file()) {
            Some(u) => {
                let _ = Command::new(u).spawn();
            }
            None => eprintln!("{} is not installed here. Remove it from Settings → Apps.", super::APP_NAME),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Linux / macOS: the binary installs itself
// ---------------------------------------------------------------------------------------------

#[cfg(unix)]
mod platform {
    use super::{
        installed_root, installed_version, is_install_root, same_path, sibling, swap_in, update,
        write_marker, Offer, APP_ID, APP_NAME, CURRENT_VERSION, EXE_NAME, NEW_SUFFIX,
    };
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    fn pointer_file() -> Option<PathBuf> {
        super::config_dir().map(|d| d.join(APP_ID).join("install-dir"))
    }

    pub fn remembered_root() -> Option<PathBuf> {
        let text = std::fs::read_to_string(pointer_file()?).ok()?;
        let dir = text.trim();
        (!dir.is_empty()).then(|| PathBuf::from(dir))
    }

    fn remember(root: &Path) {
        if let Some(file) = pointer_file() {
            if let Some(d) = file.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(&file, root.to_string_lossy().as_bytes());
        }
    }

    fn forget() {
        if let Some(f) = pointer_file() {
            let _ = std::fs::remove_file(&f);
            if let Some(d) = f.parent() {
                let _ = std::fs::remove_dir(d);
            }
        }
    }

    pub fn default_install_dir() -> PathBuf {
        let home = super::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        if cfg!(target_os = "macos") {
            home.join("Applications").join(APP_NAME)
        } else {
            super::data_local_dir().unwrap_or_else(|| home.join(".local/share")).join(APP_NAME)
        }
    }

    fn bundle(root: &Path) -> PathBuf {
        root.join(format!("{APP_NAME}.app"))
    }

    pub fn installed_bin(root: &Path) -> PathBuf {
        if cfg!(target_os = "macos") {
            bundle(root).join("Contents/MacOS").join(EXE_NAME)
        } else {
            root.join(EXE_NAME)
        }
    }

    pub fn startup_offer() -> Offer {
        match installed_root() {
            Some(root) => {
                let installed = installed_version(&root);
                let missing = !installed_bin(&root).is_file();
                if missing || installed.as_deref().is_none_or(|v| update::is_newer(CURRENT_VERSION, v)) {
                    Offer::Update { root, installed }
                } else {
                    Offer::None
                }
            }
            None => Offer::Install { suggested: default_install_dir() },
        }
    }

    pub fn on_installed_start(root: &Path) {
        if installed_version(root).as_deref() != Some(CURRENT_VERSION) {
            let _ = write_marker(root);
        }
        if remembered_root().is_none_or(|r| !same_path(&r, root)) {
            remember(root);
        }
    }

    fn install_to(root: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(root).map_err(|e| format!("could not create {}: {e}", root.display()))?;
        let me = std::env::current_exe().map_err(|e| e.to_string())?;
        let bin = installed_bin(root);
        if !same_path(&me, &bin) {
            if let Some(p) = bin.parent() {
                std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let staged = sibling(&bin, NEW_SUFFIX);
            std::fs::copy(&me, &staged)
                .map_err(|e| format!("could not copy the app to {}: {e}", bin.display()))?;
            std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            swap_in(&staged, &bin)?;
        }
        if cfg!(target_os = "macos") {
            write_bundle(root)?;
        } else {
            integrate(&bin);
        }
        write_marker(root).map_err(|e| e.to_string())?;
        remember(root);
        Ok(bin)
    }

    fn launch(root: &Path, bin: &Path, args: &[&str]) -> Result<(), String> {
        let mut cmd = if cfg!(target_os = "macos") {
            let mut c = Command::new("/usr/bin/open");
            c.arg("-n").arg(bundle(root));
            if !args.is_empty() {
                c.arg("--args");
            }
            c
        } else {
            Command::new(bin)
        };
        cmd.args(args).current_dir(root);
        cmd.spawn().map(|_| ()).map_err(|e| format!("installed, but could not start it: {e}"))
    }

    pub fn install_and_launch(root: &Path, args: &[&str]) -> Result<(), String> {
        let bin = install_to(root)?;
        launch(root, &bin, args)
    }

    /// Swap the verified binary in place and start it.
    pub fn apply_update(staged: &Path) -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::fs::set_permissions(staged, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
        swap_in(staged, &exe)?;
        let root = super::root_of_exe(&exe).ok_or("unknown install folder")?;
        if cfg!(target_os = "macos") && is_install_root(&root) {
            sign(&bundle(&root));
            return launch(&root, &exe, &["--updated-from", CURRENT_VERSION]);
        }
        Command::new(&exe)
            .args(["--updated-from", CURRENT_VERSION])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("updated, but could not restart: {e}"))
    }

    fn uninstall(delete_data: bool) -> Result<(), String> {
        let root = installed_root().ok_or(format!("{APP_NAME} is not installed"))?;
        if cfg!(target_os = "macos") {
            let _ = std::fs::remove_dir_all(bundle(&root));
        } else {
            remove_integration();
            let bin = installed_bin(&root);
            for p in [bin.clone(), sibling(&bin, ".old"), sibling(&bin, ".new")] {
                let _ = std::fs::remove_file(p);
            }
        }
        let _ = std::fs::remove_file(root.join(super::MARKER));
        forget();
        if delete_data {
            super::delete_data(&root);
        }
        let _ = std::fs::remove_dir(&root);
        Ok(())
    }

    pub fn uninstall_interactive() {
        use std::io::{IsTerminal, Write};
        let ask = |q: &str| {
            print!("{q} [y/N] ");
            let _ = std::io::stdout().flush();
            let mut line = String::new();
            std::io::stdin().read_line(&mut line).is_ok()
                && matches!(line.trim().to_ascii_lowercase().as_str(), "y" | "yes")
        };
        if !std::io::stdin().is_terminal() {
            eprintln!("Run `{APP_ID} --uninstall` from a terminal.");
            return;
        }
        let Some(root) = installed_root() else {
            println!("{APP_NAME} is not installed.");
            return;
        };
        if !ask(&format!("Uninstall {APP_NAME} from {}?", root.display())) {
            return;
        }
        let delete_data = ask("Also delete its settings and saved data?");
        match uninstall(delete_data) {
            Ok(()) => println!("{APP_NAME} has been uninstalled."),
            Err(e) => eprintln!("Uninstall failed: {e}"),
        }
    }

    // --- Linux desktop integration ------------------------------------------------------------

    fn applications_dir() -> Option<PathBuf> {
        super::data_local_dir().map(|d| d.join("applications"))
    }
    fn icon_path() -> Option<PathBuf> {
        super::data_local_dir().map(|d| d.join("icons/hicolor/256x256/apps").join(format!("{APP_ID}.png")))
    }
    fn bin_link() -> Option<PathBuf> {
        super::home_dir().map(|h| h.join(".local/bin").join(APP_ID))
    }

    /// Quote a path for a .desktop `Exec=` key (freedesktop spec: escape " ` $ \ inside quotes).
    pub(super) fn exec_quote(p: &Path) -> String {
        let mut out = String::from("\"");
        for c in p.to_string_lossy().chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                out.push('\\');
            }
            out.push(c);
        }
        out.push('"');
        out
    }

    fn integrate(exe: &Path) {
        let entry = format!(
            "[Desktop Entry]\nType=Application\nName={APP_NAME}\n{}Exec={}\nIcon={APP_ID}\nTerminal=false\nStartupWMClass={APP_ID}\n",
            super::DESKTOP_KEYS,
            exec_quote(exe)
        );
        if let Some(icon) = icon_path() {
            if let Some(p) = icon.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            let _ = std::fs::write(&icon, super::ICON_PNG);
        }
        if let Some(dir) = applications_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join(format!("{APP_ID}.desktop")), entry);
            let _ = Command::new("update-desktop-database").arg(&dir).output();
        }
        if let Some(link) = bin_link() {
            if let Some(p) = link.parent() {
                let _ = std::fs::create_dir_all(p);
            }
            let _ = std::fs::remove_file(&link);
            let _ = std::os::unix::fs::symlink(exe, &link);
        }
    }

    fn remove_integration() {
        if let Some(dir) = applications_dir() {
            let _ = std::fs::remove_file(dir.join(format!("{APP_ID}.desktop")));
            let _ = Command::new("update-desktop-database").arg(&dir).output();
        }
        for p in [icon_path(), bin_link()].into_iter().flatten() {
            let _ = std::fs::remove_file(p);
        }
    }

    // --- macOS app bundle ---------------------------------------------------------------------

    /// `.icns` holding the 256×256 PNG icon (`ic08`).
    pub(super) fn icns() -> Vec<u8> {
        let png = super::ICON_PNG;
        let entry_len = 8 + png.len() as u32;
        let mut out = Vec::with_capacity(8 + entry_len as usize);
        out.extend_from_slice(b"icns");
        out.extend_from_slice(&(8 + entry_len).to_be_bytes());
        out.extend_from_slice(b"ic08");
        out.extend_from_slice(&entry_len.to_be_bytes());
        out.extend_from_slice(png);
        out
    }

    fn info_plist() -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>{APP_NAME}</string>
  <key>CFBundleDisplayName</key><string>{APP_NAME}</string>
  <key>CFBundleIdentifier</key><string>{}</string>
  <key>CFBundleExecutable</key><string>{EXE_NAME}</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>{CURRENT_VERSION}</string>
  <key>CFBundleVersion</key><string>{CURRENT_VERSION}</string>
  <key>NSHumanReadableCopyright</key><string>Made by Akulav</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#,
            super::BUNDLE_ID
        )
    }

    fn sign(bundle: &Path) {
        let _ = Command::new("/usr/bin/codesign").args(["--force", "--deep", "-s", "-"]).arg(bundle).output();
    }

    fn write_bundle(root: &Path) -> Result<(), String> {
        let app = bundle(root);
        std::fs::create_dir_all(app.join("Contents/Resources")).map_err(|e| e.to_string())?;
        std::fs::write(app.join("Contents/Resources/icon.icns"), icns()).map_err(|e| e.to_string())?;
        std::fs::write(app.join("Contents/Info.plist"), info_plist()).map_err(|e| e.to_string())?;
        let _ = Command::new("xattr").args(["-dr", "com.apple.quarantine"]).arg(&app).output();
        sign(&app);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_names() {
        assert_eq!(sibling(Path::new("/x/IPTrail.exe"), ".old"), PathBuf::from("/x/IPTrail.old.exe"));
        assert_eq!(sibling(Path::new("/x/iptrail"), ".new"), PathBuf::from("/x/iptrail.new"));
    }

    #[test]
    fn program_roots() {
        let p = |s: &str| PathBuf::from(s);
        assert_eq!(root_of_exe(&p("/opt/x/iptrail")), Some(p("/opt/x")));
        assert_eq!(
            root_of_exe(&p("/Users/a/Applications/IPTrail/IPTrail.app/Contents/MacOS/iptrail")),
            Some(p("/Users/a/Applications/IPTrail"))
        );
        assert_eq!(root_of_exe(&p("/x/MacOS/iptrail")), Some(p("/x/MacOS")));
    }

    #[test]
    fn swaps_and_verifies() {
        let d = std::env::temp_dir().join(format!("{APP_ID}-install-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let (target, staged) = (d.join("app.exe"), d.join("app.new.exe"));
        std::fs::write(&target, b"v1").unwrap();
        std::fs::write(&staged, b"v2").unwrap();
        swap_in(&staged, &target).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"v2");
        assert_eq!(std::fs::read(d.join("app.old.exe")).unwrap(), b"v1");

        let bad = d.join("bad");
        std::fs::write(&bad, b"<html>404</html>").unwrap();
        assert!(verify_update(&bad, "00").is_err() && !bad.exists());
        let header: &[u8] = if cfg!(windows) {
            b"MZ\x90\x00"
        } else if cfg!(target_os = "macos") {
            &[0xcf, 0xfa, 0xed, 0xfe]
        } else {
            b"\x7fELF"
        };
        let good = d.join("good");
        std::fs::write(&good, header).unwrap();
        let sum = sha256_file(&good).unwrap();
        assert!(verify_update(&good, &sum.to_uppercase()).is_ok());
        assert!(verify_update(&good, &"0".repeat(64)).is_err() && !good.exists());

        std::fs::create_dir_all(d.join("a/b")).unwrap();
        std::fs::write(d.join("a/b/f"), b"x").unwrap();
        move_entry(&d.join("a"), &d.join("moved/a")).unwrap();
        assert!(d.join("moved/a/b/f").is_file() && !d.join("a").exists());
        copy_all(&d.join("moved"), &d.join("copy")).unwrap();
        assert!(d.join("copy/a/b/f").is_file());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn install_dir_input() {
        assert!(parse_install_dir("").is_err());
        assert!(parse_install_dir("relative/dir").is_err());
        if cfg!(unix) {
            assert!(parse_install_dir("/").is_err());
            assert_eq!(parse_install_dir(" /opt/x ").unwrap(), PathBuf::from("/opt/x"));
        } else {
            assert!(parse_install_dir(r"C:\").is_err());
            assert_eq!(parse_install_dir(r"D:\Apps\X").unwrap(), PathBuf::from(r"D:\Apps\X"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn unix_files() {
        assert_eq!(platform::exec_quote(Path::new("/a b/$x")), r#""/a b/\$x""#);
        let i = platform::icns();
        assert_eq!(&i[..4], b"icns");
        assert_eq!(u32::from_be_bytes(i[4..8].try_into().unwrap()) as usize, i.len());
    }
}
