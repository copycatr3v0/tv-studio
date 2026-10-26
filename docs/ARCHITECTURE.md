# ARCHITECTURE.md — Android TV Customizer Studio

Status: Stage 2 draft, 2026-10-26. Built on [RESEARCH.md](RESEARCH.md).

## 1. Shape

```
┌──────────────────────── Windows EXE (Tauri 2) ─────────────────────────┐
│ React + TS strict + Tailwind UI                                        │
│   Wizard · Inspector · Theme Studio (16:9 canvas) · Debloat · Deploy   │
│        │  typed IPC (tauri::command + generated TS bindings, specta)    │
│ Rust core (workspace crates)                                           │
│   tvs-transport ─ tvs-device ─ tvs-inventory ─ tvs-safety ─ tvs-theme  │
│   tvs-planner ─ tvs-executor ─ tvs-verify ─ tvs-recovery ─ tvs-bench   │
│   tvs-store (SQLite)        tvs-kb (bundled package knowledge base)    │
│        │                                                               │
│   bundled: adb.exe + AdbWinApi.dll · launcher.apk · themes/ · kb.sqlite│
└────────┼───────────────────────────────────────────────────────────────┘
         │ ADB over TCP (5555 legacy) or TLS wireless debugging (pair + connect)
┌────────▼──────────────── Android TV / Google TV ───────────────────────┐
│ pm / cmd package / settings / dumpsys / content / am / input           │
│ Studio Launcher (forked com.gathin.tvhome → studio.tvcustomizer.home)  │
│   reads files/config.json via ThemeProvider (WRITE_SECURE_SETTINGS)    │
└────────────────────────────────────────────────────────────────────────┘
```

No TV-side manager app. The launcher is an optional deliverable that Mode B installs.

## 2. Rust crates (all independent, each with unit tests)

| Crate | Responsibility | Key types |
|---|---|---|
| `tvs-transport` | `trait Transport { shell, push, pull, install, reboot }`. `AdbExeTransport` wraps the bundled adb.exe with an explicit `-s <serial>` on every call. `MockTransport` takes scripted device states, failures, delays and reboots. Bounded timeouts. Never chains `pm` commands. | `Serial`, `ShellOutput{stdout,stderr,code,elapsed}` |
| `tvs-device` | Discovery (`adb mdns services`, manual IP, pairing), connection diagnostics, authorization state, capability probe (getprop, SDK, RAM/storage, Home, HOME claimants, `ro.adb.secure`, `cmd` and `content write` support, OEM family detection: CVTE, Xiaomi, TCL, Google TV) | `DeviceInfo`, `Capabilities`, `OemFamily` |
| `tvs-inventory` | `pm list packages -f -U --user 0` (enabled/disabled/system), labels, permissions, intent handlers, TV_INPUT_HARDWARE scan. All read-only. | `Package`, `PackageState` |
| `tvs-kb` | Bundled SQLite built at compile time from imported CleanYourTV `examples/` lists (MIT, attributed), the privacy catalogue and our own curated entries | `KbEntry{category, risk, evidence[], devices[]}` |
| `tvs-safety` | **Hard rules, no override in UI or config**: core regex, name validation, Home guard, setupwraith-Wi-Fi, input-hardware, intent handlers, factory never-start, CVTE rule. Returns `Verdict::{Allowed, NeedsConfirm(reason), Blocked(reason)}` | `Guard` trait, `GuardReport` |
| `tvs-theme` | Normalized design model. Schema v1 = CleanYourTV keys, plus `x_studio.*` extensions. Validation, CleanYourTV import, per-launcher-version support table | `Theme`, `SupportLevel` |
| `tvs-planner` | Turns a `DeploymentProfile` into an ordered `Plan` of `Step`s with dependencies, batches (≤5 packages), checkpoints and dry-run output | `Plan`, `Step{id, kind, preconds, verify, undo}` |
| `tvs-executor` | Runs the plan, writes a journal row before and after each step, makes idempotent re-entry decisions from the journal, retries only idempotent steps (bounded), stops dependents on failure | `Journal`, `StepOutcome` |
| `tvs-verify` | Reads actual state back: enabled packages per user, `mFocusedApp` after a HOME key, settings values, theme revision file, launcher alive | `Verification` |
| `tvs-recovery` | Snapshot (first-wins baseline per device), full, partial, theme-only and selective restore, drift detection and a repair plan | `Snapshot`, `DriftReport` |
| `tvs-bench` | gfxinfo framestats (completion-time matching), `am start -W`, return-to-Home, PSS, idle CPU, N series with median and IQR | `BenchRun`, `Comparison` |
| `tvs-store` | SQLite (rusqlite + refinery migrations): devices, snapshots, profiles (versioned), journals, bench runs | |

Rules that cross every crate:
- Every package name passes `tvs-safety::validate_name` before reaching a shell string.
- Commands are built by typed builders, never with `format!` from user input.
- The device serial is a newtype that every transport call requires.

## 3. Deployment state machine

```
Idle → Preflight(reconnect, identify serial+fingerprint, capabilities)
     → Snapshot(write baseline if none for this device; never overwrite)
     → Plan(validate, guards, dependency order) ──dry-run──▶ Report
     → [Launcher] Push APK → pm install -r → verify installed → am start → alive?
                  → set-home-activity → HOME key → mFocusedApp == ours? (else rollback launcher steps)
     → [Debloat] batches of ≤5: disable one by one → read back -e --user 0 → checkpoint
     → [Settings] put → get → compare
     → [Theme] stage → content write (fallback cp) → revision → restart Home → verify file+alive
     → [Stock launcher] disable only if ours verified as Home AND guard allows AND not CVTE
     → FinalVerify(Home, Settings opens, inputs handler present) → Journal close → Report
any failure → Halt(stop dependents) → offer: retry step (if idempotent) | rollback this run | keep
```

Reboot or Wi-Fi drop: the executor pauses, reconnects with backoff (max ~3 min), re-reads state, and
checks the journal to decide whether the step already took effect before it continues.

## 4. Theme pipeline (same model drives preview and TV)

`Theme` (Rust, serde) ⇄ Zod schema (generated from a single JSON Schema in `schema/theme.v1.json`)
→ React preview renderer (16:9 canvas, D-pad simulator) → `config.json` + assets → TV.
Each control declares `{ appliesTo: "launcher" | "android" | "manual", minLauncherVersion }`.
A control the target launcher doesn't support is shown as unsupported and is never reported as deployed.

## 5. Launcher fork

- Package `studio.tvcustomizer.home`, applicationId renamed, `LICENSE` + NOTICE keep © Gathin (MIT).
- Same minSdk 26 / target 34+, no INTERNET permission, same size discipline (target < 1 MB).
- Additions: `x_studio.*` keys (typography choices, extra card shapes, accent palette) behind flags that
  are off by default, plus a `version.json` the desktop app reads to build the support table.
- Built by Gradle in CI, signed with our release key, embedded in the EXE resources.

## 6. Security

- adb keys live in `%APPDATA%\TVStudio\adbkey`, ACL'd to the current user. Option to reuse `~/.android/adbkey`.
- No telemetry. No network calls from the desktop app except optional update checks (off by default).
- Imported themes are untrusted: JSON is schema-validated, image files are re-encoded on the desktop
  (strips payloads, resizes to 1920×1080 max), and file names are regex-restricted.
- End-of-session wizard: turn debugging off, revoke authorizations, and verify the port is closed ("Connection refused").

## 7. Packaging

Tauri bundler produces an NSIS installer (`.exe`) and a portable `.exe`. It bundles adb platform-tools
(redistribution under the Android SDK terms is reviewed in SECURITY.md/licences), the launcher APK and
`kb.sqlite`. WebView2 uses the evergreen bootstrapper.
