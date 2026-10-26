# CAPABILITY_MATRIX.md

Legend: **A** = stock ADB (Mode A, no APK) · **L** = needs Studio Launcher (Mode B) · **M** = manual step on the TV · ✗ = not possible without root (out of scope)

| Capability | Source | Min Android | Notes |
|---|---|---|---|
| Read model, SDK, fingerprint, RAM, storage | A | 5 | getprop, /proc/meminfo, df |
| List packages, states, permissions | A | 5 | per user 0 |
| Disable / re-enable packages | A | 5 (`pm disable-user`) | Some firmware refuses (Fire OS 5). Verified by readback. |
| Query Home owner & HOME claimants | A | 7 (`cmd package`) | `dumpsys activity` fallback before 7 |
| Set default Home | A | 7 | Must be verified with `mFocusedApp`; can silently fail on Google TV |
| Animation scales | A | 5 | `settings put global …` |
| Private DNS (ad-blocking) | A | 9 | Opt-in; bypasses router DNS rules |
| Screen timeout, screensaver component | A | 5 | `settings put secure/system` (OEM may ignore) |
| Screenshots for verification | A | 5 | Empty on DRM/HDMI surfaces |
| Benchmarks (framestats, cold start, PSS) | A | 7 | framestats needs API 23+ |
| Install APK | A | 5 | Android 14: push + `pm install -r`. Some China firmware blocks it. |
| Restyle the **stock** Google TV / PatchWall home | ✗ | — | ADB cannot change launcherx layout, rows or ads. Mode A can only disable recommendation/ads packages. |
| Custom wallpaper, grid, dock, cards, fonts, focus style, clock, widgets | L | 8.0 (API 26) | Theme pushed as JSON, no rebuild |
| Hidden apps, favorites, app library, top-bar apps | L | 8.0 | |
| Inputs menu (HDMI/AV/Live TV) | L | 8.0 | TvInputManager, vendor fallbacks |
| Remote key remapping (Netflix/Input/Settings buttons) | L | 8.0 | Accessibility helper, opt-in. Can break key repeat on some TCL. The user must press the real button to confirm. |
| Boot to Home / Live TV | L | 8.0 | One or the other, never both |
| Enable Developer options / pair | M | — | Wizard guides the user |
| Turn off network debugging afterwards | M | — | Wizard verifies the port is closed |
| Re-join Wi-Fi owned by setupwraith | M | — | Required before setupwraith can be disabled |
