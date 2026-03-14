use jarvis_core::{
    gliner_models,
    models::{self, Task},
    vosk_models,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct VoskModel {
    pub name: String,
    pub language: String,
    pub size: String,
}

#[derive(Serialize)]
pub struct GlinerVariant {
    pub display_name: String,
    pub value: String,
}

#[tauri::command]
pub fn list_vosk_models() -> Vec<VoskModel> {
    vosk_models::scan_vosk_models()
        .into_iter()
        .map(|m| VoskModel {
            name: m.name,
            language: m.language,
            size: m.size,
        })
        .collect()
}

/// Return backend options that are actually usable for the given task.
/// `task` must be one of: "intent", "slots", "stt", "vad", "noise_suppression".
/// Each option includes `available: bool` so the frontend can show disabled options
/// (e.g. "Download required") vs selectable ones.
///
/// Requires the Model Registry to be initialized (models::init() must have run).
#[tauri::command]
pub fn list_available_models(task: String) -> Vec<jarvis_core::models::BackendOption> {
    let t = match task.as_str() {
        "intent" => Task::Intent,
        "slots" => Task::Slots,
        "stt" => Task::Stt,
        "vad" => Task::Vad,
        "noise_suppression" => Task::NoiseSuppression,
        _ => {
            warn!("list_available_models: unknown task '{}'", task);
            return vec![];
        }
    };
    models::list_available(t)
}

#[tauri::command]
pub fn list_gliner_models() -> Vec<GlinerVariant> {
    gliner_models::scan_gliner_variants()
        .into_iter()
        .map(|m| GlinerVariant {
            display_name: m.display_name,
            value: m.value,
        })
        .collect()
}
