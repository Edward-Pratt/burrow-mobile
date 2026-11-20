//! Shells from the phone: the device unlock they wait for, and attaching
//! to a kept terminal.
//!
//! The unlock is held here, in Rust, rather than in the page, so a script
//! in the page can't skip it. The app sets it once the system's
//! fingerprint or PIN prompt succeeds, and clears it whenever the app goes
//! to the background, so a phone left unlocked on a table still asks again.

use std::sync::atomic::Ordering;

use burrow_api::SessionId;
use tokio::sync::mpsc;

use crate::bridge::Bridge;
use crate::{Error, Result};

impl Bridge {
    /// The user passed the device unlock.
    pub fn unlock(&self) {
        self.inner.unlocked.store(true, Ordering::SeqCst);
    }

    /// The app went to the background: shells wait for the unlock again.
    pub fn lock(&self) {
        self.inner.unlocked.store(false, Ordering::SeqCst);
    }

    pub fn is_unlocked(&self) -> bool {
        self.inner.unlocked.load(Ordering::SeqCst)
    }

    pub(crate) fn check_unlocked(&self) -> Result<()> {
        if !self.is_unlocked() {
            return Err(Error::Locked);
        }
        Ok(())
    }

    /// Attaches to a kept terminal the phone opened or listed: returns what
    /// it kept, then a stream of its output. The stream ends when the
    /// terminal does, or when another reader takes over or this one falls
    /// behind, in which case the page attaches again.
    pub async fn attach(&self, session: SessionId) -> Result<(Vec<u8>, mpsc::Receiver<Vec<u8>>)> {
        self.check_unlocked()?;
        self.conn()?.attach(session).await
    }
}
