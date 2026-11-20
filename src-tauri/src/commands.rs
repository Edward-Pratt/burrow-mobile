//! The commands the page may call. Each is a thin call into the bridge;
//! errors come back as the text to show.

use burrow_api::{Command, Response};
use burrow_link::bridge::{Bridge, LinkState};
use tauri::State;

use crate::LastState;

type Answer<T> = Result<T, String>;

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Where the link stands now, for a page that loaded after it last said.
#[tauri::command]
pub fn link_state(last: State<'_, LastState>) -> LinkState {
    last.0.lock().unwrap().clone()
}

/// A daemon command, as the API's JSON. The bridge refuses any the
/// screens don't use.
#[tauri::command]
pub async fn execute(bridge: State<'_, Bridge>, command: serde_json::Value) -> Answer<Response> {
    let command: Command = serde_json::from_value(command).map_err(text)?;
    bridge.execute(command).await.map_err(text)
}

/// The computer it is paired with: where, and as whom.
#[tauri::command]
pub fn paired(bridge: State<'_, Bridge>) -> Answer<Option<serde_json::Value>> {
    let pairing = bridge.paired().map_err(text)?;
    Ok(
        pairing
            .map(|p| serde_json::json!({ "address": p.address, "port": p.port, "user": p.user })),
    )
}

#[tauri::command]
pub fn key_line(bridge: State<'_, Bridge>, user: String) -> Answer<String> {
    bridge.key_line(&user).map_err(text)
}

#[tauri::command]
pub fn pair(bridge: State<'_, Bridge>, address: String, port: u16, user: String) -> Answer<()> {
    bridge.pair(&address, port, &user).map_err(text)
}

#[tauri::command]
pub fn pin(bridge: State<'_, Bridge>, key: String) -> Answer<()> {
    bridge.pin(&key).map_err(text)
}

#[tauri::command]
pub fn forget(bridge: State<'_, Bridge>) -> Answer<()> {
    bridge.forget().map_err(text)
}

#[tauri::command]
pub fn reconnect(bridge: State<'_, Bridge>) {
    bridge.reconnect();
}
