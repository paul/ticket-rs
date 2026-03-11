use std::process::Command;

fn main() {
    // Re-run this build script when commits change.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");

    let date = Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=format:%Y%m%d"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=TICKET_VERSION_DATE=v{date}");

    let compiled_at = Command::new("git")
        .args(["log", "-1", "--format=%cd", "--date=iso-strict"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=TICKET_COMPILED_AT={compiled_at}");
}
