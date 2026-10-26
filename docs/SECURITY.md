# Security

This document covers what the app is allowed to do to a TV and to your PC, how it enforces that, what the
final review checked, and how to report a problem.

## Trust boundaries

| Boundary | What crosses it | Enforcement |
|---|---|---|
| UI (WebView) → Rust | Typed Tauri commands only | Only `core:default` permissions are granted (no shell, no fs, no http plugins). A strict CSP blocks remote scripts and objects. Every argument is validated in Rust (`Serial`, `PackageName`, `Component` newtypes, size caps on imported files). |
| Rust → TV shell | `adb -s <serial> shell …` | `ShellCommand` accepts only fixed tokens written in the source, validated identifiers, or runtime tokens checked against shell metacharacters, glob characters, whitespace and control characters. A stdin redirect is accepted only from `/data/local/tmp/tvs-[A-Za-z0-9_.-]+`. |
| Rust → TV files | `adb push` | Refused by the transport unless the destination is under `/data/local/tmp/tvs-` (the real and simulated transports both enforce this). |
| TV → Rust | Command output, package lists, dumpsys | Treated as data. Package and component names are re-validated after parsing. Results are confirmed by reading the TV back, never by an exit code alone. |
| Internet → PC | Google's platform-tools zip (only when you click "Get adb from Google") | HTTPS only. The final address after redirects must be a Google download host. The download is capped at 64 MB. Only `adb.exe`, `AdbWinApi.dll` and `AdbWinUsbApi.dll` are extracted, under fixed names, and each must be a Windows executable. A signature that Windows reports as broken (`HashMismatch` / `NotTrusted`) is removed. Nothing runs during setup. |
| Files you import → Rust | Theme JSON, profiles, wallpapers | Theme ≤ 256 KB and profile ≤ 1 MB, with unknown profile fields rejected. Wallpapers are decoded with limits (≤ 8192 px per side, ≤ 256 MB of memory, ≤ 30 MB file), then re-encoded as a fresh JPEG ≤ 1920×1080, so no original bytes reach the TV. |
| Other apps on the TV → Studio launcher | Theme provider | Requires `WRITE_SECURE_SETTINGS`, which the adb shell has and ordinary apps cannot get. Only the allow-listed theme file names are accepted. |

## What the app will never do

- Root, exploit, flash firmware, unlock the bootloader, or uninstall a system package (only `pm disable-user --user 0`, which is reversible).
- Disable a protected package. These are always blocked and there is no override:
  - core Android, Settings, providers, the permission and package installers, mainline modules;
  - WebView (AOSP/Google), the network stack, tethering, Wi-Fi resources, KeyChain, the certificate installer, input devices, external storage and the fused location provider.
- Disable these on a TV where they are in use (also blocked, no override):
  - the only working launcher;
  - the setup wizard when it owns the Wi-Fi network;
  - the TV-input hardware package;
  - the Settings or input handlers.
- Start a factory or service-menu package, even just to look at it.
- Run a downloaded script. Theme files are data; adb comes from Google and is only run when needed, like a manually installed copy.
- Run two changes to a TV at once. Every command that changes a TV takes a single app-wide lock, so Revert can't interleave with Apply Everything.
- Retry a non-idempotent step blindly. After a dropped connection, the step is re-checked on the TV and then run at most once more.
- Claim a microphone permission means recording, or promise any TV will become faster.

## Data the app keeps

Everything stays on your PC in `%APPDATA%\studio.tvcustomizer.desktop\studio.db`:

- settings;
- each TV's build identity, model and last address;
- the write-once original-state baseline and per-run snapshots;
- deployment journals, saved profiles and themes, benchmark results.

There is no telemetry, and "check for updates" is off by default. The diagnostics bundle replaces the TV's
address/serial, IPv4/IPv6 and MAC addresses before you see it, and you choose whether to share it.

Demo mode writes to the same database under a separate simulated identity (`…:user/demo-keys`), which can
never match a real TV.

## Known limitations

- **Unsigned installer.** Until a code-signing certificate is added, Windows SmartScreen will warn on the installer. Verify `SHA256SUMS.txt` from the GitHub release.
- **Launcher signing.** The Studio launcher APK is signed with the release key when CI has the `STUDIO_KEYSTORE_*` secrets; local builds use whatever key Gradle is configured with. An update signed with a different key can't replace an installed launcher. Uninstall it first, which "Revert to original" does safely.
- **Location permission.** When the Studio launcher is installed, the app grants it `ACCESS_FINE_LOCATION`, inherited from CleanYourTV, so the network pill can show the Wi-Fi name on Android 10+. A refusal changes nothing else, and uninstalling the launcher removes the permission.
- **Platform-tools trust.** adb's integrity rests on HTTPS to Google's download host plus Windows' signature check. The app doesn't pin a hash, because Google replaces the zip with each release.
- **Simulator only.** Everything here was verified against the simulated TV. See TESTING.md for what still needs real hardware.

## Final review (M10) findings

| # | Finding | Severity | Fix |
|---|---|---|---|
| 1 | A tab or other whitespace in a runtime shell token would split it into extra arguments on the TV; `[` `]` allowed globbing | Medium | Runtime tokens reject all whitespace and control characters; `[` `]` added to the forbidden set (tests) |
| 2 | `adb push` destinations were checked only by callers | Low | The transport itself refuses anything outside the staging area |
| 3 | No lock across changing commands: Revert could run during Apply Everything, and a benchmark could clear a pending cancel | Medium | App-wide change lock with a clear "X is still running" message (test) |
| 4 | A small image claiming huge dimensions could exhaust memory | Medium | Decoder limits (test) |
| 5 | WebView, network stack, KeyChain, cert installer and similar weren't hard-protected | Medium | Added to the core block list; vendor WebViews need confirmation instead (tests) |
| 6 | Diagnostics redaction missed IPv6 and MAC addresses | Low | Added (test) |
| 7 | A mistyped package name in an imported profile was skipped silently (found in M9) | Low | Blocks the plan |
| 8 | The CSP had no explicit `connect-src` / `object-src` / `base-uri` | Low | Tightened |

## Reporting a vulnerability

Open a private security advisory on the GitHub repository (Security → Report a vulnerability). Please include
the app version, the TV model and firmware, and the redacted diagnostics bundle if it helps.
