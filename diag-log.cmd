@echo off
set ADB=%LOCALAPPDATA%\Android\Sdk\platform-tools\adb.exe
set D=-s 192.168.0.66:5555
set LOG=%~dp0diag-log.txt
echo === %date% %time% > "%LOG%"
"%ADB%" connect 192.168.0.66:5555 >> "%LOG%" 2>&1
"%ADB%" %D% shell "logcat -d | grep -iE 'Background activity|BootHome|blocked|gathin.tvhome.MainActivity|Abort' | grep -v FGS | tail -20" >> "%LOG%" 2>&1
"%ADB%" %D% shell "dumpsys activity broadcasts | grep -iE -A3 'BOOT_COMPLETED' | head -20" >> "%LOG%" 2>&1
"%ADB%" %D% shell "logcat -d | head -2; logcat -d | tail -2" >> "%LOG%" 2>&1
echo DONE >> "%LOG%"
