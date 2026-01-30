use std::env;
use std::path::PathBuf;

fn main() {
    // 1. 处理 Windows 图标 (保留原有逻辑)
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.compile().unwrap();

        println!("cargo:rerun-if-changed=src/ffi_guard.c");
        cc::Build::new()
            .file("src/ffi_guard.c")
            .compile("ffi_guard");
    }

    // 2. 编译 SweetLine C++ 静态库
    println!("cargo:rerun-if-changed=native/sweetline");
    let mut config = cmake::Config::new("native/sweetline");
    config.define("STATIC_LIB", "ON").profile("Release");
    
    #[cfg(target_os = "windows")]
    config.cxxflag("/EHsc");
    
    let dst = config.build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());

    // Windows 下库名为 sweetline_static，Linux/macOS 下通常为 libsweetline_static.a
    println!("cargo:rustc-link-lib=static=sweetline_static");

    // 3. 链接 C++ 标准库
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=onig");
    }
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=dylib=c++");
        println!("cargo:rustc-link-lib=dylib=iconv");
        println!("cargo:rustc-link-lib=dylib=onig");
        
        // 尝试通过 pkg-config 查找 oniguruma 路径 (解决 macOS 上 brew 安装路径问题)
        use std::process::Command;
        if let Ok(output) = Command::new("pkg-config").args(&["--libs-only-L", "oniguruma"]).output() {
            let s = String::from_utf8_lossy(&output.stdout);
            for part in s.split_whitespace() {
                if let Some(path) = part.strip_prefix("-L") {
                    println!("cargo:rustc-link-search=native={}", path);
                }
            }
        }
    }

    // 4. 生成 Rust 绑定
    let bindings = bindgen::Builder::default()
        .header("native/sweetline/src/include/c_sweetline.h")
        .clang_arg("-Inative/sweetline/src/include")
        .clang_arg("-xc++")
        // 只生成 sl_ 开头的类型和函数，避免引入 std::* 导致的 _Tp 错误
        .allowlist_function("sl_.*")
        .allowlist_type("sl_.*")
        .allowlist_var("sl_.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
