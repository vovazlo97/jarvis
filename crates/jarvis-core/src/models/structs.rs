use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// tasks that components can request a backend for
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Task {
    Intent,
    Slots,
    Vad,
    NoiseSuppression,
    Stt,
}

// metadata about a model, parsed from model.toml on disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDef {
    pub id: String,
    pub name: String,
    pub tasks: Vec<Task>,

    #[serde(default)]
    pub description: String,

    // set at runtime after scanning the folder
    #[serde(skip)]
    pub path: PathBuf,

    /// true if the primary binary file exists and is not a Git LFS pointer.
    /// Set at runtime during catalog scan. Always false in a default/deserialized struct.
    #[serde(skip)]
    pub available: bool,
}

// a selectable option for a task (shown in UI / stored in settings)
#[derive(Debug, Clone, Serialize)]
pub struct BackendOption {
    pub id: String,
    pub name: String,
    // if Some, this option uses a model from the registry.
    // if None, it's a code-only backend (like energy VAD) or disabled.
    pub model_id: Option<String>,
    /// true if this option can actually be used:
    /// - code backends (energy, vosk, intent-classifier): always true
    /// - "none" / "Disabled": always true
    /// - model backends: true only if the binary is present and not an LFS pointer
    pub available: bool,
}
