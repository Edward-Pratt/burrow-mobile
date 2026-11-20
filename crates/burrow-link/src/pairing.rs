//! The computer the phone is paired with: where it is, who to sign in as,
//! and its host key once the user has confirmed the fingerprint. The values
//! are checked before use, since the user name ends up inside the forced
//! command's path.

use std::fmt;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::files::{read_if_there, write_private};
use crate::{Error, Result};

const FILE: &str = "pairing.json";

/// A user name as `useradd` accepts it by default, so it can't add a
/// quote, a space or a path to the `authorized_keys` line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct User(String);

impl User {
    pub fn parse(name: &str) -> Result<User> {
        let mut chars = name.chars();
        let first_ok = chars
            .next()
            .is_some_and(|c| c.is_ascii_lowercase() || c == '_');
        let rest_ok =
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-".contains(c));
        if name.len() > 32 || !first_ok || !rest_ok {
            return Err(Error::invalid(
                "User",
                "must start with a lower-case letter or _, then letters, digits, _ or -, up to 32",
            ));
        }
        Ok(User(name.to_owned()))
    }
}

impl TryFrom<String> for User {
    type Error = Error;
    fn try_from(name: String) -> Result<User> {
        User::parse(&name)
    }
}

impl From<User> for String {
    fn from(user: User) -> String {
        user.0
    }
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where the computer is: a host name or an IP address, as typed.
pub fn check_address(address: &str) -> Result<()> {
    let ok = !address.is_empty()
        && address.len() <= 253
        && !address.starts_with('-')
        && address
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-:[]".contains(c));
    if ok {
        Ok(())
    } else {
        Err(Error::invalid(
            "Address",
            "must be a host name or an IP address",
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pairing {
    pub address: String,
    pub port: u16,
    pub user: User,
    /// The computer's host key in OpenSSH form (`ssh-ed25519 AAAA…`), once
    /// the user has confirmed its fingerprint. Until then, nothing is sent.
    pub host_key: Option<String>,
}

impl Pairing {
    pub fn new(address: &str, port: u16, user: &str) -> Result<Pairing> {
        check_address(address)?;
        if port == 0 {
            return Err(Error::invalid("Port", "can't be 0"));
        }
        Ok(Pairing {
            address: address.to_owned(),
            port,
            user: User::parse(user)?,
            host_key: None,
        })
    }

    /// The saved pairing, or `None` before the first one.
    pub fn load(dir: &Path) -> Result<Option<Pairing>> {
        let path = dir.join(FILE);
        let Some(text) = read_if_there(&path)? else {
            return Ok(None);
        };
        let pairing: Pairing =
            serde_json::from_str(&text).map_err(|e| Error::Damaged(path.clone(), e.to_string()))?;
        // The file is the app's own, but a hand-edited one gets the same
        // checks as typed values.
        check_address(&pairing.address).map_err(|e| Error::Damaged(path, e.to_string()))?;
        Ok(Some(pairing))
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let json = serde_json::to_vec_pretty(self).expect("a pairing always serializes");
        write_private(&dir.join(FILE), &json)
    }

    /// Forgets the pairing, for pairing again with another computer.
    pub fn forget(dir: &Path) -> Result<()> {
        let path = dir.join(FILE);
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(Error::io(path, e)),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_names_that_could_break_the_line_are_refused() {
        for bad in [
            "",
            "Edward",
            "ed ward",
            "ed\"",
            "-x",
            "a/b",
            "../x",
            &"a".repeat(33),
        ] {
            assert!(User::parse(bad).is_err(), "{bad:?}");
        }
        for good in ["edward", "_svc", "burrow-mon", "u1"] {
            assert!(User::parse(good).is_ok(), "{good:?}");
        }
    }

    #[test]
    fn addresses_are_host_names_or_ips() {
        for good in [
            "warren",
            "warren.example.ts.net",
            "100.96.1.2",
            "fd7a::1",
            "[fd7a::1]",
        ] {
            assert!(check_address(good).is_ok(), "{good:?}");
        }
        for bad in ["", "-oProxyCommand=x", "a b", "host;rm"] {
            assert!(check_address(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_pairing_round_trips_and_a_bad_file_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(Pairing::load(dir.path()).unwrap(), None);
        let mut pairing = Pairing::new("warren", 22, "edward").unwrap();
        pairing.host_key = Some("ssh-ed25519 AAAA".into());
        pairing.save(dir.path()).unwrap();
        assert_eq!(Pairing::load(dir.path()).unwrap(), Some(pairing));

        std::fs::write(
            dir.path().join(FILE),
            r#"{"address":"x","port":22,"user":"Bad","host_key":null}"#,
        )
        .unwrap();
        assert!(matches!(Pairing::load(dir.path()), Err(Error::Damaged(..))));
        Pairing::forget(dir.path()).unwrap();
        assert_eq!(Pairing::load(dir.path()).unwrap(), None);
    }
}
