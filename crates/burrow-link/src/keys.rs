//! The phone's own SSH key: made on first run, kept in the app's private
//! folder, and never handed to the webview. The pairing is this key's
//! `authorized_keys` line on the computer running `burrowd`.

use std::path::{Path, PathBuf};

use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, HashAlg, PrivateKey};

use crate::files::{read_if_there, write_private};
use crate::pairing::User;
use crate::{Error, Result};

const FILE: &str = "id_ed25519";
const COMMENT: &str = "burrow-mobile";

/// The options DESIGN.md gives the phone's key: it can only ever run the
/// proxy, never a shell or a forward.
const OPTIONS: &str = "no-port-forwarding,no-pty,no-agent-forwarding,no-X11-forwarding";

pub struct PhoneKey {
    key: PrivateKey,
}

impl PhoneKey {
    /// The key in `dir`, made and saved there first if there is none.
    pub fn load_or_create(dir: &Path) -> Result<PhoneKey> {
        let path = dir.join(FILE);
        if let Some(text) = read_if_there(&path)? {
            let key = PrivateKey::from_openssh(text.as_bytes())
                .map_err(|e| Error::Damaged(path.clone(), e.to_string()))?;
            if key.algorithm() != Algorithm::Ed25519 {
                return Err(Error::Damaged(path, "not an ed25519 key".into()));
            }
            return Ok(PhoneKey { key });
        }
        let mut key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
            .map_err(|e| Error::Key(e.to_string()))?;
        key.set_comment(COMMENT);
        let pem = key
            .to_openssh(LineEnding::LF)
            .map_err(|e| Error::Key(e.to_string()))?;
        write_private(&path, pem.as_bytes())?;
        Ok(PhoneKey { key })
    }

    /// Where the key is kept in `dir`.
    pub fn path(dir: &Path) -> PathBuf {
        dir.join(FILE)
    }

    /// For signing in; stays inside this crate.
    pub(crate) fn private(&self) -> &PrivateKey {
        &self.key
    }

    /// `ssh-ed25519 AAAA… burrow-mobile`.
    pub fn public_line(&self) -> String {
        let public = self.key.public_key();
        let base = public.to_openssh().unwrap_or_default();
        // to_openssh leaves the comment out when the key has none.
        match base.split(' ').count() {
            2 => format!("{base} {COMMENT}"),
            _ => base,
        }
    }

    /// `SHA256:…`, for the user to tell keys apart.
    pub fn fingerprint(&self) -> String {
        self.key
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string()
    }

    /// The line to add to `~/.ssh/authorized_keys` for `user` on the
    /// computer. The path is absolute because sshd runs a forced command
    /// through the user's shell non-interactively, where `~/.local/bin` is
    /// usually not on `PATH`.
    pub fn authorized_keys_line(&self, user: &User) -> String {
        format!(
            "command=\"/home/{user}/.local/bin/burrow proxy --role mobile\",{OPTIONS} {}",
            self.public_line()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    #[test]
    fn a_key_is_made_once_and_kept_private() {
        let dir = tempfile::tempdir().unwrap();
        let first = PhoneKey::load_or_create(dir.path()).unwrap();
        let again = PhoneKey::load_or_create(dir.path()).unwrap();
        assert_eq!(first.public_line(), again.public_line());
        let mode = std::fs::metadata(PhoneKey::path(dir.path()))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn the_line_limits_the_key_to_the_proxy() {
        let dir = tempfile::tempdir().unwrap();
        let key = PhoneKey::load_or_create(dir.path()).unwrap();
        let user = User::parse("edward").unwrap();
        let line = key.authorized_keys_line(&user);
        assert!(line.starts_with(
            "command=\"/home/edward/.local/bin/burrow proxy --role mobile\",\
             no-port-forwarding,no-pty,no-agent-forwarding,no-X11-forwarding ssh-ed25519 AAAA"
        ));
        assert!(line.ends_with(" burrow-mobile"));
        assert!(key.fingerprint().starts_with("SHA256:"));
    }

    #[test]
    fn a_damaged_key_file_is_reported_not_replaced() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(PhoneKey::path(dir.path()), "not a key").unwrap();
        assert!(matches!(
            PhoneKey::load_or_create(dir.path()),
            Err(Error::Damaged(..))
        ));
        assert_eq!(
            std::fs::read_to_string(PhoneKey::path(dir.path())).unwrap(),
            "not a key"
        );
    }
}
