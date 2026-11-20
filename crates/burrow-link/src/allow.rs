//! What the webview may ask the daemon. The page is the app's own, but a
//! bug or an injected script there should reach no more than v1 needs: no
//! terminals, files, commands, secrets or changes to nodes.

use burrow_api::{Command, SessionId};

/// Whether `command` is one the phone's screens use.
pub fn allowed(command: &Command) -> bool {
    matches!(
        command,
        Command::ListNodes
            | Command::ListAlerts
            | Command::ListMonitoring
            | Command::ListSessions
            | Command::OpenMonitor { .. }
            | Command::CloseSession { .. }
            | Command::MonitorTrend { .. }
            | Command::AnswerPrompt { .. }
    )
}

/// The session a command acts on, for the ones that may only act on a
/// session the phone opened itself, never on the desktop's terminals or
/// their prompts.
pub fn own_session(command: &Command) -> Option<SessionId> {
    match command {
        Command::CloseSession { session } | Command::AnswerPrompt { session, .. } => Some(*session),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(json: &str) -> Command {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_screens_commands_are_allowed() {
        for json in [
            r#"{"type":"list_nodes"}"#,
            r#"{"type":"list_alerts"}"#,
            r#"{"type":"list_monitoring"}"#,
            r#"{"type":"list_sessions"}"#,
            r#"{"type":"close_session","session":3}"#,
        ] {
            assert!(allowed(&parse(json)), "{json}");
        }
    }

    #[test]
    fn closing_and_answering_name_their_session() {
        let close = parse(r#"{"type":"close_session","session":3}"#);
        assert_eq!(own_session(&close), Some(SessionId(3)));
        let answer =
            parse(r#"{"type":"answer_prompt","session":4,"prompt":1,"answer":{"type":"confirm"}}"#);
        assert_eq!(own_session(&answer), Some(SessionId(4)));
        assert_eq!(own_session(&parse(r#"{"type":"list_nodes"}"#)), None);
    }

    #[test]
    fn anything_that_runs_reads_or_changes_is_refused() {
        let node = "00000000-0000-0000-0000-000000000001";
        for json in [
            format!(r#"{{"type":"exec","node":"{node}","command":"id"}}"#),
            format!(r#"{{"type":"open_terminal","node":"{node}","cols":80,"rows":24}}"#),
            format!(r#"{{"type":"open_files","node":"{node}"}}"#),
            format!(r#"{{"type":"delete_node","id":"{node}"}}"#),
            format!(r#"{{"type":"set_monitoring","node":"{node}","enabled":true}}"#),
        ] {
            assert!(!allowed(&parse(&json)), "{json}");
        }
    }
}
