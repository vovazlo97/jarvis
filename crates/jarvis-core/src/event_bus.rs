use once_cell::sync::OnceCell;
use serde::Serialize;
use tokio::sync::broadcast;

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

/// Capacity of the internal event bus channel.
/// 64 slots: enough for burst during fast-path pipeline without dropping events.
const CHANNEL_CAPACITY: usize = 64;

static EVENT_BUS_TX: OnceCell<broadcast::Sender<JarvisEvent>> = OnceCell::new();

/// Initialize the Event Bus. Idempotent — safe to call multiple times.
/// Returns a clone of the sender.
pub fn init() -> broadcast::Sender<JarvisEvent> {
    if let Some(tx) = EVENT_BUS_TX.get() {
        return tx.clone();
    }
    let (tx, _) = broadcast::channel::<JarvisEvent>(CHANNEL_CAPACITY);
    EVENT_BUS_TX.set(tx.clone()).ok();
    log::info!(
        "EventBus: broadcast channel initialized (capacity={})",
        CHANNEL_CAPACITY
    );
    tx
}

/// Publish an event to all current subscribers.
/// Silently ignores send errors (no receivers = normal during startup/shutdown).
pub fn publish(event: JarvisEvent) {
    if let Some(tx) = EVENT_BUS_TX.get() {
        match tx.send(event) {
            Ok(n) => log::debug!("EventBus: published to {} subscriber(s)", n),
            Err(_) => {}
        }
    }
}

/// Subscribe to the Event Bus. Returns a `Receiver` that gets all future events.
/// Must call `init()` first, or this returns `None`.
pub fn subscribe() -> Option<broadcast::Receiver<JarvisEvent>> {
    EVENT_BUS_TX.get().map(|tx| tx.subscribe())
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

    #[tokio::test]
    async fn test_publish_subscribe_roundtrip() {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<JarvisEvent>(8);
        tx.send(JarvisEvent::WakeWordDetected).unwrap();
        let received = rx.recv().await.unwrap();
        assert!(matches!(received, JarvisEvent::WakeWordDetected));
    }

    #[tokio::test]
    async fn test_multiple_subscribers_all_receive() {
        let (tx, mut rx1) = tokio::sync::broadcast::channel::<JarvisEvent>(8);
        let mut rx2 = tx.subscribe();
        tx.send(JarvisEvent::Listening).unwrap();
        assert!(matches!(rx1.recv().await.unwrap(), JarvisEvent::Listening));
        assert!(matches!(rx2.recv().await.unwrap(), JarvisEvent::Listening));
    }

    #[test]
    fn test_publish_with_no_receivers_does_not_panic() {
        let (tx, _rx) = tokio::sync::broadcast::channel::<JarvisEvent>(8);
        drop(_rx);
        let _ = tx.send(JarvisEvent::Listening);
    }

    #[test]
    fn test_init_idempotent() {
        let tx1 = init();
        let tx2 = init();
        drop(tx1);
        drop(tx2);
    }
}
