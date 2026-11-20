//! The computer's host key, pinned once the user has confirmed it. The
//! phone keeps no `known_hosts` but this one key: a new computer's key is
//! shown as a fingerprint for the user to compare with
//! `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` there, and a key
//! that differs from the pinned one is refused, never re-pinned silently.

use russh::keys::{Algorithm, HashAlg, PublicKey};

use crate::{Error, Result};

/// What the offered key means for the connection.
#[derive(Debug, PartialEq, Eq)]
pub enum Pin {
    /// Nothing is pinned yet. The user must confirm this key first.
    Unknown {
        offered: PublicKey,
    },
    Matches,
    Mismatch {
        pinned: PublicKey,
        offered: PublicKey,
    },
}

/// The pinned key, from its OpenSSH form (`ssh-ed25519 AAAA…`).
pub fn parse(line: &str) -> Result<PublicKey> {
    PublicKey::from_openssh(line.trim())
        .map_err(|e| Error::invalid("Host key", format!("isn't an OpenSSH public key: {e}")))
}

/// The key's OpenSSH form without a comment, as the pairing keeps it.
pub fn to_line(key: &PublicKey) -> String {
    let mut key = key.clone();
    key.set_comment("");
    key.to_openssh().unwrap_or_default()
}

pub fn fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

/// Compares the key the server offered with the pinned one. Comments
/// don't count: only the key itself.
pub fn decide(pinned: Option<&PublicKey>, offered: &PublicKey) -> Pin {
    match pinned {
        None => Pin::Unknown {
            offered: offered.clone(),
        },
        Some(pinned) if pinned.key_data() == offered.key_data() => Pin::Matches,
        Some(pinned) => Pin::Mismatch {
            pinned: pinned.clone(),
            offered: offered.clone(),
        },
    }
}

/// The host key types to ask for, the pinned one's first, so a server with
/// several keys offers the one the phone knows rather than one that would
/// look like a change.
pub fn preferred(pinned: Option<&PublicKey>, defaults: &[Algorithm]) -> Vec<Algorithm> {
    let mut algorithms = defaults.to_vec();
    if let Some(pinned) = pinned {
        let first = pinned.algorithm();
        algorithms.retain(|a| *a != first);
        algorithms.insert(0, first);
    }
    algorithms
}

#[cfg(test)]
mod tests {
    use russh::keys::PrivateKey;

    use super::*;

    fn key() -> PublicKey {
        PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519)
            .unwrap()
            .public_key()
            .clone()
    }

    #[test]
    fn a_new_computer_is_unknown_until_confirmed() {
        let offered = key();
        assert_eq!(
            decide(None, &offered),
            Pin::Unknown {
                offered: offered.clone()
            }
        );
    }

    #[test]
    fn the_pinned_key_matches_whatever_its_comment() {
        let pinned = key();
        let line = to_line(&pinned);
        let mut offered = parse(&line).unwrap();
        offered.set_comment("root@warren");
        assert_eq!(decide(Some(&pinned), &offered), Pin::Matches);
        assert!(fingerprint(&pinned).starts_with("SHA256:"));
    }

    #[test]
    fn another_key_is_a_mismatch() {
        let (pinned, offered) = (key(), key());
        assert!(matches!(
            decide(Some(&pinned), &offered),
            Pin::Mismatch { .. }
        ));
    }

    #[test]
    fn the_pinned_type_is_asked_for_first() {
        let defaults = [Algorithm::Rsa { hash: None }, Algorithm::Ed25519];
        assert_eq!(
            preferred(Some(&key()), &defaults),
            vec![Algorithm::Ed25519, Algorithm::Rsa { hash: None }]
        );
        assert_eq!(preferred(None, &defaults), defaults.to_vec());
    }

    #[test]
    fn a_bad_pin_is_refused() {
        assert!(parse("not a key").is_err());
    }
}
