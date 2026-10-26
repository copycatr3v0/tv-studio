# RESEARCH.md — CleanYourTV reference analysis

Stage 1 of Android TV Customizer Studio. Source studied: `github.com/Gathin23/cleanyourtv`,
commit `f592b6d` (2026-10-26). MIT licence, © 2026 Gathin.

## 1. What the repository actually is

| Part | Size | What it does |
|---|---|---|
| `PROMPT.md` / `AGENTS.md` | ~400 lines | The procedure an AI agent follows on a real TV. Most of the safety knowledge lives here as prose. |
| `tools/` | 28 scripts, ~6.6k lines (Python + PowerShell/sh twins) | ADB client (`tv.py`, uses `adb_shell`, no adb binary), debloat with guards (`disable.*`), exact undo (`recovery.py`), drift check (`check.py`), privacy audit, benchmark, traces, theme push. |
| `launcher/` | 65 Java classes, ~16k lines, `com.gathin.tvhome` v3.0 | Native TV launcher. minSdk 26, target/compile 34. Only deps: RecyclerView 1.3.2, DynamicAnimation 1.0.0. No INTERNET. APK size gate 576 KB. Strong unit tests (grid navigation, ordering, remote keys, cards). |
| `themes/` | 13 themes | `config.json` + `screenshot.jpg` + optional wallpapers. About 100 documented keys. |
| `examples/` | 317 tested TV folders | `debloat.txt` (commented package lists with per-package reasons) + `tv.json`. **This is the most valuable asset: a real-device knowledge base.** |
| `docs/` | TROUBLESHOOTING (1.2k lines), 40+ device notes, workflows | Edge cases learned the hard way, with issue numbers. |
| `addons/` | 4 optional apps | Module-card apps (category `com.gathin.tvhome.category.DOCK_CARD`) that never touch the launcher. |
| `tests/` | ~150 Python unittest files | Includes fake-TV tests for guards and tools. |

## 2. Proven safety mechanisms to reproduce (all verified in source)

These move into our Rust `safety` and `planner` modules as **non-overridable code**, not prose.

1. **Core package block** (`recovery.py` `CORE_RE` / `CORE_OK`): `android`, systemui, shell, phone,
   settings, tv.settings, permissioncontroller, packageinstaller, ext.services/shared, modulemetadata
   (both `com.android.` and `com.google.android.`), `overlay.modules*`, `com.android.providers.*` except
   calendar/contacts/userdictionary/blockednumber. Origin: issue #884, a Mi TV Stick that never booted again.
2. **Package-name validation** before anything reaches the TV shell:
   `^[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z0-9_]+)+$`. All names go into shell commands, so this prevents injection.
3. **Home guard**: `cmd package resolve-activity --brief … HOME`, fall back to `dumpsys activity`
   `mHomeProcess` before Android 7. Never disable the Home owner unless another *usable* launcher remains.
   Not launchers: setupwraith, tv.settings, settings, provision, any `*/FallbackHome`, factory-test apps.
   If nothing answers clearly, keep the known stock launchers.
4. **Home verification**: a successful `set-home-activity` proves nothing (some Google TV builds print
   `Success` and change nothing). The proof is `dumpsys activity activities | grep mFocusedApp` after a real HOME key press.
5. **setupwraith Wi-Fi ownership**: `dumpsys wifi | grep -o 'cname=[^ ]*'`. If setupwraith owns the saved
   network, disabling it deletes the network and the TV drops off Wi-Fi and ADB. If the check can't run, keep it enabled.
6. **TV input hardware guard** (#321): any package requesting `android.permission.TV_INPUT_HARDWARE`
   drives HDMI/AV/tuner. Disabling it gives a permanent "No Signal" that only a full restore and a cold boot fix.
7. **Intent-handler guard**: keep any package answering `VIEW vnd.android.cursor.item/channel` (HDMI input
   tiles) or `android.settings.SETTINGS` (otherwise the way back to Developer options is lost).
8. **Factory packages are never started**, not even to look (`*hkcfac*`, `*fac.menu*`, `com.at.factory.*`,
   `mediatek.factorymenu.ui`). On CVTE/Haier boards this sets a factory flag the shell cannot undo.
   Read-only `dumpsys package` is the only allowed way to look at them.
9. **CVTE boards** (any `com.cvte.*`): leave the stock launcher enabled, set Home, reboot once, confirm,
   and never enable a key helper in the same pass (#741, a JVC stuck at boot with ADB gone).
10. **Read back, don't trust exit codes**: after disabling, check with `pm list packages -e --user 0`
    (per user, since Android 14 merges users). If *every* disable is refused, stop: the firmware forbids it
    (Fire OS 5). `pm hide` can report success while changing nothing.
11. **One `pm` per shell call**: chained commands can silently stop after the first one.
12. **Exact undo** (`recovery.py`): a record is written *before* the first change. The first record wins
    and later runs only add new names, so a retry after a half-applied run never records broken state as
    the original. Restore replays *recorded* states (packages that were already disabled stay disabled), the
    original Home component, and the original animation scales (`null` → `settings delete`).
13. **Drift** (`check.py`): compares the TV with the *target list* (not the record), detects re-enabled
    packages and a moved Home, re-runs every safety guard before `--fix`, and never writes a recovery record.
    Exit codes 0/1/2 are clearly separated.
14. **ADB exposure**: warn when `ro.adb.secure=0` (any LAN device gets a shell on 5555). Closing is verified
    by "Connection refused" (`adb-closed.py`), not by a timeout.
15. **Install quirks**: Android 14 `adb install` waits for an on-screen confirmation, so the tools push and
    run `pm install -r` instead. `INSTALL_FAILED_VERIFICATION_FAILURE` on some China-market sets, HOME
    rejected without the OEM signature, and SDK < 26 all mean the launcher half is impossible: say so and stop.
16. **Display overrides**: read `wm size`, `wm density` and `display_density_forced` before and after;
    an "enlarged" screen is often a leftover override, not a launcher bug.
17. **Privacy honesty** (`privacy.py`): on system-uid firmware about 50 packages "have" RECORD_AUDIO.
    Evidence levels: OEM-nominated / known catalogue entry / name match only / AppOps mic history. Never "is listening".

## 3. Theme delivery (no rebuild)

- The launcher reads `/sdcard/Android/data/<pkg>/files/config.json` as an overlay on its built-in
  `config.xml`. Unknown or badly typed keys are ignored, so the format can be extended safely.
- Transfer order: stage in `/data/local/tmp`, then **write through `ThemeProvider` first**
  (`content write --uri content://<pkg>.theme/<file>`), and use `cp` only as a fallback.
  Reason: on Android 14 `cp` "succeeds", but the file then belongs to uid 2000 and the launcher gets EACCES.
  Android 8/9 have no `content write` and print "Unsupported operation" with exit 0, which must be treated as failure.
- The provider is protected by `WRITE_SECURE_SETTINGS`: the adb shell has it, normal apps don't. This is a good, simple trust boundary.
- `theme_revision` (random GUID) makes the launcher drop on-TV choices after a push. `theme_restart.py`
  restarts Home and repairs accessibility helpers.
- Wallpapers: names `^[A-Za-z0-9_.-]+$`, larger files go through base64 chunks with an md5 check
  (`b64push.py`), and `wallpaper_true_color` is off by default (RGB_565, 4 MB at 1080p) for 1 GB TVs.

## 4. Reuse decisions

| Component | Decision | Why |
|---|---|---|
| Launcher (Java) | **Adapt (fork)** under our own package id, theme schema kept compatible | 16k lines tuned on 300+ TVs. Rewriting would lose years of firmware workarounds. MIT allows it. Their policy routes forks to the fork's own repo plus a SHOWCASE row, which matches what we're doing. |
| `config.json` key set | **Reuse as schema v1**, define it in Zod + Rust serde, extend with namespaced keys | Gives import compatibility for free. |
| Safety guards, recovery, drift | **Reimplement in Rust** from the logic above, with their test cases ported as fixtures | Users must not need Python. These become typed, testable modules. |
| `tv.py` / `adb_shell` transport | **Replace** | See §5. |
| `examples/*/debloat.txt` | **Import as data** into a bundled package knowledge base (name, reason, category, device evidence) | Gives device-specific risk evidence instead of name guessing. Comments are MIT text and must be attributed. |
| `privacy.py` catalogue | **Port the data and evidence model** | |
| `bench.py` methodology | **Reimplement** (framestats completion-time matching, `am start -W`, PSS, idle CPU, series and spread) | |
| Themes and wallpapers | **Do not reuse the assets.** Write our own 15 themes. | Wallpapers are CC BY / CC BY-SA from Wikimedia, not MIT. Outfit font is OFL, which can be bundled with its licence file. |
| Add-ons | **Keep the module-card contract** for compatibility; build our own modules later | |

## 5. Architecture risks and choices found

- **ADB transport.** Options: (a) bundle Google's `adb.exe` (platform-tools) and drive it as a subprocess:
  most compatible, has `pair` and mDNS, and scrcpy does the same on Windows. (b) A pure-Rust client: no
  binary, but pairing (TLS + SPAKE2) and mDNS are less mature. **Default: (a) behind a `Transport` trait**,
  with a mock transport for tests and (b) possible later.
- **Prebuilt APK.** Their repo forbids prebuilt APKs. Ours ships one inside the EXE, built reproducibly in CI and signed with our own key.
- **Accessibility key helpers** (HomeKeyService / InputKeyService) break key repeat on some TCL G09 builds.
  They must be off by default, an explicit choice, and verified by the user pressing the real remote button.
- **What ADB alone cannot do**: it cannot restyle the stock Google TV home, change recommendation rows
  inside launcherx, or read most OEM settings. The capability matrix must say this plainly.
- **Repo location**: the chosen folder is inside OneDrive. `node_modules`, `target/` and Gradle caches
  (hundreds of thousands of files) will choke OneDrive sync. Recommend a non-synced path plus git.
