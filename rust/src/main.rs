#[cfg(feature = "gui")]
fn main() {
    station_scout::ui::wx::run();
}

#[cfg(not(feature = "gui"))]
fn main() {
    eprintln!("Station Scout Rust GUI requires: cargo run --features gui");
}
