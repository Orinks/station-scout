use std::env;
use std::path::Path;

fn main() {
    let manifest = Path::new("rust/windows/station-scout.exe.manifest");
    println!("cargo:rerun-if-changed={}", manifest.display());

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest_path = manifest
            .canonicalize()
            .expect("Station Scout Windows manifest should exist");
        println!("cargo:rustc-link-arg-bin=station-scout=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=station-scout=/MANIFESTINPUT:{}",
            manifest_path.display()
        );
    }
}
