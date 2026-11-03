# Development

## Prerequisites

- Rust **1.95+** (`rustup update stable`)
- **Windows:** the MSVC toolchain (Visual Studio Build Tools).
- **Linux:** a C compiler plus

  ```bash
  sudo apt install build-essential pkg-config libxkbcommon-dev libwayland-dev \
    libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libfontconfig1-dev
  ```

## Everyday commands

```bash
cargo run                         # desktop app
cargo run -- --demo               # app with sample data, no network
cargo run -- --once               # one-shot CLI lookup
cargo test                        # unit tests
cargo fmt --all                   # format (rustfmt.toml: 110 columns)
cargo clippy --all-targets -- -D warnings
cargo build --release             # optimized, stripped binary
```

CI runs `fmt --check`, `clippy -D warnings`, `test` and a release build on both
Windows and Linux for every push and pull request.

## Project layout

```
assets/            logo.svg (source artwork), generated icon PNGs and icon.ico
docs/              this documentation and the README screenshot
examples/gen_icon.rs  renders the PNG/ICO icons from code
packaging/linux/   .desktop entry for the .deb and tarball
src/               application code — see architecture.md
build.rs           embeds icon + version info into the Windows .exe
.github/workflows/ ci.yml and release.yml
```

## Icons

The icon is drawn procedurally so it stays crisp at 16 px. After changing the
artwork in `examples/gen_icon.rs` (keep `assets/logo.svg` in sync), regenerate:

```bash
cargo run --example gen_icon
```

This rewrites `assets/icon-{32,64,128,256}.png` and the multi-size `assets/icon.ico`.

## Screenshots

Never put a real public IP in the docs. Use the demo mode, which only shows
RFC 5737 documentation addresses (`198.51.100.0/24`, `203.0.113.0/24`):

```bash
cargo run -- --demo
```

## Platform notes

- **Windows:** `iptrail.exe` (`src/main.rs`) uses the GUI subsystem in release builds,
  so it never flashes a console window. Shells don't wait for GUI programs, so
  `src/bin/iptrail-cli.rs` builds the same code as a console app (`iptrail-cli.exe`)
  for terminal use. Both call `iptrail::run()` in `src/lib.rs`.
- **Tray:** implemented on Windows only (`src/ui/tray.rs`). Linux desktops vary
  too much in tray support; there the window simply minimizes.
- **Fonts:** the UI uses Segoe UI / Cascadia Mono on Windows and Inter or Noto
  on Linux when installed, falling back to egui's bundled fonts.
