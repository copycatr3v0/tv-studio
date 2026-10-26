# PLAN.md — milestones, dependencies, acceptance

| # | Milestone | Depends on | Acceptance criteria (each needs passing tests) | Suggested model |
|---|---|---|---|---|
| M1 | Research | — | RESEARCH.md ✅ | Opus |
| M2 | Spec + architecture | M1 | SPEC, ARCHITECTURE, CAPABILITY_MATRIX, PLAN, TASKS approved ✅ (pending your review) | Opus / Fable |
| M3 | Windows shell | M2 | `pnpm tauri dev` opens the 15-section nav, dark UI, SQLite migrations run, settings persist, Beginner/Advanced toggle, CI builds the portable EXE on windows-latest | Opus (backend), Sonnet (UI) |
| M4 | ADB connectivity | M3 | MockTransport suite: multi-device, unauthorized, wrong port, pairing fail, reconnect. Real device: wizard connects over 5555 **and** via pairing; capabilities panel shows real values | Opus |
| M5 | Inventory + safety + snapshot | M4 | KB imported (317 lists). Every guard from RESEARCH §2 has fixtures ported from CleanYourTV tests. Snapshot is first-wins. Disable/restore round-trip on mock and on a real TV. | Opus |
| M6 | Launcher fork + theme system | M3 | Fork builds with lint at 0 errors, Gradle tests pass, APK < 1 MB. Theme schema v1 + CleanYourTV import. Editor preview matches the TV screenshot for 3 reference themes. Push via ThemeProvider verified on Android 9 and 12+. | Opus (launcher), Sonnet (editor UI) |
| M7 | Automation engine | M5, M6 | Dry-run plan. Journal re-entry after a kill at every step. Reboot mid-deploy recovers. Home handover verified with `mFocusedApp`. Failure halts dependents. Rollback restores the exact snapshot. | Opus |
| M8 | Benchmarks + drift | M7 | Before/after comparison with spread. Drift detects re-enabled packages, moved Home, missing launcher, and new preloads; repair is dry-run first. | Opus |
| M9 | Integration | M8 | E2E on mock (all §15 cases) + one Google TV + one Android TV, documented as real-device or simulated | Opus + Sonnet (tests) |
| M10 | Release | M9 | NSIS + portable EXE from CI, licences/NOTICE, README, RECOVERY, SECURITY, BENCHMARKS, TESTING complete; installs on clean Win10/11 | Opus, Haiku (docs) |

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Home handover silently fails (Google TV) | High | High | `mFocusedApp` proof; never disable the stock launcher before proof; auto-revert |
| setupwraith deletes Wi-Fi → TV unreachable | Med | Critical | Hard guard; manual re-join flow |
| Disabled input driver → permanent No Signal | Med | High | TV_INPUT_HARDWARE hard block |
| Factory package started | Low | Critical | Never-start list; inventory is read-only |
| OEM blocks APK install / HOME | Med | Med | Detect early, offer Mode A only |
| Key helper breaks volume repeat | Med | Low | Off by default; explicit trade-off text |
| adb.exe redistribution terms | Low | Med | Review before M10; fall back to a first-run download from Google or a pure-Rust transport |
| Wallpaper/font licence contamination | Med | Med | Original assets only; OFL fonts with licence files |
| OneDrive sync of build dirs | — | — | Repo moved to C:\dev ✅ |
