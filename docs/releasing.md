# Versioning & releases

IPTrail follows [Semantic Versioning](https://semver.org):

| Bump | When |
|---|---|
| **MAJOR** `x.0.0` | Breaking changes: removed CLI flags, incompatible settings/history formats |
| **MINOR** `3.x.0` | New features, new data sources, new settings |
| **PATCH** `3.0.x` | Bug fixes and small polish |

The single source of truth is `version` in `Cargo.toml`. The UI footer, `--version`, the Windows file
properties, the setup and the release tag all come from it.

> History: the C# IPLogger reached 2.5.1, so the Rust rewrite starts at **3.0.0**.

## Cutting a release

1. Move the items under `## [Unreleased]` in `CHANGELOG.md` into a new `## [X.Y.Z] - YYYY-MM-DD` section.
2. Bump `version` in `Cargo.toml` and run `cargo build` so `Cargo.lock` updates.
3. Commit (`Release vX.Y.Z`) and push to `main`.

The **Build & Release** workflow (`.github/workflows/build.yml`) notices that release `vX.Y.Z` doesn't exist
yet and:

1. checks formatting and Clippy, then tests and builds on Windows, Linux and macOS (`--locked`),
2. packages exactly one file per system, the same file for installing and updating:
   - `IPTrail-setup.exe` (Inno Setup, `installer/iptrail.iss`; installs `IPTrail.exe` and `iptrail-cli.exe`)
   - `IPTrail-linux-x64` and `IPTrail-macos-arm64` (the binaries; they install themselves)
   - `SHA256SUMS.txt`
3. creates the `vX.Y.Z` tag and a GitHub Release whose notes are the matching `CHANGELOG.md` section.

These file names are API: installed copies look for them when they check for updates. Never rename them.

Pushes that don't change the version build and test only. You can also push a `vX.Y.Z` tag manually — the
workflow refuses it if it doesn't match `Cargo.toml`.

## Required repository settings

*Settings → Actions → General → Workflow permissions* must allow **Read and write** so the workflow can
create tags and releases (the workflow also requests `contents: write` explicitly).
