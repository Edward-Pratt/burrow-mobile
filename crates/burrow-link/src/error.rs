//! What can go wrong, as messages the app can show.

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("the phone's key: {0}")]
    Key(String),
    #[error("{field} {reason}")]
    Invalid { field: &'static str, reason: String },
    #[error("{0} is damaged: {1}")]
    Damaged(PathBuf, String),
    /// The computer isn't pinned yet: the user must compare `fingerprint`
    /// with its host key, then pin `key`.
    #[error("new computer: check that its host key is {fingerprint}")]
    NewHostKey { fingerprint: String, key: String },
    #[error(
        "the computer's host key changed: it was {pinned}, now it is {offered}. \
         If you didn't reinstall it or change its keys, someone may be in between"
    )]
    HostKeyChanged { pinned: String, offered: String },
    #[error("couldn't reach {0}")]
    Unreachable(String),
    #[error("the computer refused the phone's key: add its line to ~/.ssh/authorized_keys there")]
    KeyRefused,
    #[error("SSH: {0}")]
    Ssh(String),
    #[error("unlock the phone to use shells")]
    Locked,
    #[error("burrowd: {0}")]
    Daemon(String),
}

impl Error {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub(crate) fn invalid(field: &'static str, reason: impl Into<String>) -> Self {
        Error::Invalid {
            field,
            reason: reason.into(),
        }
    }
}
