//! What the webview may ask the daemon. The page is the app's own, but a
//! bug or an injected script there should reach no more than the screens
//! need: hosts, alerts, monitors and shells, but no files, commands,
//! secrets or changes to nodes. Shells also wait for the device unlock,
//! which the bridge checks with [`needs_unlock`].

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
            // A login shell only: `run` would control services.
            | Command::OpenTerminal { run: None, .. }
            | Command::KeepTerminal { .. }
            | Command::WriteTerminal { .. }
            | Command::ResizeTerminal { .. }
    )
}

/// Whether `command` reaches a shell, so it waits for the device unlock.
pub fn needs_unlock(command: &Command) -> bool {
    matches!(
        command,
        Command::OpenTerminal { .. }
            | Command::KeepTerminal { .. }
            | Command::WriteTerminal { .. }
            | Command::ResizeTerminal { .. }
    )
}

/// The session a command acts on, for the ones that may only act on a
/// session the phone opened itself, never on the desktop's terminals or
/// their prompts.
pub fn own_session(command: &Command) -> Option<SessionId> {
    match command {
        Command::CloseSession { session }
        | Command::AnswerPrompt { session, .. }
        | Command::KeepTerminal { session }
        | Command::WriteTerminal { session, .. }
        | Command::ResizeTerminal { session, .. } => Some(*session),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NODE: &str = "00000000-0000-0000-0000-000000000001";

    fn parse(json: &str) -> Command {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn the_screens_commands_are_allowed() {
        for json in [
            r#"{"type":"list_nodes"}"#.to_string(),
            r#"{"type":"list_alerts"}"#.to_string(),
            r#"{"type":"list_monitoring"}"#.to_string(),
            r#"{"type":"list_sessions"}"#.to_string(),
            r#"{"type":"close_session","session":3}"#.to_string(),
            format!(r#"{{"type":"open_terminal","node":"{NODE}","cols":80,"rows":24}}"#),
            r#"{"type":"keep_terminal","session":3}"#.to_string(),
            r#"{"type":"write_terminal","session":3,"data":"bHMK"}"#.to_string(),
            r#"{"type":"resize_terminal","session":3,"cols":40,"rows":20}"#.to_string(),
        ] {
            assert!(allowed(&parse(&json)), "{json}");
        }
    }

    #[test]
    fn session_commands_name_their_session() {
        let close = parse(r#"{"type":"close_session","session":3}"#);
        assert_eq!(own_session(&close), Some(SessionId(3)));
        let answer =
            parse(r#"{"type":"answer_prompt","session":4,"prompt":1,"answer":{"type":"confirm"}}"#);
        assert_eq!(own_session(&answer), Some(SessionId(4)));
        let write = parse(r#"{"type":"write_terminal","session":5,"data":"bHMK"}"#);
        assert_eq!(own_session(&write), Some(SessionId(5)));
        assert_eq!(own_session(&parse(r#"{"type":"list_nodes"}"#)), None);
    }

    #[test]
    fn shells_wait_for_the_unlock_and_monitors_do_not() {
        let open = format!(r#"{{"type":"open_terminal","node":"{NODE}","cols":80,"rows":24}}"#);
        assert!(needs_unlock(&parse(&open)));
        let write = parse(r#"{"type":"write_terminal","session":5,"data":"bHMK"}"#);
        assert!(needs_unlock(&write));
        let monitor = format!(r#"{{"type":"open_monitor","node":"{NODE}"}}"#);
        assert!(!needs_unlock(&parse(&monitor)));
        assert!(!needs_unlock(&parse(r#"{"type":"list_sessions"}"#)));
    }

    #[test]
    fn anything_else_that_runs_reads_or_changes_is_refused() {
        let service = r#"{"kind":"unit","name":"nginx.service"}"#;
        for json in [
            format!(r#"{{"type":"exec","node":"{NODE}","command":"id"}}"#),
            format!(
                r#"{{"type":"open_terminal","node":"{NODE}","cols":80,"rows":24,"run":{{"kind":"logs","service":{service}}}}}"#
            ),
            format!(r#"{{"type":"open_files","node":"{NODE}"}}"#),
            format!(r#"{{"type":"delete_node","id":"{NODE}"}}"#),
            format!(r#"{{"type":"set_monitoring","node":"{NODE}","enabled":true}}"#),
        ] {
            assert!(!allowed(&parse(&json)), "{json}");
        }
    }
}
