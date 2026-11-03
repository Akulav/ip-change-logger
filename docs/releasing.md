# Versioning & releases

IPTrail follows [Semantic Versioning](https://semver.org):

| Bump | When |
|---|---|
| **MAJOR** `x.0.0` | Breaking changes: removed CLI flags, incompatible settings/history formats |
| **MINOR** `3.x.0` | New features, new data sources, new settings |
| **PATCH** `3.0.x` | Bug fixes and small polish |

The single source of truth is `version` in `Cargo.toml`. The UI footer,
`--version`, the Windows file properties and the release tag all come from it.

> History: the C# IPLogger reached 2.5.1, so the Rust rewrite starts at **3.0.0**.

## Cutting a release

1. Move the items under `## [Unreleased]` in `CHANGELOG.md` into a new
   `## [X.Y.Z] - YYYY-MM-DD` section and add the compare link at the bottom.
2. Bump `version` in `Cargo.toml` and run `cargo build` so `Cargo.lock` updates.
3. Commit (`Release vX.Y.Z`) and push to `main`.

That's it. The **Release** workflow notices that `vX.Y.Z` doesn't exist yet and:

1. builds and tests on Windows and Linux (`--locked`),
2. packages
   - `iptrail-X.Y.Z-windows-x86_64.zip` and `IPTrail-X.Y.Z-portable.exe`
   - `iptrail-X.Y.Z-linux-x86_64.tar.gz` and `iptrail_X.Y.Z_amd64.deb`
   - `SHA256SUMS.txt`
3. creates the `vX.Y.Z` tag and a GitHub Release whose notes are the matching
   `CHANGELOG.md` section.

Pushes that don't change the version are a no-op for the release workflow (CI
still runs). You can also push a `vX.Y.Z` tag manually — the workflow refuses
it if it doesn't match `Cargo.toml`. A manual run is available from the
Actions tab (*Run workflow*).

## Required repository settings

*Settings → Actions → General → Workflow permissions* must allow **Read and
write** so the workflow can create tags and releases (the workflow also
requests `contents: write` explicitly).
