//! IPTrail — watch your public IP address and log every change.
//!
//! The logic lives in this library; `src/main.rs` (desktop, GUI subsystem on
//! Windows) and `src/bin/iptrail-cli.rs` (console) are thin entry points.

mod config;
mod demo;
mod history;
mod install;
mod monitor;
mod net;
mod notify;
mod ui;

use config::{Paths, Settings, MAX_INTERVAL_SECS, MIN_INTERVAL_SECS};
use monitor::{Event, Monitor};
use std::process::ExitCode;

const HELP: &str = "\
IPTrail — watch your public IP address and log every change.

USAGE:
    iptrail [OPTIONS]

OPTIONS:
    (none)              Open the desktop app
    --once              Print the current public IP with its intelligence and exit
    --headless          Monitor in the terminal without a window (Ctrl+C to stop)
    --interval <SECS>   Override the check interval (5-3600) for this run
    --no-intel          Skip location / network lookups for this run
    --no-feeds          Skip Tor and abuse blocklist checks for this run
    --paths             Print where settings, history and logs are stored
    --demo              Open the app with sample data (offline; for screenshots)
    --here              Linux/macOS: run without offering to install
    --portable          Keep data next to the program and never offer to install
    --uninstall         Remove the installed app (Windows: starts the uninstaller)
    -V, --version       Print version
    -h, --help          Print this help
";

#[derive(Default)]
struct Args {
    once: bool,
    headless: bool,
    paths: bool,
    demo: bool,
    interval: Option<u64>,
    no_intel: bool,
    no_feeds: bool,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args::default();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("iptrail {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--once" => args.once = true,
            "--headless" => args.headless = true,
            "--paths" => args.paths = true,
            "--demo" => args.demo = true,
            "--no-intel" => args.no_intel = true,
            "--no-feeds" => args.no_feeds = true,
            // handled by `install::on_startup`
            "--updated-from" => {
                it.next();
            }
            f if install::is_install_flag(f) => {}
            "--interval" => {
                let v = it.next().ok_or("--interval needs a value in seconds")?;
                let secs: u64 = v.parse().map_err(|_| format!("invalid interval {v:?}"))?;
                if !(MIN_INTERVAL_SECS..=MAX_INTERVAL_SECS).contains(&secs) {
                    return Err(format!(
                        "interval must be between {MIN_INTERVAL_SECS} and {MAX_INTERVAL_SECS} seconds"
                    ));
                }
                args.interval = Some(secs);
            }
            other => return Err(format!("unknown argument {other:?} (see --help)")),
        }
    }
    Ok(Some(args))
}

/// Shared entry point for both binaries.
pub fn run() -> ExitCode {
    let has_cli_args = std::env::args().len() > 1;
    if has_cli_args {
        attach_console();
    }

    let args = match parse_args() {
        Ok(Some(a)) => a,
        Ok(None) => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    if args.demo {
        return match demo::paths().and_then(|paths| {
            ui::run(paths, demo::settings(), install::StartupInfo::default()).map_err(|e| e.to_string())
        }) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        };
    }

    // Installing / updating / uninstalling, moving old data into the install folder.
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let gui = !(args.once || args.headless || args.paths);
    let startup = match install::on_startup(&raw, gui) {
        install::Startup::Exit => return ExitCode::SUCCESS,
        install::Startup::Run(info) => info,
    };
    for note in &startup.notes {
        eprintln!("iptrail: {note}");
    }

    let paths = Paths::discover();
    let mut settings = Settings::load(&paths);
    if args.paths {
        println!("settings  {}", paths.settings.display());
        println!("history   {}", paths.history.display());
        println!("log       {}", settings.log_file(&paths).display());
        println!("cache     {}", paths.cache_dir.display());
        return ExitCode::SUCCESS;
    }
    if let Some(secs) = args.interval {
        settings.interval_secs = secs;
    }
    settings.ip_intel &= !args.no_intel;
    settings.threat_feeds &= !args.no_feeds;

    if args.once {
        return once(&paths, &settings);
    }
    if args.headless {
        return headless(paths, settings);
    }

    match ui::run(paths, settings, startup) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn once(paths: &Paths, settings: &Settings) -> ExitCode {
    let agent = net::agent();
    let lookup = match net::public_ip::fetch(&agent) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", lookup.ip);

    let mut intel = if settings.ip_intel {
        match net::intel::lookup(&agent, lookup.ip) {
            Ok(i) => Some(i),
            Err(e) => {
                eprintln!("warning: {e}");
                None
            }
        }
    } else {
        None
    };
    if settings.threat_feeds {
        let mut feeds = net::feeds::ThreatFeeds::new(paths.cache_dir.clone());
        let (v, problems) = feeds.check(&agent, lookup.ip);
        for p in problems {
            eprintln!("warning: {p}");
        }
        let i = intel.get_or_insert_with(Default::default);
        i.tor = v.tor;
        i.attacker = v.attacker;
        i.abuser = v.abuser;
    }
    if let Some(i) = intel {
        let yn = |v: Option<bool>| match v {
            Some(true) => "yes",
            Some(false) => "no",
            None => "unknown",
        };
        let show =
            |label: &str, v: Option<String>| println!("  {label:<12} {}", v.unwrap_or_else(|| "—".into()));
        show("location", i.place());
        show("network", i.network());
        show("timezone", i.timezone.clone());
        show("reverse", i.reverse_dns.clone());
        println!("  {:<12} {}", "anonymous", yn(i.anonymous()));
        println!("  {:<12} {}", "threat", yn(i.threat()));
        println!(
            "  {:<12} proxy={} tor={} hosting={} mobile={} attacker={} abuser={}",
            "signals",
            yn(i.proxy),
            yn(i.tor),
            yn(i.hosting),
            yn(i.mobile),
            yn(i.attacker),
            yn(i.abuser)
        );
    }
    ExitCode::SUCCESS
}

fn headless(paths: Paths, mut settings: Settings) -> ExitCode {
    settings.auto_start = true;
    println!(
        "IPTrail {} — checking every {}s, logging to {}",
        env!("CARGO_PKG_VERSION"),
        settings.interval_secs,
        settings.log_file(&paths).display()
    );
    let (tx, rx) = std::sync::mpsc::channel();
    let _monitor = Monitor::spawn(paths, settings, move |event| {
        let _ = tx.send(event);
    });
    for event in rx {
        match event {
            Event::Changed(entry) => println!("{}", entry.log_line()),
            Event::Error(e) => eprintln!("{} error: {e}", chrono::Local::now().format("%H:%M:%S")),
            Event::Updated => {}
        }
    }
    ExitCode::SUCCESS
}

/// Release builds on Windows have no console; reuse the parent's terminal for CLI output.
fn attach_console() {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(
            windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
        );
    }
}
