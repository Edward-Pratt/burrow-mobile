//! The hop to `burrowd`: SSH to the paired computer as the user, signed in
//! with the phone's key, then an exec channel that the key's forced
//! command turns into `burrow proxy --role mobile`. The channel's two
//! halves become a `burrow_api::Client`, so from here the phone speaks the
//! daemon's own protocol.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use burrow_api::Client;
use burrow_api::rpc::Role;
use russh::Preferred;
use russh::client::{self, AuthResult, Handle};
use russh::keys::{PrivateKeyWithHashAlg, PublicKey, PublicKeyOrCertificate};
use tokio::net::TcpStream;

use crate::hostkey::{self, Pin};
use crate::keys::PhoneKey;
use crate::pairing::Pairing;
use crate::{Error, Result};

/// What the exec channel asks for. The key's forced command runs the
/// proxy whatever this says; it names it for a key without one.
pub const PROXY: &str = "burrow proxy --role mobile";

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Mobile networks drop idle connections, and a dead one should be noticed
/// within a minute rather than at the next request.
const KEEPALIVE: Duration = Duration::from_secs(15);

/// A connection to the daemon. Dropping it closes the SSH connection.
pub struct Link {
    client: Client,
    _ssh: Handle<Handler>,
}

impl Link {
    /// Connects to the paired computer. A computer not pinned yet fails
    /// with [`Error::NewHostKey`], and a changed key with
    /// [`Error::HostKeyChanged`], before the phone's key is offered.
    pub async fn connect(pairing: &Pairing, key: &PhoneKey) -> Result<Link> {
        let pinned = pairing
            .host_key
            .as_deref()
            .map(hostkey::parse)
            .transpose()?;
        let address = pairing
            .address
            .trim_start_matches('[')
            .trim_end_matches(']');
        let socket =
            tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect((address, pairing.port)))
                .await
                .map_err(|_| Error::Unreachable(format!("{address}: no answer")))?
                .map_err(|e| Error::Unreachable(format!("{address}: {e}")))?;

        let verdict = Arc::new(Mutex::new(None));
        let handler = Handler {
            pinned: pinned.clone(),
            verdict: verdict.clone(),
        };
        let mut preferred = Preferred::default();
        preferred.key = hostkey::preferred(pinned.as_ref(), &preferred.key).into();
        let config = client::Config {
            keepalive_interval: Some(KEEPALIVE),
            preferred,
            ..Default::default()
        };
        let mut ssh = match client::connect_stream(Arc::new(config), socket, handler).await {
            Ok(ssh) => ssh,
            Err(e) => {
                return Err(
                    refusal(verdict.lock().unwrap().take()).unwrap_or(Error::Ssh(e.to_string()))
                );
            }
        };

        let signer = PrivateKeyWithHashAlg::new(Arc::new(key.private().clone()), None);
        match ssh
            .authenticate_publickey(pairing.user.to_string(), signer)
            .await
        {
            Ok(AuthResult::Success) => {}
            Ok(AuthResult::Failure { .. }) => return Err(Error::KeyRefused),
            Err(e) => return Err(Error::Ssh(e.to_string())),
        }
        let channel = ssh
            .channel_open_session()
            .await
            .map_err(|e| Error::Ssh(e.to_string()))?;
        channel
            .exec(true, PROXY)
            .await
            .map_err(|e| Error::Ssh(e.to_string()))?;
        let (read, write) = tokio::io::split(channel.into_stream());
        let client = Client::over(read, write, Role::Mobile)
            .await
            .map_err(|e| Error::Daemon(e.to_string()))?;
        Ok(Link { client, _ssh: ssh })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }
}

/// Why the handler turned the host key down, as the error to show.
fn refusal(verdict: Option<Pin>) -> Option<Error> {
    match verdict? {
        Pin::Unknown { offered } => Some(Error::NewHostKey {
            fingerprint: hostkey::fingerprint(&offered),
            key: hostkey::to_line(&offered),
        }),
        Pin::Mismatch { pinned, offered } => Some(Error::HostKeyChanged {
            pinned: hostkey::fingerprint(&pinned),
            offered: hostkey::fingerprint(&offered),
        }),
        Pin::Matches => None,
    }
}

pub struct Handler {
    pinned: Option<PublicKey>,
    verdict: Arc<Mutex<Option<Pin>>>,
}

impl client::Handler for Handler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        key: &PublicKeyOrCertificate,
    ) -> std::result::Result<bool, Self::Error> {
        // A host certificate would need a CA the phone trusts, which it
        // doesn't have, so only a plain pinned key gets through.
        let PublicKeyOrCertificate::PublicKey { key, .. } = key else {
            return Ok(false);
        };
        let pin = hostkey::decide(self.pinned.as_ref(), key);
        let accepted = pin == Pin::Matches;
        *self.verdict.lock().unwrap() = Some(pin);
        Ok(accepted)
    }
}
