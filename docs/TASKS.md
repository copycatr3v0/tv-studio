# TASKS.md

Status: `[x]` done with tests · `[~]` in progress · `[ ]` open. A task is only ticked once its tests pass.

## M1 Research
- [x] Clone and study CleanYourTV source → RESEARCH.md

## M2 Planning
- [x] SPEC.md, ARCHITECTURE.md, CAPABILITY_MATRIX.md, PLAN.md, TASKS.md
- [x] User review of the docs (launcher fork, Android+Google TV, C:\dev\tv-studio, local+CI builds)

## M3 Windows shell
- [x] Create `C:\dev\tv-studio`, git init
- [ ] Private GitHub repo + first push (needs your `gh auth login`)
- [ ] Verify Windows toolchain on your PC: rustup stable, Node 22, pnpm, VS Build Tools (MSVC), WebView2, JDK 17, Android SDK 34
- [x] Scaffold Tauri 2 + React + TS strict + Tailwind + Cargo workspace (crates are added only when they get real code)
- [x] tvs-store: SQLite + migrations, write-once baseline, versioned profiles (7 tests)
- [x] App shell: 15-section nav, Beginner/Advanced mode, settings page, real adb/device status (7 UI tests)
- [x] CI workflow written: typecheck, vitest, cargo test, clippy, NSIS + exe artifact on windows-latest (first run pending the push)

## M4 ADB connectivity
- [x] Transport trait, AdbExeTransport, MockTransport (scripted states), typed ShellCommand (14 tests)
- [x] Discovery (`adb mdns services`), manual IP, pairing, legacy connect, TCP diagnostics with advice (tvs-device, 11 tests)
- [x] Capability probe (getprop, meminfo, df, packages, Home + HOME candidates, adb.secure) + OEM family detection; failed queries reported, never guessed
- [x] Connection wizard UI: per-brand guidance, legacy and pairing tabs, mDNS scan, bounded authorization wait, end-of-session port-closed check
- [x] Device Inspector UI; device remembered by build identity in SQLite
- [ ] Bundle adb platform-tools in the installer (licence review first; until then SDK/PATH/custom adb)
- [ ] Real-device validation on one Google TV and one Android TV (only mock-tested so far)

## M5 Inventory, safety, snapshot
- [ ] Import CleanYourTV examples → kb.sqlite (with attribution)
- [~] Guards 1–11 from RESEARCH §2 + ported fixtures (static ones done: core block, factory never-start, launcher usability, name validation)
- [ ] Snapshot/restore (first-wins), Debloat Center UI

## M6–M10
See PLAN.md. Tasks get broken down when each milestone starts.
