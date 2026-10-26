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
- [x] Import CleanYourTV examples → `crates/tvs-kb/data/kb.json` via `tools/build_kb.py` (316 lists, 1,691 packages, attributed)
- [x] Guards: core block, name validation, factory never-start, Home guard (selection-aware), setupwraith Wi-Fi, TV_INPUT_HARDWARE, input/Settings handlers, CVTE launcher, essential-service confirmations; all fail closed, no override (13 + 2 device tests)
- [x] Inventory (all/disabled/system per user 0), live facts gathering
- [x] Baseline (write-once) + pre-change snapshots; verified batch disable with stop-on-refusal / stop-on-disconnect, no blind retry; exact restore (full or scoped) with readback (9 tests)
- [x] Applications, Debloat Center (Safe / Balanced / Advanced, dry-run check, per-item confirmation) and Restore UI (4 UI tests)
- [x] "Revert to original" button in the app header + Restore page, with a baseline-status check (3 UI tests)
- [ ] Real-device round trip on a Google TV and an Android TV (mock-only so far)

## M6 Launcher fork + theme system
- [x] Fork CleanYourTV launcher as `studio.tvcustomizer.home` via its own `launcherApplicationId` mechanism (Java namespace unchanged; NOTICE + licence kept)
- [x] CI job: Gradle unit tests, lint, release build, 576 KB size gate, package-name check; APK bundled into the Windows installer
- [ ] First green launcher CI run (needs the GitHub push) — not compiled locally: no Android SDK reachable from this workspace
- [x] Theme schema v1 generated from CleanYourTV docs + built-in config.xml defaults (106 keys), shared by Rust and the UI
- [x] Rust validator, CleanYourTV import (keeps valid keys, reports dropped ones), remote-button keys refused until helpers are supported
- [x] Wallpaper intake: decode + re-encode to JPEG ≤1920×1080 (strips payloads), safe names
- [x] Theme delivery: staging → ThemeProvider `content write` → `cp` fallback → read-back verify → restart Home only if it is Home (13 tests)
- [x] Launcher install (pm install -r, typed refusals), Home handover verified on screen, hand-back, uninstall
- [x] Revert to original extended: Home handed back and confirmed before the launcher is uninstalled; kept if it can't be confirmed (round-trip test)
- [x] 15 original themes + original procedural wallpapers (25 after M12: 10 more added) (`tools/make_wallpapers.py`)
- [x] Live Theme Editor: 16:9 preview from the same keys, D-pad simulation, schema-driven controls, undo/redo, import/export, saved versions, wallpaper upload
- [x] Apply-to-TV flow (install → Home → theme, each verified) and Launcher Builder page (8 UI tests)
- [ ] Real-TV checks: preview vs. TV screenshot for 3 themes; theme push on Android 9 and on 12+

## M7 Automation engine
- [x] Versioned deployment profile (launcher, apps, animation speed, Private DNS, theme), unknown fields rejected
- [x] Planner: reads the TV, previews every step (check / will change / already done / blocked), dependency order, verified batches; launchers never in app batches; Private DNS needs API 28; profile can be bound to one TV
- [x] Runner: check-then-act steps, journal per step, dependents skipped on failure, critical failure stops; reconnect loop after a drop (re-runs the step once, finishing only what's left); stock launcher disabled only after Home re-confirmed, re-enabled at once if Home stops answering (14 tests)
- [x] Snapshot before the first change (baseline once + pre-deploy per run); DB v2 stores each run's request and snapshot
- [x] Resume an interrupted run from the TV's real state; discard; history; undo one deployment (incl. removing a launcher it installed, Home first)
- [x] Original state and undo now include Private DNS settings
- [x] Deployment page (dry run, Apply Everything, live progress, cancel, result, undo) and Profiles page (save/load/import/export, history) (6 UI tests)
- [ ] Real-TV run of a full deployment and of a reboot mid-run

## M8 Benchmarks, drift, privacy, diagnostics
- [x] Benchmark (tvs-insight): framestats key response/movement, cold start, return to Home, jank, idle CPU, PSS; cautious IQR-based verdicts (7 tests + mock run)
- [x] Privacy audit with graded evidence (TV-named voice service, tested TVs, name-only, AppOps history, system-UID caveat); never claims listening (3 tests)
- [x] Drift check vs. the last deployment: re-enabled apps, Home moved, launcher missing, settings, new preloads; repair = same plan after review (round-trip test)
- [x] DB v3 bench runs; diagnostics bundle with IP/serial redaction
- [x] Optimization Benchmarks, Privacy & Performance and Diagnostics pages (4 UI tests); all 15 sections functional
- [ ] Real-TV benchmark repeatability check (two runs, same launcher, should read "no clear change")

## M9 Integration testing
- [x] Simulated TCL Google TV (`crates/tvs-demo`), built from CleanYourTV's tested G09 list: 62 packages, setup wizard owning Wi-Fi, TV-input hardware, benchmark and privacy data
- [x] 13 end-to-end scenarios on it (full deployment + exact revert, multiple / unauthorized TVs, protected packages, plan-to-run changes, invalid configuration, firmware refusing everything, sticky packages, reboot mid-run, interrupted + resume, benchmark repeatability, privacy audit, demo pairing)
- [x] Found and fixed: an invalid package name in an imported profile was silently skipped; it now blocks the plan
- [x] Demo mode: Settings toggle (or `TVS_DEMO=1`), banner on every screen, simulated TV with its own identity so it can never share a baseline with a real TCL set (2 UI tests)
- [x] docs/TESTING.md: what the simulator proves and what only a real TV can
- [ ] Real-TV pass: the checklist in TESTING.md, on at least one Google TV and one Android TV
- [ ] Automated UI run of the desktop app in demo mode (tauri-driver, Windows only)

## M11 Guided first run
- [x] "Guided setup" section (opens on first launch, then never again unless chosen): get ready (adb) → connect → look at your TV → choose a goal → check (dry run) → apply, with a plain-language result, undo and next steps (5 UI tests)
- [x] Goals: gentle clean-up (Safe), standard clean-up (Balanced), clean-up + Studio launcher + current theme. Never disables the stock launcher, never touches DNS. Anything the TV's live checks don't plainly allow is left out and listed with its reason
- [x] E2E: the guide's exact selection logic runs on the simulated G09 in both modes (nothing protected picked, Home untouched)
- [ ] Watch a first-time user (you) go through it on a real TV and note where it confuses

## M10 Release + final review
- [x] adb: not bundled (Google SDK licence §3.4); Dashboard "Get adb from Google" downloads the official zip on request, with host allow-list, size caps, fixed-name extraction and a Windows signature check (3 tests + UI test)
- [x] Final security review: 8 findings fixed (docs/SECURITY.md)
- [x] CI: clippy on Windows; tag `v*` → draft GitHub release with checksums
- [x] SECURITY.md, TESTING.md, README final
- [ ] Code-signing certificate for the installer (optional; SmartScreen warns without it)
- [ ] Real-TV validation (TESTING.md checklist): the only thing standing between this and a public release
