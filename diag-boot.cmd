@echo off
set ADB=%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe
set D=-s 192.168.0.66:5555
set LOG=%~dp0diag-boot.txt
set OUT=%~dp0diag-shots
echo === %date% %time% > "%LOG%"
"%ADB%" connect 192.168.0.66:5555 >> "%LOG%" 2>&1
echo --- before >> "%LOG%"
"%ADB%" %D% shell "grep -E 'MemTotal|MemAvailable' /proc/meminfo; settings get secure enabled_accessibility_services; cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.HOME | tail -1; dumpsys activity activities | grep -E 'ResumedActivity' | head -2" >> "%LOG%" 2>&1
echo --- memory (PSS KB) >> "%LOG%"
"%ADB%" %D% shell "for p in com.google.android.apps.tv.launcherx studio.tvcustomizer.home com.google.android.apps.tv.dreamx; do echo $p; dumpsys meminfo $p | grep -E 'TOTAL PSS|TOTAL:' | head -1; done; dumpsys activity processes | grep -E 'launcherx|tvcustomizer' | grep -E 'ProcessRecord|oom' | head -6" >> "%LOG%" 2>&1
echo --- standby test >> "%LOG%"
"%ADB%" %D% shell input keyevent KEYCODE_SLEEP
timeout /t 20 >nul
"%ADB%" %D% shell input keyevent KEYCODE_WAKEUP
timeout /t 10 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\b1-wake.png"
"%ADB%" %D% shell "dumpsys activity activities | grep -E 'ResumedActivity' | head -2" >> "%LOG%" 2>&1
echo --- reboot test >> "%LOG%"
"%ADB%" %D% reboot
timeout /t 60 >nul
:wait
"%ADB%" connect 192.168.0.66:5555 >nul 2>&1
"%ADB%" %D% shell getprop sys.boot_completed 2>nul | find "1" >nul
if errorlevel 1 (timeout /t 5 >nul & goto wait)
timeout /t 25 >nul
"%ADB%" %D% exec-out screencap -p > "%OUT%\b2-boot.png"
"%ADB%" %D% shell "cat /proc/uptime; settings get secure enabled_accessibility_services; settings get secure accessibility_enabled; dumpsys activity activities | grep -E 'ResumedActivity' | head -2; dumpsys accessibility | grep -iE 'Bound services|Enabled services' | head -3; logcat -d | grep -iE 'HomeKeyService|ProbeActivity|tvcustomizer' | tail -15" >> "%LOG%" 2>&1
echo --- memory after boot >> "%LOG%"
"%ADB%" %D% shell "for p in com.google.android.apps.tv.launcherx studio.tvcustomizer.home; do echo $p; dumpsys meminfo $p | grep -E 'TOTAL PSS|TOTAL:' | head -1; done" >> "%LOG%" 2>&1
echo DONE >> "%LOG%"
