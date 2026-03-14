use serde::{Deserialize, Serialize};

/// High-level assistant state for GUI and IPC consumers.
/// Separate from VadState (low-level audio VAD) in jarvis-app.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantState {
    /// Waiting for wake word. Microphone active, no intent detected.
    Idle,
    /// Wake word heard. Playing reply sound, about to record command.
    Activated,
    /// Actively recording and transcribing the spoken command.
    Listening,
    /// STT complete. Running intent recognition + command dispatch.
    Processing,
    /// Command dispatched. Audio feedback playing, then returning to Idle.
    Responding,
}
