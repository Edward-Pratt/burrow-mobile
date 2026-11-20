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

    /// The app went to the background: shells wait for the unlock again,
    /// and the streams attached end.
    pub fn lock(&self) {
        self.inner.unlocked.store(false, Ordering::SeqCst);
        self.inner.locks.send_replace(());
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
    /// terminal does, when another reader takes over or this one falls
    /// behind, in which case the page attaches again, and when shells lock.
    pub async fn attach(&self, session: SessionId) -> Result<(Vec<u8>, mpsc::Receiver<Vec<u8>>)> {
        let mut locks = self.inner.locks.subscribe();
        self.check_unlocked()?;
        let (replay, mut stream) = self.conn()?.attach(session).await?;
        // Locked while attaching: nothing reaches the page.
        self.check_unlocked()?;
        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    chunk = stream.recv() => {
                        let Some(chunk) = chunk else { return };
                        if tx.send(chunk).await.is_err() {
                            return;
                        }
                    }
                    // Dropping the stream detaches; the terminal runs on.
                    _ = locks.changed() => return,
                }
            }
        });
        Ok((replay, rx))
    }
}
