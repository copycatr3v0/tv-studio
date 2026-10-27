# One command: build the TV launcher, bundle it into the Windows app, build the app.
# Usage (from C:\dev\tv-studio):  pnpm release
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Push-Location "$root\launcher"
.\gradlew.bat testReleaseUnitTest assembleRelease
if ($LASTEXITCODE -ne 0) { Pop-Location; throw "Launcher build failed" }
Pop-Location
$dst = "$root\src-tauri\resources\launcher"
New-Item -ItemType Directory -Force $dst | Out-Null
Copy-Item "$root\launcher\app\build\outputs\apk\release\app-release.apk" "$dst\studio-home.apk" -Force
Write-Host "Bundled launcher -> $dst\studio-home.apk"
Push-Location $root
pnpm tauri build
Pop-Location
Write-Host "Done: $root\target\release\tv-studio.exe"
