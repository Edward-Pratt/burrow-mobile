//! The link against a throwaway `sshd` (see `common`). Skips when sshd
//! isn't installed.

mod common;

use std::fs;
use std::process::Command as Process;

use burrow_api::{Command, Response};
use burrow_link::Error;
use burrow_link::link::{Link, PROXY};

use common::{Server, phone_key};

#[tokio::test(flavor = "multi_thread")]
async fn the_phone_reaches_the_proxy_as_mobile() {
    let (_keys, key) = phone_key();
    let Some(server) = Server::start(&key) else {
        return eprintln!("skipping: sshd is not installed");
    };
    let pairing = server.pairing(Some(server.host_key()));
    let link = Link::connect(&pairing, &key).await.unwrap();
    assert_eq!(link.client().info().build, "fake");
    let nodes = link.client().execute(Command::ListNodes).await.unwrap();
    assert!(matches!(nodes, Response::Nodes { nodes } if nodes.is_empty()));

    let command = fs::read_to_string(server.path("command")).unwrap();
    assert_eq!(command.trim(), PROXY);
    let hello = fs::read_to_string(server.path("initialize")).unwrap();
    assert!(hello.contains(r#""method":"initialize""#), "{hello}");
    assert!(hello.contains(r#""role":"mobile""#), "{hello}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_new_computer_asks_for_its_fingerprint_first() {
    let (_keys, key) = phone_key();
    let Some(server) = Server::start(&key) else {
        return eprintln!("skipping: sshd is not installed");
    };
    let Err(Error::NewHostKey {
        fingerprint,
        key: offered,
    }) = Link::connect(&server.pairing(None), &key).await
    else {
        panic!("an unpinned computer connected");
    };
    let listed = Process::new("ssh-keygen")
        .arg("-lf")
        .arg(server.path("host_key.pub"))
        .output()
        .unwrap();
    let listed = String::from_utf8(listed.stdout).unwrap();
    assert!(listed.contains(&fingerprint), "{listed} vs {fingerprint}");
    // The proxy never ran: nothing was sent before the key was confirmed.
    assert!(!server.path("command").exists());

    Link::connect(&server.pairing(Some(offered)), &key)
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_host_key_is_refused() {
    let (_keys, key) = phone_key();
    let Some(server) = Server::start(&key) else {
        return eprintln!("skipping: sshd is not installed");
    };
    let (_other_dir, other) = phone_key();
    let wrong = other.public_line();
    let result = Link::connect(&server.pairing(Some(wrong)), &key).await;
    assert!(matches!(result, Err(Error::HostKeyChanged { .. })));
    assert!(!server.path("command").exists());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_key_the_computer_doesnt_know_is_refused() {
    let (_keys, key) = phone_key();
    let Some(server) = Server::start(&key) else {
        return eprintln!("skipping: sshd is not installed");
    };
    let (_other_dir, stranger) = phone_key();
    let result = Link::connect(&server.pairing(Some(server.host_key())), &stranger).await;
    assert!(
        matches!(result, Err(Error::KeyRefused)),
        "{:?}",
        result.err()
    );
}
