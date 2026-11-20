//! Android 17's permission to reach the local network, which a computer
//! paired by its LAN address needs. The page asks for it before pairing
//! with such an address; elsewhere it is always there.

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{AppHandle, Runtime};

#[cfg(target_os = "android")]
struct LocalNetwork<R: Runtime>(tauri::plugin::PluginHandle<R>);

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("local-network")
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                let handle =
                    _api.register_android_plugin("dev.burrow.mobile", "LocalNetworkPlugin")?;
                _app.manage(LocalNetwork(handle));
            }
            Ok(())
        })
        .build()
}

/// Asks Android for the permission if it isn't granted yet. Whether it is
/// granted afterwards.
#[tauri::command]
pub async fn allow_local_network<R: Runtime>(app: AppHandle<R>) -> Result<bool, String> {
    #[cfg(target_os = "android")]
    {
        use tauri::Manager;
        let plugin = app.state::<LocalNetwork<R>>();
        let states: serde_json::Value = plugin
            .0
            .run_mobile_plugin_async(
                "requestPermissions",
                serde_json::json!({ "permissions": ["localNetwork"] }),
            )
            .await
            .map_err(|e| e.to_string())?;
        Ok(states["localNetwork"] == "granted")
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(true)
    }
}
