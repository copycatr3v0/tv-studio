@echo off
set ADB=%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe
set D=-s 192.168.0.66:5555
set LOG=%~dp0diag-home.txt
set OUT=%~dp0diag-shots
set U=content://studio.tvcustomizer.home.theme
set SVC=studio.tvcustomizer.home/com.gathin.tvhome.HomeKeyService
echo === %date% %time% > "%LOG%"
cd /d %~dp0launcher
call gradlew.bat testReleaseUnitTest assembleRelease -q >> "%LOG%" 2>&1
echo gradle=%errorlevel% >> "%LOG%"
if not "%errorlevel%"=="0" goto end
"%ADB%" connect 192.168.0.66:5555 >> "%LOG%" 2>&1
"%ADB%" %D% install -r app\build\outputs\apk\release\app-release.apk >> "%LOG%" 2>&1
call :apply
"%ADB%" %D% shell input keyevent KEYCODE_MENU
timeout /t 3 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\c1-settings.png"
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_RIGHT
timeout /t 1 >nul
"%ADB%" %D% shell input keyevent KEYCODE_BACK
timeout /t 6 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\c2-classic.png"
"%ADB%" %D% shell input keyevent KEYCODE_MENU
timeout /t 3 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\c3-settings-classic.png"
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
timeout /t 1 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\c4-middle.png"
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
"%ADB%" %D% shell input keyevent KEYCODE_DPAD_DOWN
timeout /t 1 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\c5-bottom.png"
"%ADB%" %D% shell input keyevent KEYCODE_BACK
timeout /t 2 >nul
call :apply
"%ADB%" %D% exec-out screencap -p > "%OUT%\c6-studio-again.png"
goto end
:apply
"%ADB%" %D% push "%OUT%\tv-main.json" /data/local/tmp/tvs-layout.json >> "%LOG%" 2>&1
"%ADB%" %D% push "%OUT%\fam-kids.json" /data/local/tmp/tvs-kids.json >> "%LOG%" 2>&1
"%ADB%" %D% shell "content write --uri %U%/profiles/kids.json < /data/local/tmp/tvs-kids.json; content write --uri %U%/layout.json < /data/local/tmp/tvs-layout.json; echo f-%RANDOM% > /data/local/tmp/tvs-rev; content write --uri %U%/theme_revision < /data/local/tmp/tvs-rev; rm /data/local/tmp/tvs-layout.json /data/local/tmp/tvs-kids.json /data/local/tmp/tvs-rev" >> "%LOG%" 2>&1
"%ADB%" %D% shell am force-stop studio.tvcustomizer.home
"%ADB%" %D% shell settings delete secure enabled_accessibility_services >> "%LOG%" 2>&1
"%ADB%" %D% shell settings put secure enabled_accessibility_services %SVC% >> "%LOG%" 2>&1
"%ADB%" %D% shell settings put secure accessibility_enabled 1 >> "%LOG%" 2>&1
timeout /t 2 >nul
"%ADB%" %D% shell am start -a android.settings.SETTINGS >nul 2>&1
timeout /t 2 >nul
"%ADB%" %D% shell input keyevent KEYCODE_HOME
timeout /t 10 >nul
exit /b
:end
echo DONE >> "%LOG%"
