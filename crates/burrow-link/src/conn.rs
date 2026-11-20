//! One live connection to the daemon, with the sessions `open_monitor`
//! gave it: the only ones the page may close or answer prompts for. They
//! go together, so an answer from an old connection can't add to a new
//! one's, whose daemon may have restarted and handed out the same ids.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use burrow_api::{Client, Command, Event, Response, SessionId, SessionState};

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

    /// Takes note of a session that ended, which is no longer the page's.
    pub(crate) fn saw(&self, event: &Event) {
        self.opened.lock().unwrap().saw(event);
    }
}

#[derive(Default)]
struct Opened(HashSet<SessionId>);

impl Opened {
    fn check(&self, command: &Command) -> Result<()> {
        match own_session(command) {
            Some(session) if !self.0.contains(&session) => {
                Err(Error::invalid("Session", "isn't one the phone opened"))
            }
            _ => Ok(()),
        }
    }

    fn record(&mut self, response: &Response, closed: Option<SessionId>) {
        if let Response::Session { session } = response {
            self.0.insert(*session);
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
    use burrow_api::{NodeId, PromptAnswer};

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
