//! The app's Rust side: wiring only. Everything that isn't Tauri lives in
//! `burrow-link`, so it builds and tests like any crate.

mod commands;
mod local_network;
mod network;
mod shells;

use std::sync::{Arc, Mutex};

use burrow_link::bridge::{Bridge, LinkState, Outward};
use tauri::{Emitter, Manager};

/// The link's latest state, kept for a page that asks after it was said.
pub(crate) struct LastState(pub Mutex<LinkState>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(mobile)]
    let builder = builder.plugin(tauri_plugin_biometric::init());
    builder
        .plugin(local_network::plugin())
        .plugin(network::plugin())
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
            network::watch(app.handle(), bridge.clone());
            app.manage(bridge);
            Ok(())
        })
        .on_window_event(|_window, _event| {
            // Coming back to the front, a connection may have died while
            // the phone slept; the bridge checks it now rather than later.
            #[cfg(mobile)]
            match _event {
                tauri::WindowEvent::Resumed => _window.state::<Bridge>().reconnect(),
                // Shells ask for the unlock again after the app was away.
                tauri::WindowEvent::Suspended => _window.state::<Bridge>().lock(),
                _ => {}
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
            local_network::allow_local_network,
            shells::shells_unlocked,
            shells::unlock_shells,
            shells::attach_terminal,
            shells::detach_terminal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Burrow");
}
