fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winresource::WindowsResource::new();
        if std::path::Path::new("../../assets/icons/snipe.ico").exists() {
            res.set_icon("../../assets/icons/snipe.ico");
        }
        // Enable Per-Monitor DPI Awareness V2 before any window is created
        // (repair-plan P0-1 / PLAN.md 6.2). Monitor enumeration then reports
        // physical pixel coordinates without DPI virtualization.
        res.set_manifest(include_str!("../snipe.manifest"));
        res.set("ProductName", "Snipe");
        res.set("FileDescription", "Snipe High Performance Screenshot Tool");
        res.set("LegalCopyright", "Copyright (C) 2026 Snipe Contributors");
        res.compile()
            .expect("failed to compile Windows application resources");
    }
}
