fn main() {
    if std::env::var("TARGET").map(|t| t.contains("android")).unwrap_or(false) {
        let ndk_path = std::env::var("USERPROFILE").unwrap() + r"\AppData\Local\Android\Sdk\ndk\26.1.10909125\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\aarch64-linux-android\26";
        println!("cargo:rustc-link-search=native={}", ndk_path);
        println!("cargo:rustc-link-lib=nativewindow");
        println!("cargo:rustc-link-lib=mediandk");
        println!("cargo:rustc-link-lib=android");
    }
}
