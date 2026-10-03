fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=ui/cockpit.slint");
    println!("cargo:rerun-if-changed=build.rs");

    slint_build::compile("ui/cockpit.slint").expect("Failed to compile Slint UI specification");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "Antigravity 多实例管理器");
        res.set("ProductName", "Antigravity Multi-Instance Manager");
        let file_version = format!("{}.0", env!("CARGO_PKG_VERSION"));
        res.set("FileVersion", &file_version);

        if let Err(e) = res.compile() {
            eprintln!("Failed to compile windows resource: {}", e);
        }
    }
}
