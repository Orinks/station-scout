//! Points GStreamer at the copy bundled next to the Windows executable.
//!
//! Windows release builds ship GStreamer's DLLs beside `StationScout.exe`
//! and its plugins under `lib/gstreamer-1.0`. GStreamer does not look there
//! on its own, so we set its search paths before it initializes. Builds
//! without a bundled copy (Linux, `cargo run`) are left untouched.

use std::path::{Path, PathBuf};

/// Plugin folder inside a bundled install, relative to the executable.
pub const BUNDLED_PLUGIN_DIR: &str = "lib/gstreamer-1.0";
/// GIO modules folder (TLS support for HTTPS streams).
pub const BUNDLED_GIO_MODULE_DIR: &str = "lib/gio/modules";
/// Out-of-process plugin scanner.
pub const BUNDLED_PLUGIN_SCANNER: &str = "libexec/gstreamer-1.0/gst-plugin-scanner.exe";

/// Elements a radio or podcast stream needs to play.
pub const REQUIRED_ELEMENTS: &[&str] = &[
    "playbin",
    "uridecodebin",
    "typefind",
    "souphttpsrc",
    "audioconvert",
    "audioresample",
    "volume",
    "autoaudiosink",
];

/// At least one element from each group must be present.
pub const REQUIRED_ELEMENT_GROUPS: &[(&str, &[&str])] = &[
    ("MP3 decoder", &["mpg123audiodec", "avdec_mp3"]),
    ("AAC decoder", &["fdkaacdec", "avdec_aac"]),
];

/// Environment variables to set for the bundled GStreamer in `app_dir`.
/// Returns an empty list when `app_dir` has no bundled plugin folder.
pub fn bundled_gstreamer_env(app_dir: &Path) -> Vec<(&'static str, PathBuf)> {
    let plugin_dir = app_dir.join(BUNDLED_PLUGIN_DIR);
    if !plugin_dir.is_dir() {
        return Vec::new();
    }
    // Override the system path so a separate GStreamer install on the
    // machine can't mix its plugins with our core libraries.
    let mut vars = vec![
        ("GST_PLUGIN_SYSTEM_PATH_1_0", plugin_dir.clone()),
        ("GST_PLUGIN_SYSTEM_PATH", plugin_dir),
    ];
    let scanner = app_dir.join(BUNDLED_PLUGIN_SCANNER);
    if scanner.is_file() {
        vars.push(("GST_PLUGIN_SCANNER_1_0", scanner.clone()));
        vars.push(("GST_PLUGIN_SCANNER", scanner));
    }
    let gio_modules = app_dir.join(BUNDLED_GIO_MODULE_DIR);
    if gio_modules.is_dir() {
        vars.push(("GIO_MODULE_DIR", gio_modules));
    }
    vars
}

/// Applies [`bundled_gstreamer_env`] for the running executable.
///
/// Call once at the very start of `main`, before any threads start and
/// before GStreamer initializes.
pub fn configure_bundled_gstreamer() {
    let Some(app_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
    else {
        return;
    };
    for (key, value) in bundled_gstreamer_env(&app_dir) {
        // SAFETY: called from main before any other thread exists.
        unsafe { std::env::set_var(key, value) };
    }
}

/// Initializes GStreamer and reports which playback elements are missing.
/// Used by `--check-gstreamer` and the release build's smoke test.
pub fn check_playback_support() -> Result<Vec<String>, String> {
    gstreamer::init().map_err(|error| format!("Failed to initialize GStreamer: {error}"))?;
    // Creating the element, not just finding its factory, catches plugins
    // whose libraries fail to load at runtime (souphttpsrc needs libsoup).
    let has = |name: &str| gstreamer::ElementFactory::make(name).build().is_ok();
    let mut missing: Vec<String> = REQUIRED_ELEMENTS
        .iter()
        .filter(|name| !has(name))
        .map(|name| (*name).to_string())
        .collect();
    for (label, choices) in REQUIRED_ELEMENT_GROUPS {
        if !choices.iter().any(|name| has(name)) {
            missing.push(format!("{label} ({})", choices.join(" or ")));
        }
    }
    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_bundle_sets_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(bundled_gstreamer_env(dir.path()).is_empty());
    }

    #[test]
    fn bundle_points_gstreamer_at_app_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(BUNDLED_PLUGIN_DIR)).unwrap();
        std::fs::create_dir_all(dir.path().join(BUNDLED_GIO_MODULE_DIR)).unwrap();
        let scanner = dir.path().join(BUNDLED_PLUGIN_SCANNER);
        std::fs::create_dir_all(scanner.parent().unwrap()).unwrap();
        std::fs::write(&scanner, b"").unwrap();

        let vars = bundled_gstreamer_env(dir.path());
        let get = |key: &str| vars.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone());
        assert_eq!(
            get("GST_PLUGIN_SYSTEM_PATH_1_0"),
            Some(dir.path().join(BUNDLED_PLUGIN_DIR))
        );
        assert_eq!(get("GST_PLUGIN_SCANNER_1_0"), Some(scanner));
        assert_eq!(
            get("GIO_MODULE_DIR"),
            Some(dir.path().join(BUNDLED_GIO_MODULE_DIR))
        );
    }

    #[test]
    fn scanner_and_gio_are_optional() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(BUNDLED_PLUGIN_DIR)).unwrap();
        let keys: Vec<_> = bundled_gstreamer_env(dir.path())
            .into_iter()
            .map(|(k, _)| k)
            .collect();
        assert_eq!(
            keys,
            ["GST_PLUGIN_SYSTEM_PATH_1_0", "GST_PLUGIN_SYSTEM_PATH"]
        );
    }
}
