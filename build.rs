fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=ui/cockpit.slint");
    println!("cargo:rerun-if-changed=build.rs");

    slint_build::compile("ui/cockpit.slint").expect("Failed to compile Slint UI specification");

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            eprintln!("Failed to compile windows resource: {}", e);
        }
    }
}
