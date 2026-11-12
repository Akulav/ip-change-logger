# CLAUDE.md

Guidance for Claude Code (and other contributors) working in this repository.

## What this is

**IPTrail** (repo: `ip-change-logger`) — a Rust desktop app + CLI that polls the
machine's public IPv4 address, records every change, and shows location /
network / anonymity / threat signals from free, keyless sources. It replaced a
C# WinForms app called IPLogger (last version 2.5.1); the Rust rewrite started at 3.0.0.

## Commands

```bash
cargo run                     # GUI
cargo run -- --demo           # GUI with sample data, offline (use for screenshots)
cargo run -- --once           # CLI one-shot (hits the network)
cargo test
cargo fmt --all               # rustfmt.toml: max_width 110
cargo clippy --all-targets -- -D warnings   # CI enforces zero warnings
cargo run --example gen_icon  # regenerate assets/icon*.png + icon.ico
```

Before finishing a change, run fmt, clippy (`-D warnings`) and tests — CI runs
the same on Windows and Linux.

## Layout

- `src/lib.rs` — `run()`: arg parsing, `--once` / `--headless` / `--demo`, starts UI
- `src/main.rs` (GUI subsystem on Windows) and `src/bin/iptrail-cli.rs` (console)
  — one-line entry points calling `iptrail::run()`
- `src/monitor.rs` — background worker thread; `Command` in, `Event` out,
  `Arc<Mutex<State>>` shared with the UI. All network/disk I/O happens here.
- `src/net/` — `public_ip` (provider failover), `intel` (ip-api.com → ipwho.is,
  `IpIntel`, `Risk`), `feeds` (Tor / blocklist.de / FireHOL, cached on disk)
- `src/history.rs` — `history.jsonl` + human-readable `ip-changes.log`
- `src/config.rs` — `Settings` (serde, `#[serde(default)]`) and `Paths`
- `src/ui/` — eframe/egui 0.37 app (`mod.rs`), `theme.rs`, `widgets.rs`,
  `tray.rs` (Windows only)
- `docs/` — architecture, data sources, development, releasing

## Conventions & gotchas

- **egui 0.37 API**: `eframe::App` has `ui(&mut self, ui, frame)` and
  `logic(&mut self, ctx, frame)`. `logic` also runs while the window is hidden,
  which is what makes tray "Show" work. Panels are `egui::Panel::top/bottom(..).show(ui, ..)`.
  `Id::new` is deprecated → `Id::unique`. Default renderer is wgpu (no `glow`).
- Don't render emoji for status icons (egui draws color emoji); use the vector
  `widgets::glyph` or plain text.
- Never hold the `State` mutex across a network call.
- Intel is optional everywhere: `Option<bool>` means "unknown", not "no".
- Keep new settings backwards compatible — every `Settings` field needs a default.
- ip-api.com's free tier is HTTP-only; keep the HTTPS ipwho.is fallback and the
  docs note about it.
- **Never commit a real public IP** (screenshots, tests, docs). Use RFC 5737
  ranges: `192.0.2.0/24`, `198.51.100.0/24`, `203.0.113.0/24`.
- `iptrail.exe` uses `windows_subsystem = "windows"` in release; shells don't wait for it,
  so terminal use on Windows goes through `iptrail-cli.exe`. `attach_console()` is a
  best-effort fallback for the GUI binary.
- Tray is Windows-only by design (avoids GTK deps on Linux).

## Install, update and data location (same model as Akulav/akulav-mc-launcher)

- `src/install/` — `mod.rs` (install folder, marker `.iptrail-install`, data root, moving 3.0 data in,
  Windows/Inno + Linux/macOS self-install, download/verify/apply), `update.rs` (GitHub releases +
  `SHA256SUMS.txt`), `gui.rs` (install prompt, footer update button).
- **Everything the app writes lives in the install folder** (`Paths::discover` → `install::data_root`):
  settings, history, log, cache, downloaded update. Never the OS config/data/cache folders or the temp folder.
- Windows: `installer/iptrail.iss` (asks for the folder once, `/UPDATE /DIR=` reuses it; installs
  `IPTrail.exe` + `iptrail-cli.exe`). **Never change its AppId.** Linux/macOS: the binary offers to install
  itself into a folder the user picks (remembered in `<config dir>/iptrail/install-dir`).
- Linux/macOS-only code can't be compiled here (no cross C compiler for `ring`): review it by hand; CI checks it.

## Versioning & releases

- Version lives only in `Cargo.toml`; follow SemVer and update `CHANGELOG.md`
  (Keep a Changelog format) in the same commit.
- One workflow, `.github/workflows/build.yml`: lint + test + build on every push/PR; pushing to `main`
  with a version that has no `vX.Y.Z` release publishes exactly three files — `IPTrail-setup.exe`,
  `IPTrail-linux-x64`, `IPTrail-macos-arm64` — plus `SHA256SUMS.txt` (names are API: installed copies
  look for them), with the matching CHANGELOG section as notes. Don't create tags by hand unless they
  match `Cargo.toml`.
- `Cargo.lock` is committed; CI uses `--locked`.

## License

Source-available (see `LICENSE`), not open source. Don't add dependencies with
licenses that conflict with distributing binaries, and don't remove notices.
