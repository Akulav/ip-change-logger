# Changelog

All notable changes to IPTrail are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
the project uses [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [3.1.0] - 2026-11-12

### Added
- **One file per system that installs and updates itself**, like the other Akulav apps:
  `IPTrail-setup.exe` (Windows), `IPTrail-linux-x64` and, new, `IPTrail-macos-arm64` (Apple Silicon).
  Windows asks for the install folder once; Linux and macOS offer to install themselves into a folder you pick
  (with an app-menu entry and `iptrail` command, or `IPTrail.app`), or *Just run it*.
- **In-app updates**: IPTrail checks GitHub 10 seconds after start and every 6 hours; *Update* downloads the new
  version into the install folder, verifies it with SHA-256, and *Restart & update* installs it into the same folder.
- `--here`, `--portable` (or `portable.txt` next to the program) and `--uninstall`.

### Changed
- **Everything IPTrail saves now lives in its install folder**: settings, history, the change log and the
  block-list cache. Nothing goes to `%APPDATA%`, `~/.config`, `~/.local/share` or `~/.cache` any more. Files from
  3.0 are moved in on the first start, also when the install folder is on another drive.
- One build workflow: every push is checked and built for all three systems. The portable exe, zip, tar.gz and
  .deb downloads are replaced by the three files above.
- New README in the same style as the other Akulav apps.

## [3.0.0] - 2026-11-03

IPTrail 3 is a ground-up rewrite of the original C#/WinForms **IPLogger** in Rust.

### Added
- New name and identity: **IPTrail**, with a new logo and app icon.
- Cross-platform desktop app for **Windows and Linux** (egui), with a dark dashboard UI:
  current IP hero card, risk badge, stats, anonymity/threat signals, network details
  and a change timeline.
- **No API key required.** Intelligence now comes from free sources:
  ip-api.com (location, ISP/ASN, reverse DNS, proxy/hosting/mobile flags) with an
  ipwho.is fallback, the Tor Project exit list, blocklist.de and FireHOL level 1.
- Public IP detection with automatic failover across ipify, icanhazip,
  AWS checkip and ident.me — one service being down no longer looks like an IP change.
- Desktop notifications when the address changes.
- Persistent history (`history.jsonl`) that survives restarts, and changes that
  happened while the app was closed are detected on the next start.
- Settings dialog: check interval (5 s – 1 h), auto-start, notifications,
  intelligence toggles, tray behaviour and a custom log file path.
- Command line: `--once`, `--headless`, `--interval`, `--no-intel`, `--no-feeds`, `--paths`, `--demo`;
  Windows ships `iptrail-cli.exe` for terminal use.
- Windows system tray icon with Show / Check now / Pause / Quit menu.
- `.deb` package and tarball for Linux, zip and portable `.exe` for Windows,
  published automatically to GitHub Releases with SHA-256 checksums.

### Changed
- The change log moved from `ip.txt` on the desktop to the app data folder
  (`ip-changes.log`); the path is configurable.
- Intelligence is fetched only when the IP changes (2.x queried ipdata.co on every tick).
- Public IP is fetched over HTTPS (2.x used plain-HTTP checkip.dyndns.org).

### Removed
- ipdata.co integration and the `key.txt` API key file.
- The C# / .NET Framework project.

## [2.5.1] - 2021-02-20

Last release of the original Windows-only IPLogger (C#, WinForms) using ipdata.co.
Earlier 1.x/2.x versions are recorded in the git history.

[Unreleased]: https://github.com/Akulav/ip-change-logger/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/Akulav/ip-change-logger/releases/tag/v3.0.0
