use station_scout::gst_runtime;

fn main() {
    gst_runtime::configure_bundled_gstreamer();

    if std::env::args().any(|arg| arg == "--check-gstreamer") {
        std::process::exit(check_gstreamer());
    }

    run_gui();
}

/// Prints whether GStreamer can play streams. Exit code 0 means yes.
fn check_gstreamer() -> i32 {
    match gst_runtime::check_playback_support() {
        Ok(missing) if missing.is_empty() => {
            println!("{}: playback ready.", gstreamer::version_string());
            0
        }
        Ok(missing) => {
            println!(
                "{}: missing {}.",
                gstreamer::version_string(),
                missing.join(", ")
            );
            1
        }
        Err(error) => {
            println!("{error}");
            1
        }
    }
}

#[cfg(feature = "gui")]
fn run_gui() {
    station_scout::ui::wx::run();
}

#[cfg(not(feature = "gui"))]
fn run_gui() {
    eprintln!("Station Scout Rust GUI requires: cargo run --features gui");
}
