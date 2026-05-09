@echo off
set "PATH=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;%USERPROFILE%\.cargo\bin;%PATH%"

echo 1. Building Release APK...
cargo apk build --release --lib --target aarch64-linux-android --target-dir target-apk
if %ERRORLEVEL% NEQ 0 goto :error

echo 2. Installing APK to device 9d5e705b...
adb -s 9d5e705b install -r target-apk\release\apk\main.apk
if %ERRORLEVEL% NEQ 0 goto :error

echo 3. Starting application...
adb -s 9d5e705b shell am start -n com.test.liquify/android.app.NativeActivity

echo.
echo [DONE] Application started successfully!
pause
exit /b 0

:error
echo.
echo [ERROR] An error occurred during the build/install process.
pause
exit /b 1
