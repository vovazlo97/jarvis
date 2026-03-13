use serde::{Deserialize, Serialize};

use crate::state::AssistantState;

/// Internal event bus events — decoupled module-to-module communication.
/// Separate from `IpcEvent` (which is for GUI/websocket communication).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum JarvisEvent {
    /// Wake word detected — pipeline starts.
    WakeWordDetected,

    /// Active recording started.
    Listening,

    /// STT produced a transcript.
    SpeechRecognized { text: String },

    /// Intent classifier matched a command.
    CommandRecognized { id: String, text: String },

    /// Command dispatcher finished (success or failure).
    CommandExecuted { id: String, success: bool },

    /// High-level assistant state changed.
    StateChanged { state: AssistantState },

    /// Error in any pipeline stage.
    Error { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_is_clone() {
        let e = JarvisEvent::WakeWordDetected;
        let _ = e.clone();
    }

    #[test]
    fn test_event_is_debug() {
        let e = JarvisEvent::SpeechRecognized {
            text: "test".into(),
        };
        assert!(format!("{:?}", e).contains("SpeechRecognized"));
    }

    #[test]
    fn test_event_command_executed() {
        let e = JarvisEvent::CommandExecuted {
            id: "open_browser".into(),
            success: true,
        };
        match e {
            JarvisEvent::CommandExecuted { id, success } => {
                assert_eq!(id, "open_browser");
                assert!(success);
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn test_error_event_has_message() {
        let e = JarvisEvent::Error {
            message: "oops".into(),
        };
        match e {
            JarvisEvent::Error { message } => assert_eq!(message, "oops"),
            _ => panic!("wrong variant"),
        }
    }
}
