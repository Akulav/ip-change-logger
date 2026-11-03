fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    // Embed the icon and version info into the Windows executable.
    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico")
            .set("ProductName", "IPTrail")
            .set("FileDescription", "IPTrail — public IP monitor")
            .set("CompanyName", "Akulav")
            .set("LegalCopyright", "Copyright (c) 2026 Akulav");
        if let Err(e) = res.compile() {
            println!("cargo:warning=could not embed Windows resources: {e}");
        }
    }
}
