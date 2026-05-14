fn main() {
    // ── Компилируем YCbCr шейдеры ────────────────────────────────────────
    let shaders = [
        ("shaders/ycbcr_blit.vert", "shaders/ycbcr_blit.vert.spv"),
        ("shaders/ycbcr_blit.frag", "shaders/ycbcr_blit.frag.spv"),
    ];

    for (src, out) in &shaders {
        // Пропускаем компиляцию если .spv уже существует (для CI/CD)
        if !std::path::Path::new(src).exists() {
            continue; // Шейдер исходный не найден
        }

        if std::path::Path::new(out).exists() {
            // .spv уже есть, только отметим зависимость
            println!("cargo:rerun-if-changed={}", src);
            continue;
        }

        let status = std::process::Command::new("glslc")
            .args([src, "-o", out])
            .status()
            .unwrap_or_else(|e| {
                eprintln!("⚠️  glslc не найден: {}", e);
                eprintln!("   Установи Android NDK с vulkan-tools");
                eprintln!("   Или скомпилируй шейдеры вручную:");
                eprintln!("   glslc {} -o {}", src, out);
                std::process::exit(1);
            });

        if !status.success() {
            panic!("❌ glslc failed для {}", src);
        }

        println!("✓ Скомпилирован {}", out);
        println!("cargo:rerun-if-changed={}", src);
    }

    // ── Android NDK libs ─────────────────────────────────────────────────
    if std::env::var("TARGET").map(|t| t.contains("android")).unwrap_or(false) {
        let ndk_path = std::env::var("USERPROFILE").unwrap() + r"\AppData\Local\Android\Sdk\ndk\26.1.10909125\toolchains\llvm\prebuilt\windows-x86_64\sysroot\usr\lib\aarch64-linux-android\26";
        println!("cargo:rustc-link-search=native={}", ndk_path);
        println!("cargo:rustc-link-lib=nativewindow");
        println!("cargo:rustc-link-lib=mediandk");
        println!("cargo:rustc-link-lib=android");
    }
}
