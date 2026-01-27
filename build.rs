fn main() {
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
}
