@echo off
set "TOOLCHAIN_BIN=%USERPROFILE%\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin"
set "PATH=%TOOLCHAIN_BIN%;%USERPROFILE%\.cargo\bin;%PATH%"

echo --- Starting Build and Run --- > build_log.txt

echo 0. Ensuring target... >> build_log.txt
rustup target add aarch64-linux-android >> build_log.txt 2>&1

echo 1. Building and Packaging APK... >> build_log.txt
cargo apk build --release >> build_log.txt 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed. Check build_log.txt >> build_log.txt
    type build_log.txt
    exit /b %ERRORLEVEL%
)

echo 2. Installing APK... >> build_log.txt
:: Путь изменился после переименования либы в liquify
adb install -r target/release/apk/liquify.apk >> build_log.txt 2>&1

echo 3. Starting application... >> build_log.txt
:: Сначала принудительно остановим
adb shell am force-stop com.test.liquify >> build_log.txt 2>&1
adb shell am start -n com.test.liquify/android.app.NativeActivity >> build_log.txt 2>&1

echo [DONE] Application started. Full log in build_log.txt
type build_log.txt
