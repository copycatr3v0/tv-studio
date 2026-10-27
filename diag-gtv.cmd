@echo off
set ADB=%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe
set D=-s 192.168.0.66:5555
set LOG=%~dp0diag-gtv.txt
set OUT=%~dp0diag-shots
set G=com.google.android.apps.tv.launcherx
echo === %date% %time% > "%LOG%"
"%ADB%" connect 192.168.0.66:5555 >> "%LOG%" 2>&1
echo --- switch off >> "%LOG%"
"%ADB%" %D% shell pm disable-user --user 0 %G% >> "%LOG%" 2>&1
"%ADB%" %D% shell cmd role add-role-holder --user 0 android.app.role.HOME studio.tvcustomizer.home >> "%LOG%" 2>&1
"%ADB%" %D% shell cmd package set-home-activity --user 0 studio.tvcustomizer.home/com.gathin.tvhome.MainActivity >> "%LOG%" 2>&1
"%ADB%" %D% shell input keyevent KEYCODE_HOME
timeout /t 3 >nul
"%ADB%" %D% shell "cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME | tail -1; dumpsys activity activities | grep -E 'ResumedActivity' | head -1" >> "%LOG%" 2>&1
"%ADB%" %D% exec-out screencap -p > "%OUT%\g1-off.png"
echo --- reboot with google tv off >> "%LOG%"
"%ADB%" %D% reboot
timeout /t 45 >nul
:wait
"%ADB%" connect 192.168.0.66:5555 >nul 2>&1
"%ADB%" %D% shell getprop sys.boot_completed 2>nul | find "1" >nul
if errorlevel 1 (timeout /t 3 >nul & goto wait)
"%ADB%" %D% shell "cat /proc/uptime; dumpsys activity activities | grep -E 'ResumedActivity' | head -1" >> "%LOG%" 2>&1
"%ADB%" %D% exec-out screencap -p > "%OUT%\g2-boot-early.png"
timeout /t 20 >nul
"%ADB%" %D% shell "cat /proc/uptime; dumpsys activity activities | grep -E 'ResumedActivity' | head -1; grep MemAvailable /proc/meminfo; dumpsys activity processes | grep -c launcherx" >> "%LOG%" 2>&1
"%ADB%" %D% exec-out screencap -p > "%OUT%\g3-boot.png"
echo --- switch back on >> "%LOG%"
"%ADB%" %D% shell pm enable --user 0 %G% >> "%LOG%" 2>&1
"%ADB%" %D% shell cmd role remove-role-holder --user 0 android.app.role.HOME studio.tvcustomizer.home >> "%LOG%" 2>&1
"%ADB%" %D% shell input keyevent KEYCODE_HOME
timeout /t 4 >nul
"%ADB%" %D% shell "cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME | tail -1; dumpsys activity activities | grep -E 'ResumedActivity' | head -1; pm list packages -d | grep launcherx" >> "%LOG%" 2>&1
"%ADB%" %D% exec-out screencap -p > "%OUT%\g4-on.png"
echo DONE >> "%LOG%"
