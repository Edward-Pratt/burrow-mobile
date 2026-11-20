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
