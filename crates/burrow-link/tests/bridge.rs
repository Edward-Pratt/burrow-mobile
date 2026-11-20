//! The bridge against a throwaway `sshd` (see `common`): pairing, pinning
//! the host key the user confirmed, connecting, and the allowlist. Skips
//! when sshd isn't installed.

mod common;

use std::sync::Arc;
use std::time::Duration;

use burrow_api::{Command, Response};
use burrow_link::bridge::{Bridge, LinkState, Outward};
use tempfile::TempDir;
use tokio::sync::mpsc;

use common::{Server, whoami};

async fn next_state(rx: &mut mpsc::UnboundedReceiver<Outward>) -> LinkState {
    loop {
        let outward = tokio::time::timeout(Duration::from_secs(15), rx.recv())
            .await
            .expect("the bridge said nothing")
            .unwrap();
        if let Outward::Link(state) = outward
            && state != LinkState::Connecting
        {
            return state;
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_bridge_pairs_pins_connects_and_filters() {
    let app = TempDir::new().unwrap();
    let (tx, mut rx) = mpsc::unbounded_channel();
    let bridge = Bridge::start(app.path().to_path_buf(), Arc::new(move |o| _ = tx.send(o)));
    assert_eq!(next_state(&mut rx).await, LinkState::Unpaired);

    // The key exists once its line has been shown, as on the phone.
    let line = bridge.key_line(&whoami()).unwrap();
    assert!(line.contains("burrow proxy --role mobile"));
    let key = burrow_link::keys::PhoneKey::load_or_create(app.path()).unwrap();
    let Some(server) = Server::start(&key) else {
        return eprintln!("skipping: sshd is not installed");
    };

    bridge.pair("127.0.0.1", server.port, &whoami()).unwrap();
    let LinkState::NewHostKey { key, .. } = next_state(&mut rx).await else {
        panic!("connected without a pinned host key");
    };
    bridge.pin(&key).unwrap();
    assert!(matches!(
        next_state(&mut rx).await,
        LinkState::Connected { build } if build == "fake"
    ));
    assert!(matches!(rx.recv().await, Some(Outward::Relist)));

    let nodes = bridge.execute(Command::ListNodes).await.unwrap();
    assert!(matches!(nodes, Response::Nodes { nodes } if nodes.is_empty()));
    let refused = bridge
        .execute(serde_json::from_str(r#"{"type":"list_addons"}"#).unwrap())
        .await;
    assert!(refused.is_err());
    // A session the phone didn't open, such as a desktop terminal, is
    // refused before it reaches the daemon, which the stand-in proxy
    // would never answer.
    let close = Command::CloseSession {
        session: burrow_api::SessionId(7),
    };
    let refused = tokio::time::timeout(Duration::from_secs(2), bridge.execute(close)).await;
    assert!(matches!(refused, Ok(Err(_))));
}
