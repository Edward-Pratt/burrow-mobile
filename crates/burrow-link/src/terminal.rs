//! Shells from the phone: the device unlock they wait for, and attaching
//! to a kept terminal.
//!
//! The unlock is held here, in Rust, rather than in the page, so a script
//! in the page can't skip it. The app sets it once the system's
//! fingerprint or PIN prompt succeeds, and clears it whenever the app goes
//! to the background, so a phone left unlocked on a table still asks again.

use std::sync::atomic::Ordering;

use burrow_api::SessionId;
use tokio::sync::{mpsc, watch};
use tokio::task::AbortHandle;

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
    /// behind, in which case the page attaches again, when shells lock, and
    /// on [`Bridge::detach`].
    pub async fn attach(&self, session: SessionId) -> Result<(Vec<u8>, mpsc::Receiver<Vec<u8>>)> {
        let locks = self.inner.locks.subscribe();
        self.check_unlocked()?;
        let (replay, stream) = self.conn()?.attach(session).await?;
        // Locked while attaching: nothing reaches the page.
        self.check_unlocked()?;
        let (rx, task) = forward(stream, locks);
        let earlier = self.inner.streams.lock().unwrap().insert(session, task);
        if let Some(earlier) = earlier {
            earlier.abort();
        }
        Ok((replay, rx))
    }

    /// The page stopped showing a shell: its stream goes, which detaches
    /// it in the daemon, and the terminal runs on.
    pub fn detach(&self, session: SessionId) {
        if let Some(task) = self.inner.streams.lock().unwrap().remove(&session) {
            task.abort();
        }
    }
}

/// Passes `stream` on until it ends, the page stops reading, or shells
/// lock. Dropping `stream` then detaches it in the daemon.
fn forward(
    mut stream: mpsc::Receiver<Vec<u8>>,
    mut locks: watch::Receiver<()>,
) -> (mpsc::Receiver<Vec<u8>>, AbortHandle) {
    let (tx, rx) = mpsc::channel(1);
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                chunk = stream.recv() => {
                    let Some(chunk) = chunk else { return };
                    if tx.send(chunk).await.is_err() {
                        return;
                    }
                }
                _ = locks.changed() => return,
            }
        }
    });
    (rx, task.abort_handle())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn output_passes_until_shells_lock() {
        let (tx, stream) = mpsc::channel(4);
        let (lock, locks) = watch::channel(());
        let (mut rx, _task) = forward(stream, locks);
        tx.send(b"before".to_vec()).await.unwrap();
        assert_eq!(rx.recv().await.unwrap(), b"before");
        lock.send_replace(());
        assert_eq!(rx.recv().await, None);
        // The daemon's stream was let go of, which detaches it there.
        assert!(tx.send(b"after".to_vec()).await.is_err());
    }

    #[tokio::test]
    async fn aborting_lets_go_of_the_stream() {
        let (tx, stream) = mpsc::channel(4);
        let (_lock, locks) = watch::channel(());
        let (mut rx, task) = forward(stream, locks);
        task.abort();
        assert_eq!(rx.recv().await, None);
        assert!(tx.send(b"after".to_vec()).await.is_err());
    }
}
