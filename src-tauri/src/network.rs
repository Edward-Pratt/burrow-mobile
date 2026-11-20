//! Android saying a network came or went. The bridge then checks its
//! connection at once instead of at the next backoff or keepalive, so the
//! app is back as soon as the phone is. Elsewhere there is nothing to hear.

use burrow_link::bridge::Bridge;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Runtime};

#[cfg(target_os = "android")]
struct Network<R: Runtime>(tauri::plugin::PluginHandle<R>);

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("network")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                let handle = _api.register_android_plugin("dev.burrow.mobile", "NetworkPlugin")?;
                _app.manage(Network(handle));
            }
            Ok(())
        })
        .build()
}

/// Has the bridge check its connection whenever a network comes or goes.
/// A connection that still answers is kept.
pub fn watch<R: Runtime>(app: &AppHandle<R>, bridge: Bridge) {
    #[cfg(target_os = "android")]
    {
        use tauri::Manager;
        let on_change = tauri::ipc::Channel::<()>::new(move |_| {
            bridge.reconnect();
            Ok(())
        });
        let args = serde_json::json!({ "onChange": on_change });
        // Without it the backoff and keepalive still reconnect, only later.
        let _ = app
            .state::<Network<R>>()
            .0
            .run_mobile_plugin::<serde_json::Value>("watch", args);
    }
    #[cfg(not(target_os = "android"))]
    let _ = (app, bridge);
}
