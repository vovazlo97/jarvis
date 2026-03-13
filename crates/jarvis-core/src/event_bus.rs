use serde::Serialize;

use crate::state::AssistantState;

/// Internal event bus events — decoupled module-to-module communication.
/// Separate from `IpcEvent` (which is for GUI/websocket communication).
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum JarvisEvent {
    /// Wake word detected — pipeline starts.
    WakeWordDetected,

    /// Active recording started.
    Listening,

    /// STT produced a transcript.
    SpeechRecognized { text: String },

    /// Intent classifier matched a command. `utterance` is the original speech text.
    CommandRecognized { id: String, utterance: String },

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

    #[test]
    fn test_serde_unit_variant_tag() {
        let e = JarvisEvent::WakeWordDetected;
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"event":"wake_word_detected"}"#);
    }

    #[test]
    fn test_serde_struct_variant_tag() {
        let e = JarvisEvent::SpeechRecognized {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains(r#""event":"speech_recognized""#));
        assert!(json.contains(r#""text":"hello""#));
    }
}
