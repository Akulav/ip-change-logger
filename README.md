<div align="center">

<img src="assets/logo.svg" width="112" alt="IPTrail logo">

# IPTrail

**Know every hop your public IP takes.**

IPTrail watches your public IP address, logs every change, and tells you who the
internet thinks you are — location, network, VPN/proxy/Tor status and abuse-list
hits. No accounts. No API keys.

[![CI](https://github.com/Akulav/ip-change-logger/actions/workflows/ci.yml/badge.svg)](https://github.com/Akulav/ip-change-logger/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/Akulav/ip-change-logger?label=release&color=6366F1)](https://github.com/Akulav/ip-change-logger/releases/latest)
[![Platforms](https://img.shields.io/badge/platforms-Windows%20%7C%20Linux-22D3EE)](#install)
[![Made with Rust](https://img.shields.io/badge/made%20with-Rust-B7410E?logo=rust)](https://www.rust-lang.org)

<img src="docs/screenshot.png" width="860" alt="IPTrail dashboard showing the current IP, a 'Masked' risk badge, stats, anonymity signals and change history">

</div>

---

## Why IPTrail?

VPNs drop, proxies rotate, ISPs reassign addresses. IPTrail sits quietly in the
background and makes sure you **notice** — with a timestamped trail you can go
back to later.

|  |  |
|---|---|
| 🛰️ **Always watching** | Polls your public IP every 5 s – 1 h (default 15 s) with automatic failover across four independent services, so one outage never looks like a change. |
| 🧾 **A real trail** | Every change is saved with time, previous address, location and flags — in the app, in `history.jsonl`, and in a plain-text log you can `grep`. |
| 🛡️ **Anonymity & threat signals** | Proxy/VPN, Tor exit node, data-center, mobile network, known attacker and known abuser — rolled up into a single **Clean / Masked / Flagged** verdict. |
| 🔑 **No API keys** | Uses free, keyless sources (ip-api.com, ipwho.is, Tor Project, blocklist.de, FireHOL). Nothing to sign up for. |
| 🔔 **Notifications** | A desktop notification the moment your address changes. |
| 🪟 **Stays out of the way** | Minimize or close to the system tray on Windows; headless mode for servers. |
| 🦀 **Small & fast** | A single native Rust binary. No runtime, no installer required. |

## Install

Grab the latest build from **[Releases](https://github.com/Akulav/ip-change-logger/releases/latest)**:

| Platform | Download | Notes |
|---|---|---|
| **Windows 10/11** (x64) | `IPTrail-<version>-portable.exe` | Single file, just run it. |
|  | `iptrail-<version>-windows-x86_64.zip` | The app, `iptrail-cli.exe` for terminals, and docs. |
| **Debian / Ubuntu** (x64) | `iptrail_<version>_amd64.deb` | `sudo apt install ./iptrail_<version>_amd64.deb` — adds a menu entry. |
| **Other Linux** (x64) | `iptrail-<version>-linux-x86_64.tar.gz` | Extract and run `./iptrail`. |

Every release ships a `SHA256SUMS.txt` to verify downloads.

> [!NOTE]
> Windows SmartScreen may warn about an unrecognised app because the binary is not
> code-signed. Choose **More info → Run anyway**, or build from source.

### Build from source

Requires [Rust](https://rustup.rs) 1.95 or newer.

```bash
git clone https://github.com/Akulav/ip-change-logger.git
cd ip-change-logger
cargo build --release
```

The binary lands in `target/release/iptrail` (`iptrail.exe` on Windows).
On Linux you may need `libxkbcommon-dev libwayland-dev libfontconfig1-dev` — see
[docs/development.md](docs/development.md).

## Usage

Launch **IPTrail** and it starts monitoring right away.

- **Pause / Start monitoring** — toggle polling.
- **Check now** — check immediately.
- **Settings** — interval, notifications, intelligence sources, tray behaviour, log file.
- **History** — double-click an entry to copy its address; 📁 opens the log folder.

### Command line

On Windows, use `iptrail-cli.exe` (in the zip) from a terminal — it is the same program built as a console app, so output appears before the prompt returns. On Linux, `iptrail` does both.

```text
iptrail                       open the desktop app
iptrail --once                print the current IP with location and signals, then exit
iptrail --headless            monitor in the terminal, printing each change
iptrail --interval 60         override the check interval for this run (5–3600 s)
iptrail --no-intel --no-feeds skip lookups / blocklists for this run
iptrail --paths               show where settings, history and logs live
iptrail --demo                open the app with sample data (offline)
```

```console
$ iptrail --once
203.0.113.18
  location     Zurich, Switzerland
  network      Example VPN AG (AS64511)
  timezone     Europe/Zurich
  reverse      node-17.vpn.example.com
  anonymous    yes
  threat       no
  signals      proxy=yes tor=no hosting=yes mobile=no attacker=no abuser=no
```

## Where data goes

| What | Windows | Linux |
|---|---|---|
| Settings | `%APPDATA%\Akulav\IPTrail\config\settings.json` | `~/.config/iptrail/settings.json` |
| History + log | `%APPDATA%\Akulav\IPTrail\data\` | `~/.local/share/iptrail/` |
| Blocklist cache | `%LOCALAPPDATA%\Akulav\IPTrail\cache\` | `~/.cache/iptrail/` |

Run `iptrail --paths` to see the exact locations on your machine.

A log line looks like this:

```text
2026-11-03 20:06:41 +01:00  203.0.113.18     (was 198.51.100.24)  Zurich, Switzerland  · Example VPN AG (AS64511)  [proxy/VPN, data center]
```

## Data sources & privacy

IPTrail only talks to the services below, and only to ask about **your own**
public address. Nothing is sent anywhere else and there is no telemetry.

| Purpose | Service | Notes |
|---|---|---|
| Public IP | [ipify](https://www.ipify.org), [icanhazip](https://icanhazip.com), [AWS checkip](https://checkip.amazonaws.com), [ident.me](https://ident.me) | HTTPS, tried in order |
| Location, ISP, proxy/hosting/mobile | [ip-api.com](https://ip-api.com) | Free tier is **HTTP only**, 45 req/min, non-commercial |
| Fallback location | [ipwho.is](https://ipwho.is) | HTTPS |
| Tor exit nodes | [Tor Project bulk exit list](https://check.torproject.org/torbulkexitlist) | Cached 1 h |
| Known attackers | [blocklist.de](https://www.blocklist.de) | Reports from the last 48 h, cached 6 h |
| Known abusers | [FireHOL level 1](https://iplists.firehol.org/?ipset=firehol_level1) | Spamhaus DROP, DShield, Feodo…, cached 12 h |

Lookups run only when your address changes. Either source can be switched off in
Settings. Details in [docs/data-sources.md](docs/data-sources.md).

## Documentation

- [How it works](docs/architecture.md) — threads, data flow and file formats
- [Data sources](docs/data-sources.md) — what each signal means and its limits
- [Development](docs/development.md) — building, testing, icons, project layout
- [Releasing](docs/releasing.md) — versioning and the automated release pipeline
- [Changelog](CHANGELOG.md)

## Upgrading from IPLogger 2.x

IPTrail 3 replaces the old C# IPLogger. The ipdata.co key and `key.txt` are no
longer needed — delete them. Your old `ip.txt` on the desktop is left untouched;
new changes go to `ip-changes.log` (or any path you choose in Settings).

## License

IPTrail is **source-available** under the [Akulav Source-Available License 1.0](LICENSE):
free to use, read and modify privately; redistribution and commercial use are not
permitted. Third-party data services keep their own terms.

<div align="center"><sub>Made by <a href="https://github.com/Akulav">Akulav</a></sub></div>
