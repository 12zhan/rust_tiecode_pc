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
    let dst = cmake::Config::new("native/sweetline")
        .define("STATIC_LIB", "ON")
        .define("CMAKE_BUILD_TYPE", "Release")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());

    // Windows 下库名为 sweetline_static，Linux/macOS 下通常为 libsweetline_static.a
    println!("cargo:rustc-link-lib=static=sweetline_static");

    // 3. 链接 C++ 标准库
    #[cfg(target_os = "linux")]
    println!("cargo:rustc-link-lib=dylib=stdc++");
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=dylib=c++");

    // 4. 生成 Rust 绑定
    let bindings = bindgen::Builder::default()
        .header("native/sweetline/src/include/c_sweetline.h")
        .clang_arg("-Inative/sweetline/src/include")
        .clang_arg("-xc++")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
