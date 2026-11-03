# How IPTrail works

IPTrail is a single Rust binary with two front ends — the desktop window and the
command line — sharing one monitoring engine.

```
                  ┌───────────────────────────── iptrail-monitor thread ─────────────────────────────┐
 UI / headless ── Command ──▶  Worker loop                                                           │
 (main thread)                 │  every N seconds (or "Check now"):                                  │
      ▲                        │   1. public_ip::fetch  ── ipify → icanhazip → AWS → ident.me        │
      │                        │   2. same IP?  done (retry intel if it failed earlier)              │
      │                        │   3. new IP:   intel::lookup  ── ip-api.com → ipwho.is              │
      │                        │                feeds::check   ── Tor / blocklist.de / FireHOL       │
      │                        │   4. append history.jsonl + ip-changes.log, notify                  │
      │                        └──────────────┬──────────────────────────────────────────────────────┘
      │                                       │ writes
      └── reads (Mutex) ─────────────  Arc<Mutex<State>>  ◀──┘
      └── Event (repaint / Changed / Error) ◀── on_event callback
```

## Modules

| File | Responsibility |
|---|---|
| `src/lib.rs` | `run()`: argument parsing, `--once`, `--headless`, `--demo`, launching the UI |
| `src/main.rs`, `src/bin/iptrail-cli.rs` | GUI-subsystem and console entry points (Windows needs both) |
| `src/config.rs` | `Settings` (JSON) and `Paths` (platform data directories) |
| `src/monitor.rs` | Background worker, `Command`s in, `Event`s out, shared `State` |
| `src/history.rs` | `Entry`, `History` (JSON Lines), plain-text log lines |
| `src/net/public_ip.rs` | "What is my IP" providers with failover |
| `src/net/intel.rs` | `IpIntel`, risk scoring, ip-api.com and ipwho.is clients |
| `src/net/feeds.rs` | Downloaded blocklists, `IpSet` (single IPs + IPv4 CIDRs), disk cache |
| `src/notify.rs` | Desktop notifications |
| `src/demo.rs` | Sample data for `--demo` (RFC 5737 addresses only) |
| `src/ui/mod.rs` | The egui app: dashboard, settings, dialogs |
| `src/ui/theme.rs` | Palette, fonts (system UI font when available), widget styling |
| `src/ui/widgets.rs` | Cards, pills, chips, vector status glyphs |
| `src/ui/tray.rs` | Windows tray icon and menu |

## Threading

- **Main thread** runs the eframe/winit event loop. `App::logic` is called every
  frame *and* while the window is hidden (whenever something requests a repaint),
  which is how tray actions still work when IPTrail lives in the tray.
- **`iptrail-monitor`** does all network and disk I/O. It never holds the state
  lock during a network call.
- **Notification threads** are short-lived so a slow D-Bus/WinRT call can't stall
  monitoring.

## Change detection

The last recorded entry in `history.jsonl` is loaded at startup and treated as
the current address. If the first check returns something else, that is recorded
as a change — so a switch that happened while IPTrail was closed is not missed.

A failed check (all four providers down, no network) is reported in the footer
and never recorded as a change.

## Risk verdict

| Verdict | When |
|---|---|
| **Flagged** | listed by blocklist.de (attacker) or FireHOL level 1 (abuser) |
| **Masked** | proxy/VPN, Tor exit or data-center address, but not flagged |
| **Clean** | at least one signal known and none positive |
| **Unknown** | no intelligence available (lookups disabled or failed) |

`anonymous = tor || proxy` and `threat = attacker || abuser` mirror the
`is_anonymous` / `is_threat` fields IPLogger 2.x showed from ipdata.co.

## File formats

**`history.jsonl`** — one JSON object per line:

```json
{"at":"2026-11-03T20:06:41.123+01:00","ip":"203.0.113.18","previous":"198.51.100.24",
 "intel":{"country":"Switzerland","country_code":"CH","city":"Zurich","isp":"Example VPN AG",
          "asn":"AS64511 Example VPN AG","proxy":true,"hosting":true,"tor":false,
          "attacker":false,"abuser":false,"source":"ip-api.com", "...": "..."}}
```

**`ip-changes.log`** — one human-readable line per change (see the README).

**`settings.json`** — every field is optional; missing fields take defaults:

```json
{
  "interval_secs": 15,
  "auto_start": true,
  "notifications": true,
  "ip_intel": true,
  "threat_feeds": true,
  "minimize_to_tray": true,
  "close_to_tray": false,
  "log_path": null
}
```
