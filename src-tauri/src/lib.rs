//! The app's Rust side: wiring only. Everything that isn't Tauri lives in
//! `burrow-link`, so it builds and tests like any crate.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Burrow");
}
