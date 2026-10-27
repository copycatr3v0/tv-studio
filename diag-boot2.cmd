@echo off
set ADB=%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe
set D=-s 192.168.0.66:5555
set LOG=%~dp0diag-boot2.txt
set OUT=%~dp0diag-shots
set SVC=studio.tvcustomizer.home/com.gathin.tvhome.HomeKeyService
echo === %date% %time% > "%LOG%"
cd /d %~dp0launcher
call gradlew.bat testReleaseUnitTest assembleRelease -q >> "%LOG%" 2>&1
echo gradle=%errorlevel% >> "%LOG%"
if not "%errorlevel%"=="0" goto end
"%ADB%" connect 192.168.0.66:5555 >> "%LOG%" 2>&1
"%ADB%" %D% install -r app\build\outputs\apk\release\app-release.apk >> "%LOG%" 2>&1
"%ADB%" %D% shell appops set studio.tvcustomizer.home GET_USAGE_STATS default >> "%LOG%" 2>&1
"%ADB%" %D% shell appops set studio.tvcustomizer.home SYSTEM_ALERT_WINDOW default >> "%LOG%" 2>&1
"%ADB%" %D% shell settings delete secure enabled_accessibility_services >> "%LOG%" 2>&1
"%ADB%" %D% shell settings put secure enabled_accessibility_services %SVC% >> "%LOG%" 2>&1
"%ADB%" %D% shell settings put secure accessibility_enabled 1 >> "%LOG%" 2>&1
"%ADB%" %D% shell "appops get studio.tvcustomizer.home GET_USAGE_STATS; pm list packages -d | grep launcherx" >> "%LOG%" 2>&1
echo --- reboot >> "%LOG%"
"%ADB%" %D% reboot
timeout /t 30 >nul
:wait
"%ADB%" connect 192.168.0.66:5555 >nul 2>&1
"%ADB%" %D% shell getprop sys.boot_completed 2>nul | find "1" >nul
if errorlevel 1 (timeout /t 2 >nul & goto wait)
for /L %%i in (1,1,12) do (
  "%ADB%" %D% shell "cut -d' ' -f1 /proc/uptime; dumpsys activity activities | grep -m1 ResumedActivity" >> "%LOG%" 2>&1
  if %%i==2 "%ADB%" %D% exec-out screencap -p > "%OUT%\h1-boot.png"
  if %%i==6 "%ADB%" %D% exec-out screencap -p > "%OUT%\h2-boot.png"
  timeout /t 2 >nul
)
"%ADB%" %D% shell "appops get studio.tvcustomizer.home GET_USAGE_STATS; dumpsys usagestats | grep -c ACTIVITY_RESUMED" >> "%LOG%" 2>&1
"%ADB%" %D% shell "logcat -d -s TvsHome; logcat -d | grep -iE 'Background activity|START u0.*tvcustomizer' | grep -v FGS | tail -8" >> "%LOG%" 2>&1
:end
echo DONE >> "%LOG%"
