  fn main() {
    println!("cargo:rerun-if-changed=resources/fridge.ico");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let icon = std::path::Path::new("resources/fridge.ico");
    if target_os == "windows" && icon.exists() {
        if let Err(e) = winres::WindowsResource::new()
            .set_icon(icon.to_str().expect("icon path must be valid UTF-8"))
            .compile()
        {
            println!("cargo:warning=Icon embedding skipped: {e}");
        }
    } else if target_os == "windows" {
        println!("cargo:warning=resources/fridge.ico not found — exe will use the default icon");
    }
}
