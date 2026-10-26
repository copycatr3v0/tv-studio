# Android TV Customizer Studio

A Windows app that customizes Android TV and Google TV sets over ADB: guided connection, safe and
reversible debloating, a visual theme studio, an optional lightweight launcher, one-click deployment
with verification, and exact rollback. No app needs to be installed on the TV unless you choose the
custom launcher.

Status: **M3 (Windows shell)**. See [docs/PLAN.md](docs/PLAN.md) and [docs/TASKS.md](docs/TASKS.md).

## Layout

| Path | What |
|---|---|
| `src/` | React + TypeScript (strict) + Tailwind UI |
| `src-tauri/` | Tauri 2 app crate: thin command layer |
| `crates/tvs-core` | Validated identifiers (package, component, serial) |
| `crates/tvs-transport` | ADB transport trait, adb.exe driver, scriptable mock TV |
| `crates/tvs-safety` | Hard safety rules (core packages, factory packages, launcher checks) |
| `crates/tvs-store` | SQLite: settings, devices, write-once baselines, versioned profiles, journal |
| `docs/` | Research, spec, architecture, capability matrix, plan |

## Develop (Windows)

Prerequisites: Rust stable (MSVC), Node 22, pnpm 10, Visual Studio Build Tools ("Desktop development with C++"), WebView2 (preinstalled on Windows 11).

```powershell
cd C:\dev\tv-studio
pnpm install
pnpm tauri dev          # run the app
pnpm test               # UI tests
cargo test --workspace  # core tests
pnpm tauri build        # NSIS installer + target\release\tv-studio.exe
```

Until adb is bundled (M4), the app uses `adb.exe` from your Android SDK (`%LOCALAPPDATA%\Android\Sdk\platform-tools`),
from `PATH`, or from the path set in Settings.

## Credits

The safety rules, recovery design and launcher are based on [CleanYourTV](https://github.com/Gathin23/cleanyourtv)
(MIT, © 2026 Gathin). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
