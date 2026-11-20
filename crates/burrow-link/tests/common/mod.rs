//! A throwaway `sshd` on 127.0.0.1 whose forced command is a small
//! stand-in for `burrow proxy`: it answers `initialize` and one `execute`,
//! and records what it was sent. There is no real `burrowd` here, since it
//! needs a Secret Service. Each test file uses only some of it.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as Process, Stdio};
use std::time::Duration;

use burrow_link::keys::PhoneKey;
use burrow_link::pairing::Pairing;
use tempfile::TempDir;

pub const SSHDS: [&str; 2] = ["/usr/bin/sshd", "/usr/sbin/sshd"];

/// Answers like the proxy would after a good `initialize`. `id` is the
/// request's, copied back.
const FAKE_PROXY: &str = r#"#!/bin/sh
printf '%s\n' "$SSH_ORIGINAL_COMMAND" > "$DIR/command"
read -r line
printf '%s\n' "$line" > "$DIR/initialize"
id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
printf '{"jsonrpc":"2.0","id":%s,"result":{"api":1,"build":"fake","addons_dir":"/x"}}\n' "$id"
read -r line
id=$(printf '%s' "$line" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
printf '{"jsonrpc":"2.0","id":%s,"result":{"type":"nodes","nodes":[]}}\n' "$id"
cat > /dev/null
"#;

pub struct Server {
    pub dir: TempDir,
    pub port: u16,
    sshd: Child,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.sshd.kill();
        let _ = self.sshd.wait();
    }
}

impl Server {
    /// An sshd that lets `key` in only through the fake proxy.
    pub fn start(key: &PhoneKey) -> Option<Server> {
        let sshd = SSHDS.into_iter().find(|p| Path::new(p).exists())?;
        let dir = TempDir::new().unwrap();
        let path = |name: &str| dir.path().join(name);
        let status = Process::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", "", "-f"])
            .arg(path("host_key"))
            .status()
            .unwrap();
        assert!(status.success());
        let script = path("proxy");
        fs::write(
            &script,
            FAKE_PROXY.replace("$DIR", &dir.path().display().to_string()),
        )
        .unwrap();
        Process::new("chmod")
            .arg("755")
            .arg(&script)
            .status()
            .unwrap();
        let line = format!(
            "command=\"{}\",no-port-forwarding,no-pty,no-agent-forwarding,no-X11-forwarding {}\n",
            script.display(),
            key.public_line()
        );
        fs::write(path("authorized_keys"), line).unwrap();

        let port = free_port();
        fs::write(
            path("sshd_config"),
            format!(
                "Port {port}\nListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\n\
                 PasswordAuthentication no\nKbdInteractiveAuthentication no\nUsePAM no\n\
                 StrictModes no\nPidFile none\nLogLevel ERROR\n",
                path("host_key").display(),
                path("authorized_keys").display(),
            ),
        )
        .unwrap();
        // sshd re-executes itself, so it is started by absolute path.
        let sshd = Process::new(sshd)
            .args(["-D", "-e", "-f"])
            .arg(path("sshd_config"))
            .stdin(Stdio::null())
            .spawn()
            .unwrap();
        let server = Server { dir, port, sshd };
        for _ in 0..100 {
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                return Some(server);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("sshd did not start on port {port}");
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    pub fn host_key(&self) -> String {
        fs::read_to_string(self.path("host_key.pub")).unwrap()
    }

    pub fn pairing(&self, pinned: Option<String>) -> Pairing {
        let mut pairing = Pairing::new("127.0.0.1", self.port, &whoami()).unwrap();
        pairing.host_key = pinned;
        pairing
    }
}

pub fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub fn whoami() -> String {
    let out = Process::new("id").arg("-un").output().unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

pub fn phone_key() -> (TempDir, PhoneKey) {
    let dir = TempDir::new().unwrap();
    let key = PhoneKey::load_or_create(dir.path()).unwrap();
    (dir, key)
}
