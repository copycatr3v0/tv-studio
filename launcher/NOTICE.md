# Studio launcher

A fork of the CleanYourTV reference launcher (https://github.com/Gathin23/cleanyourtv, commit f592b6d,
MIT, (c) 2026 Gathin). See LICENSE-CleanYourTV.

## What changed in the fork
- Application ID `studio.tvcustomizer.home` (CleanYourTV's own opt-in `launcherApplicationId` mechanism,
  already used on Xiaomi and Coocaa firmware). The Java namespace stays `com.gathin.tvhome`, so the
  code, its unit tests and its module-card contract (`com.gathin.tvhome.category.DOCK_CARD`) are unchanged.
  That keeps CleanYourTV add-ons compatible.
- Version `3.0-studio.1`.
- Release signing from `STUDIO_KEYSTORE` in CI; local builds use the debug key.
- The PowerShell/sh install scripts are not included: the desktop app installs, configures and verifies the launcher.

## Rules kept from upstream
No INTERNET permission, no analytics, minSdk 26, no new dependencies, APK under 576 KB.

## Build
```powershell
cd launcher
.\gradlew.bat --no-daemon testReleaseUnitTest lintRelease assembleRelease
# -> app\build\outputs\apk\release\app-release.apk
```
Needs JDK 17 and Android SDK platform 34 + build-tools 34.0.0 (Android Studio installs both).
