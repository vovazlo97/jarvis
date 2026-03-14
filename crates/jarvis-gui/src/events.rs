use tauri::Emitter;

// the payload type must implement `Serialize` and `Clone`.
#[derive(Clone, serde::Serialize)]
struct Payload {
    data: String,
}

#[allow(dead_code)]
pub enum EventTypes {
    AudioPlay,
    AssistantWaiting,
    AssistantGreet,
    CommandStart,
    CommandInProcess,
    CommandEnd,
}

impl EventTypes {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AudioPlay => "audio-play",
            Self::AssistantWaiting => "assistant-waiting",
            Self::AssistantGreet => "assistant-greet",
            Self::CommandStart => "command-start",
            Self::CommandInProcess => "command-in-process",
            Self::CommandEnd => "command-end",
        }
    }
}

#[allow(dead_code)]
pub fn play(phrase: &str, app_handle: &tauri::AppHandle) {
    app_handle
        .emit(
            EventTypes::AudioPlay.as_str(),
            Payload {
                data: phrase.into(),
            },
        )
        .unwrap();
}
