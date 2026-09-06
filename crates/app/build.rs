fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        if std::path::Path::new("../../assets/icons/snipe.ico").exists() {
            res.set_icon("../../assets/icons/snipe.ico");
        }
        res.set("ProductName", "Snipe");
        res.set("FileDescription", "Snipe High Performance Screenshot Tool");
        res.set("LegalCopyright", "Copyright (C) 2026 Snipe Contributors");
        let _ = res.compile();
    }
}
