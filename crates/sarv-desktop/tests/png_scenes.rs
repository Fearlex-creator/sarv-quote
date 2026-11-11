//! Headless PNG snapshots for key Rune scenes (`--snap`).

use std::process::Command;

#[test]
fn png_snapshots_for_main_scenes() {
    let dir = tempfile::tempdir().expect("tmpdir");
    let bin = env!("CARGO_BIN_EXE_sarv-desktop");
    for scene in ["overview", "sales", "inventory", "accounting", "settings", "light", "collapse"] {
        let out = dir.path().join(format!("{scene}.png"));
        let status = Command::new(bin)
            .env("SARV_DEMO", "1")
            .args([
                "--scene",
                scene,
                "--snap",
                out.to_str().unwrap(),
                "--scale",
                "1",
                "--size",
                "1180x780",
            ])
            .status()
            .unwrap_or_else(|e| panic!("spawn sarv-desktop: {e}"));
        assert!(status.success(), "snap {scene} failed: {status}");
        let meta = std::fs::metadata(&out).unwrap_or_else(|e| panic!("{scene}.png missing: {e}"));
        assert!(meta.len() > 1_000, "{scene}.png too small ({})", meta.len());
    }
}
