# SPEC.md — Android TV Customizer Studio

## Decisions (2026-10-26)

| Topic | Decision | Source |
|---|---|---|
| Launcher | Fork CleanYourTV launcher (MIT) as `studio.tvcustomizer.home`, schema-compatible | User |
| Devices | Android TV **and** Google TV, all Android-based sets; Fire OS, Tizen and webOS are not supported | User |
| Repo | `C:\dev\tv-studio`, private GitHub repo; planning docs mirrored in OneDrive `Documents\tv` | User |
| Builds | Local Windows dev loop + GitHub Actions `windows-latest` for release builds | User |
| Min Android | Mode A: Android 7+ fully, 5–6 read-only plus disable with a warning. Mode B: Android 8.0+ (API 26) | Default |
| ADB | Bundled `adb.exe` behind a `Transport` trait; legacy 5555 and wireless pairing both supported | Default |
| Default mode | Beginner: Safe debloat preset, curated themes, no key helpers | Default |
| Streaming / voice / cast / updates | Never pre-selected. They need a per-item explicit confirmation. Firmware updater stays enabled unless the user insists. | Default |
| Debloat modes | Safe (KB-evidenced telemetry/ads only) · Balanced (+ preloads and duplicates) · Advanced inspection (any non-blocked package, each one confirmed) | Default |
| Privacy | Evidence-graded audit. Private DNS opt-in with a router-DNS warning. | Default |
| Performance | Baseline → deploy → re-run on the same device. Show median and IQR, never a promise. | Default |
| Telemetry | None. Optional update check, off by default. | Default |
| Packaging | NSIS installer + portable EXE, code-signing optional (certificate is user-provided) | Default |
| Launcher network | No INTERNET permission. Widgets are local only (clock, date, stats, network status). | Default |

## Out of scope (v1)
Root, flashing, bootloader, uninstalling system packages, `pm hide` and `pm block`, Fire TV Home takeover,
cloud sync, multi-user Google TV profiles beyond user 0 (detected and warned).

## User journey
Exactly the 13 steps in the engineering prompt §1. Every pre-Apply choice is serialized as a versioned
`DeploymentProfile { device_fingerprint, mode, theme, debloat[], settings[], launcher_opts, created, version }`.
