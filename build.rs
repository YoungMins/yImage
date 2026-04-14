fn main() {
    #[cfg(windows)]
    {
        // Embed icon, version info, and a Windows 10+ / DPI-aware manifest.
        let mut res = winres::WindowsResource::new();
        // The icon file is optional during early development; ignore errors so
        // `cargo check` still works on a clean checkout.
        let icon_path = "assets/icons/yimage.ico";
        if std::path::Path::new(icon_path).exists() {
            res.set_icon(icon_path);
        }
        res.set("ProductName", "yImage");
        res.set("FileDescription", "Fast Windows image viewer and editor");
        res.set("CompanyName", "Youngmin Kim");
        res.set("LegalCopyright", "Copyright (c) 2026 Youngmin Kim");
        // Best-effort: ignore manifest errors on non-MSVC toolchains.
        let _ = res.compile();
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icons/yimage.ico");
}
