@echo off
set "PATH=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin;%USERPROFILE%\.cargo\bin;%PATH%"
echo Building Release APK (aarch64)...
cargo apk build --release --lib --target aarch64-linux-android --target-dir target-apk
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [ERROR] Build failed!
    pause
    exit /b %ERRORLEVEL%
)
echo.
echo [SUCCESS] Build finished!
echo APK location: target-apk\release\apk\main.apk
pause
