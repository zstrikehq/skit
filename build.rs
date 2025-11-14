use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=VERSION");

    let version = match fs::read_to_string("VERSION") {
        Ok(content) => content.trim().to_string(),
        Err(_) => {
            println!("cargo:warning=VERSION file not found, using default version");
            "0.1.0".to_string()
        }
    };

    println!("cargo:rustc-env=SKIT_VERSION={}", version);

    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();

        // Only set icon if it exists
        if std::path::Path::new("icon.ico").exists() {
            res.set_icon("icon.ico");
        }

        // Only set manifest if it exists
        if std::path::Path::new("manifest.xml").exists() {
            res.set_manifest_file("manifest.xml");
        }

        res
            // Version information
            .set("FileVersion", &version)
            .set("ProductVersion", &version)
            .set("FileDescription", env!("CARGO_PKG_DESCRIPTION"))
            .set("ProductName", "SKIT (Security Kit)")
            .set("CompanyName", "SKIT Project")
            .set("LegalCopyright", "© 2025 SKIT Contributors")
            .set("OriginalFilename", "skit.exe")
            .set("InternalName", "skit")
            // Add trustworthy metadata
            .set("Comments", "Open source secrets management tool")
            .set("LegalTrademarks", "")
            .compile()
            .expect("Failed to compile Windows resources");
    }

    #[cfg(not(windows))]
    {
        // No-op for non-Windows builds
    }
}
