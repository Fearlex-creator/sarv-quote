use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    if let Ok(head) = std::fs::read_to_string("../../.git/HEAD")
        && let Some(ref_path) = head.trim().strip_prefix("ref: ")
    {
        println!("cargo:rerun-if-changed=../../.git/{ref_path}");
    }
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=SARV_GIT_SHA={sha}");

    #[cfg(windows)]
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=../../assets/brand/sarv.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/brand/sarv.ico");
        if let Err(err) = res.compile() {
            panic!("failed to compile Windows icon resource: {err}");
        }
    }
}
