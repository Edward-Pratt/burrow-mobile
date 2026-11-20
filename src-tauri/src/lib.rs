//! The app's Rust side: wiring only. Everything that isn't Tauri lives in
//! `burrow-link`, so it builds and tests like any crate.

mod commands;

use std::sync::{Arc, Mutex};

use burrow_link::bridge::{Bridge, LinkState, Outward};
use tauri::{Emitter, Manager};

/// The link's latest state, kept for a page that asks after it was said.
pub(crate) struct LastState(pub Mutex<LinkState>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            app.manage(LastState(Mutex::new(LinkState::Connecting)));
            let handle = app.handle().clone();
            let sink = Arc::new(move |outward: Outward| {
                // A send fails only once the window has gone.
                let _ = match outward {
                    Outward::Link(state) => {
                        *handle.state::<LastState>().0.lock().unwrap() = state.clone();
                        handle.emit("burrow://link", state)
                    }
                    Outward::Event(event) => handle.emit("burrow://event", event),
                    Outward::Relist => handle.emit("burrow://relist", ()),
                };
            });
            // The bridge spawns onto the runtime it starts in.
            let bridge = tauri::async_runtime::block_on(async { Bridge::start(dir, sink) });
            app.manage(bridge);
            Ok(())
        })
        .on_window_event(|_window, _event| {
            // Coming back to the front, a connection may have died while
            // the phone slept; the bridge checks it now rather than later.
            #[cfg(mobile)]
            if let tauri::WindowEvent::Resumed = _event {
                _window.state::<Bridge>().reconnect();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::link_state,
            commands::execute,
            commands::paired,
            commands::key_line,
            commands::pair,
            commands::pin,
            commands::forget,
            commands::reconnect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Burrow");
}
