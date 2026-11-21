//! One live connection to the daemon, with the sessions the page may
//! close, answer prompts for or type into: the ones this connection opened,
//! and the terminals the phone kept, which outlive the connection that
//! opened them. Those are learned from listing sessions on each connection,
//! where the daemon names who kept each one, never stored, since a daemon
//! that restarted may have handed out the same ids again.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use burrow_api::{Client, Command, Event, Response, Role, SessionId, SessionKind, SessionState};
use tokio::sync::mpsc;

use crate::allow::own_session;
use crate::{Error, Result};

#[derive(Clone)]
pub(crate) struct Conn {
    client: Client,
    opened: Arc<Mutex<Opened>>,
}

impl Conn {
    pub(crate) fn new(client: Client) -> Conn {
        Conn {
            client,
            opened: Arc::default(),
        }
    }

    /// Runs `command`, refusing one aimed at a session this connection
    /// didn't open, and keeping the set of opened sessions up to date.
    pub(crate) async fn execute(&self, command: Command) -> Result<Response> {
        self.opened.lock().unwrap().check(&command)?;
        let closing = match command {
            Command::CloseSession { session } => Some(session),
            _ => None,
        };
        let response = self
            .client
            .execute(command)
            .await
            .map_err(|e| Error::Daemon(e.to_string()))?;
        self.opened.lock().unwrap().record(&response, closing);
        Ok(response)
    }

    /// Attaches to a kept terminal the page may use: what it kept, then its
    /// output.
    pub(crate) async fn attach(&self, session: SessionId) -> Result<Attached> {
        self.opened.lock().unwrap().owns(session)?;
        self.client
            .attach_output(session)
            .await
            .map_err(|e| Error::Daemon(e.to_string()))
    }

    /// Takes note of a session that ended, which is no longer the page's.
    pub(crate) fn saw(&self, event: &Event) {
        self.opened.lock().unwrap().saw(event);
    }
}

/// A kept terminal's replay, and its output from there on.
pub(crate) type Attached = (Vec<u8>, mpsc::Receiver<Vec<u8>>);

#[derive(Default)]
struct Opened(HashSet<SessionId>);

impl Opened {
    fn check(&self, command: &Command) -> Result<()> {
        match own_session(command) {
            Some(session) => self.owns(session),
            None => Ok(()),
        }
    }

    fn owns(&self, session: SessionId) -> Result<()> {
        if !self.0.contains(&session) {
            return Err(Error::invalid("Session", "isn't one the phone opened"));
        }
        Ok(())
    }

    fn record(&mut self, response: &Response, closed: Option<SessionId>) {
        match response {
            Response::Session { session } => {
                self.0.insert(*session);
            }
            // A terminal the CLI or the window kept isn't the page's to
            // type into, nor is one from a daemon too old to say who kept it.
            Response::Sessions { sessions } => {
                let kept = sessions.iter().filter(|s| {
                    s.kept && s.kind == SessionKind::Terminal && s.kept_by == Some(Role::Mobile)
                });
                self.0.extend(kept.map(|s| s.session));
            }
            _ => {}
        }
        if let Some(session) = closed {
            self.0.remove(&session);
        }
    }

    fn saw(&mut self, event: &Event) {
        if let Event::SessionChanged {
            session,
            state: SessionState::Closed { .. },
            ..
        } = event
        {
            self.0.remove(session);
        }
    }
}

#[cfg(test)]
mod tests {
    use burrow_api::{NodeId, PromptAnswer, SessionInfo};

    use super::*;

    fn answer(session: u64) -> Command {
        Command::AnswerPrompt {
            session: SessionId(session),
            prompt: 1,
            answer: PromptAnswer::Confirm,
        }
    }

    #[test]
    fn only_opened_sessions_are_the_pages() {
        let mut opened = Opened::default();
        assert!(opened.check(&answer(3)).is_err());
        opened.record(
            &Response::Session {
                session: SessionId(3),
            },
            None,
        );
        assert!(opened.check(&answer(3)).is_ok());
        assert!(opened.check(&answer(4)).is_err());
        assert!(opened.check(&Command::ListNodes).is_ok());
    }

    #[test]
    fn only_terminals_the_phone_kept_are_the_pages() {
        let mut opened = Opened::default();
        let listed = |session: u64, kept_by: Option<Role>| SessionInfo {
            session: SessionId(session),
            node: NodeId(Default::default()),
            kind: SessionKind::Terminal,
            state: SessionState::Connected,
            background: false,
            kept: kept_by.is_some(),
            kept_by,
            attached: false,
        };
        let mut desktop = listed(8, Some(Role::Mobile));
        desktop.kind = SessionKind::Vnc;
        // An older daemon says a terminal is kept, but not by whom.
        let mut unnamed = listed(11, None);
        unnamed.kept = true;
        let sessions = vec![
            listed(6, Some(Role::Mobile)),
            listed(7, None),
            desktop,
            listed(9, Some(Role::Cli)),
            listed(10, Some(Role::Gui)),
            unnamed,
        ];
        opened.record(&Response::Sessions { sessions }, None);
        assert!(opened.owns(SessionId(6)).is_ok());
        for other in [7, 8, 9, 10, 11] {
            assert!(opened.owns(SessionId(other)).is_err(), "{other}");
        }
    }

    #[test]
    fn a_session_stops_being_the_pages_once_closed_or_ended() {
        let mut opened = Opened::default();
        opened.record(
            &Response::Session {
                session: SessionId(3),
            },
            None,
        );
        opened.record(&Response::Ok, Some(SessionId(3)));
        assert!(opened.check(&answer(3)).is_err());

        opened.record(
            &Response::Session {
                session: SessionId(5),
            },
            None,
        );
        opened.saw(&Event::SessionChanged {
            session: SessionId(5),
            node: NodeId(Default::default()),
            state: SessionState::Closed { error: None },
        });
        assert!(opened.check(&answer(5)).is_err());
    }
}
