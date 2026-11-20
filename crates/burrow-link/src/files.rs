//! Writing the app's own files: private to the app, and never half-written.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::{Error, Result};

/// Writes `data` to `path` readable by the app alone, through a file beside
/// it and a rename, so a crash leaves the old file or the new one.
pub(crate) fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let tmp = path.with_extension("new");
    let _ = std::fs::remove_file(&tmp);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .map_err(|e| Error::io(&tmp, e))?;
    file.write_all(data)
        .and_then(|()| file.sync_all())
        .map_err(|e| Error::io(&tmp, e))?;
    std::fs::rename(&tmp, path).map_err(|e| Error::io(path, e))
}

/// Reads `path`, or `None` when it isn't there yet.
pub(crate) fn read_if_there(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(path, e)),
    }
}
