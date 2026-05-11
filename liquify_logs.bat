@echo off
title Liquify Logcat - FPS + All Logs

echo Clearing log buffer...
adb -s 9d5e705b logcat -c

echo Monitoring started. Press Ctrl+C to stop.
echo ------------------------------------------------

adb -s 9d5e705b logcat -v time | findstr /i "FPS liquify AndroidRuntime FATAL CRASH Exception"