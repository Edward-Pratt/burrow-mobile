//! Shells: the device unlock they wait for, and a kept terminal's output
//! streamed to the page.
//!
//! The unlock is asked for here and held by the bridge, never by the page:
//! the page only asks for it, and can't say it passed.

use burrow_api::SessionId;
use burrow_link::bridge::Bridge;
use data_encoding::BASE64;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::{AppHandle, State};

/// What a terminal's channel carries, its bytes in base64.
#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Output {
    /// What the terminal kept while nobody read it. The page writes it
    /// before anything typed is sent, since replies to old queries in it
    /// would otherwise reach the shell.
    Replay {
        data: String,
    },
    Data {
        data: String,
    },
    /// No more output on this channel: the terminal ended, or another
    /// reader took over, or this one fell behind. The page lists sessions
    /// to tell which, and attaches again if it still runs.
    Ended,
}

/// Whether shells are unlocked now, so the page knows whether to ask.
#[tauri::command]
pub fn shells_unlocked(bridge: State<'_, Bridge>) -> bool {
    bridge.is_unlocked()
}

/// Asks for the fingerprint, face or device PIN, and unlocks shells until
/// the app next goes to the background.
#[tauri::command]
pub async fn unlock_shells(app: AppHandle, bridge: State<'_, Bridge>) -> Result<(), String> {
    ask(app).await?;
    bridge.unlock();
    Ok(())
}

#[cfg(mobile)]
async fn ask(app: AppHandle) -> Result<(), String> {
    use tauri_plugin_biometric::{AuthOptions, BiometricExt};
    let options = AuthOptions {
        allow_device_credential: true,
        title: Some("Unlock shells".into()),
        subtitle: Some("Burrow opens shells on your servers".into()),
        ..AuthOptions::default()
    };
    // It waits for the user, so off the async threads.
    let asked = tauri::async_runtime::spawn_blocking(move || {
        app.biometric()
            .authenticate("Unlock shells".into(), options)
    });
    match asked.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e.to_string()),
        Err(e) => Err(e.to_string()),
    }
}

/// A desktop build is only for working on the page: it has no device
/// unlock, so it never unlocks shells.
#[cfg(not(mobile))]
async fn ask(_app: AppHandle) -> Result<(), String> {
    Err("shells need the phone's unlock; this build has none".into())
}

/// Attaches to a kept terminal and streams it to `on_output`, until the
/// stream ends or the page stops listening.
#[tauri::command]
pub async fn attach_terminal(
    bridge: State<'_, Bridge>,
    session: SessionId,
    on_output: Channel<Output>,
) -> Result<(), String> {
    let (replay, mut stream) = bridge.attach(session).await.map_err(|e| e.to_string())?;
    let replay = Output::Replay {
        data: BASE64.encode(&replay),
    };
    if on_output.send(replay).is_err() {
        return Ok(());
    }
    tauri::async_runtime::spawn(async move {
        while let Some(chunk) = stream.recv().await {
            let data = BASE64.encode(&chunk);
            // The page went: dropping the stream detaches, and the
            // terminal runs on.
            if on_output.send(Output::Data { data }).is_err() {
                return;
            }
        }
        let _ = on_output.send(Output::Ended);
    });
    Ok(())
}
