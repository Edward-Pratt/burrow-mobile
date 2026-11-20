//! The app's one connection to `burrowd`, kept up for as long as the app
//! runs. It reconnects after a drop with a backoff, tells the UI where it
//! stands, forwards the daemon's events, and asks the UI to list
//! everything again whenever it may have missed some. Only the commands in
//! [`allow`](crate::allow) get through to the daemon.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use burrow_api::{Command, Event, Response};
use serde::Serialize;
use tokio::sync::Notify;
use tokio::sync::broadcast::error::RecvError;

use crate::allow::allowed;
use crate::backoff::Backoff;
use crate::conn::Conn;
use crate::keys::PhoneKey;
use crate::link::Link;
use crate::pairing::{Pairing, User};
use crate::{Error, Result};

/// How long a connection may take to answer after the app comes back,
/// before it counts as dead. A phone that slept may hold a connection the
/// network dropped long ago.
const PROBE: Duration = Duration::from_secs(5);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum LinkState {
    Unpaired,
    Connecting,
    Connected { build: String },
    NewHostKey { fingerprint: String, key: String },
    HostKeyChanged { pinned: String, offered: String },
    KeyRefused,
    Failed { error: String, retry_in: u64 },
}

/// What the bridge tells the UI.
#[derive(Clone, Debug)]
pub enum Outward {
    Link(LinkState),
    Event(Event),
    /// List nodes, alerts, monitoring and sessions again.
    Relist,
}

pub type Sink = Arc<dyn Fn(Outward) + Send + Sync>;

#[derive(Clone)]
pub struct Bridge {
    inner: Arc<Inner>,
}

struct Inner {
    dir: PathBuf,
    sink: Sink,
    conn: Mutex<Option<Conn>>,
    wake: Notify,
    /// The pairing changed, so a live connection is to the wrong computer.
    restart: AtomicBool,
}

impl Bridge {
    /// Starts keeping the connection up, with the key and pairing in
    /// `dir`. Must be called within a tokio runtime.
    pub fn start(dir: PathBuf, sink: Sink) -> Bridge {
        let inner = Arc::new(Inner {
            dir,
            sink,
            conn: Mutex::new(None),
            wake: Notify::new(),
            restart: AtomicBool::new(false),
        });
        tokio::spawn(run(inner.clone()));
        Bridge { inner }
    }

    pub async fn execute(&self, command: Command) -> Result<Response> {
        if !allowed(&command) {
            return Err(Error::invalid("Command", "isn't one the phone sends"));
        }
        let conn = self.inner.conn.lock().unwrap().clone();
        let conn = conn.ok_or_else(|| Error::Daemon("not connected".into()))?;
        conn.execute(command).await
    }

    /// Tries again now: the app came back to the front, a network came or
    /// went, or the user asked. A connection that still answers is kept.
    pub fn reconnect(&self) {
        self.inner.wake.notify_one();
    }

    /// The computer the phone is paired with, if any.
    pub fn paired(&self) -> Result<Option<Pairing>> {
        Pairing::load(&self.inner.dir)
    }

    /// The line for `user`'s `authorized_keys` on the computer.
    pub fn key_line(&self, user: &str) -> Result<String> {
        let key = PhoneKey::load_or_create(&self.inner.dir)?;
        Ok(key.authorized_keys_line(&User::parse(user)?))
    }

    /// Pairs with a computer, its host key still to be confirmed.
    pub fn pair(&self, address: &str, port: u16, user: &str) -> Result<()> {
        Pairing::new(address, port, user)?.save(&self.inner.dir)?;
        self.restart()
    }

    /// Pins the host key the user confirmed, as [`LinkState::NewHostKey`]
    /// gave it.
    pub fn pin(&self, key: &str) -> Result<()> {
        crate::hostkey::parse(key)?;
        let mut pairing = Pairing::load(&self.inner.dir)?
            .ok_or_else(|| Error::invalid("Pairing", "is missing"))?;
        pairing.host_key = Some(key.trim().to_owned());
        pairing.save(&self.inner.dir)?;
        self.restart()
    }

    pub fn forget(&self) -> Result<()> {
        Pairing::forget(&self.inner.dir)?;
        self.restart()
    }

    fn restart(&self) -> Result<()> {
        self.inner.restart.store(true, Ordering::SeqCst);
        self.inner.wake.notify_one();
        Ok(())
    }
}

impl Inner {
    fn emit(&self, outward: Outward) {
        (self.sink)(outward);
    }

    fn state(&self, state: LinkState) {
        self.emit(Outward::Link(state));
    }
}

async fn run(inner: Arc<Inner>) {
    let mut backoff = Backoff::default();
    loop {
        inner.restart.store(false, Ordering::SeqCst);
        match attempt(&inner, &mut backoff).await {
            Some(wait) => tokio::select! {
                () = tokio::time::sleep(wait) => {}
                () = inner.wake.notified() => backoff.reset(),
            },
            // No timer: only the user can move it on, by pairing, pinning,
            // or adding the key's line on the computer. A wake still tries
            // again, in case they did that last one.
            None => {
                inner.wake.notified().await;
                backoff.reset();
            }
        }
    }
}

/// One go at connecting, and following the daemon while it lasts. Returns
/// how long to wait before the next, or `None` to wait for the user.
async fn attempt(inner: &Inner, backoff: &mut Backoff) -> Option<Duration> {
    let failed = |inner: &Inner, error: String, wait: Duration| {
        inner.state(LinkState::Failed {
            error,
            retry_in: wait.as_secs(),
        });
        Some(wait)
    };
    let pairing = match Pairing::load(&inner.dir) {
        Ok(Some(pairing)) => pairing,
        Ok(None) => {
            inner.state(LinkState::Unpaired);
            return None;
        }
        Err(e) => return failed(inner, e.to_string(), backoff.next()),
    };
    let key = match PhoneKey::load_or_create(&inner.dir) {
        Ok(key) => key,
        Err(e) => return failed(inner, e.to_string(), backoff.next()),
    };
    inner.state(LinkState::Connecting);
    match Link::connect(&pairing, &key).await {
        Ok(link) => {
            backoff.reset();
            let conn = Conn::new(link.client().clone());
            *inner.conn.lock().unwrap() = Some(conn.clone());
            inner.state(LinkState::Connected {
                build: link.client().info().build.clone(),
            });
            inner.emit(Outward::Relist);
            follow(inner, &link, &conn).await;
            *inner.conn.lock().unwrap() = None;
            if inner.restart.load(Ordering::SeqCst) {
                return Some(Duration::ZERO);
            }
            failed(inner, "the connection closed".into(), backoff.next())
        }
        Err(Error::NewHostKey { fingerprint, key }) => {
            inner.state(LinkState::NewHostKey { fingerprint, key });
            None
        }
        Err(Error::HostKeyChanged { pinned, offered }) => {
            inner.state(LinkState::HostKeyChanged { pinned, offered });
            None
        }
        Err(Error::KeyRefused) => {
            inner.state(LinkState::KeyRefused);
            None
        }
        Err(e) => failed(inner, e.to_string(), backoff.next()),
    }
}

/// Forwards events until the connection ends, the pairing changes, or a
/// wake finds it no longer answers.
async fn follow(inner: &Inner, link: &Link, conn: &Conn) {
    let mut events = link.client().subscribe();
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    conn.saw(&event);
                    inner.emit(Outward::Event(event));
                }
                Err(RecvError::Lagged(_)) => inner.emit(Outward::Relist),
                Err(RecvError::Closed) => return,
            },
            () = inner.wake.notified() => {
                if inner.restart.load(Ordering::SeqCst) {
                    return;
                }
                let probe = link.client().execute(Command::ListSessions);
                if !matches!(tokio::time::timeout(PROBE, probe).await, Ok(Ok(_))) {
                    return;
                }
            }
        }
    }
}
