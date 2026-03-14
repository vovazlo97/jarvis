mod gliner;

use once_cell::sync::OnceCell;
use std::collections::HashMap;

use crate::commands::{SlotDefinition, SlotValue};
use crate::{models, DB};

static BACKEND: OnceCell<String> = OnceCell::new();

/// Normalize backend value from DB settings.
/// The GUI historically stores "None" (Python-style) instead of "none".
/// Treat empty string and any case-variant of "none" as the canonical "none".
fn normalize_backend(raw: &str) -> String {
    if raw.is_empty() || raw.eq_ignore_ascii_case("none") {
        "none".to_string()
    } else {
        raw.to_string()
    }
}

pub fn init() -> Result<(), String> {
    if BACKEND.get().is_some() {
        return Ok(());
    }

    let raw = DB
        .get()
        .map(|db| db.read().slots_backend.clone())
        .unwrap_or_else(|| "none".to_string());

    let backend = normalize_backend(&raw);

    BACKEND
        .set(backend.clone())
        .map_err(|_| "Slot backend already set")?;

    match backend.as_str() {
        "none" => {
            info!("Slot extraction disabled");
        }
        // any model ID is treated as a GLiNER model for now
        model_id => {
            info!(
                "Initializing GLiNER slot extraction with model '{}'.",
                model_id
            );
            let model = models::gliner::load(models::registry(), model_id)?;
            gliner::init_with_model(model)?;
            info!("GLiNER slot extraction initialized.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// normalize_backend must treat "None", "NONE", "", and "none" all as "none".
    /// It must pass through real model IDs unchanged.
    #[test]
    fn normalize_backend_handles_all_none_variants() {
        assert_eq!(normalize_backend("None"), "none");
        assert_eq!(normalize_backend("NONE"), "none");
        assert_eq!(normalize_backend("none"), "none");
        assert_eq!(normalize_backend(""), "none");
        assert_eq!(normalize_backend("some-model-id"), "some-model-id");
        assert_eq!(normalize_backend("gliner-small"), "gliner-small");
    }
}

pub fn extract(text: &str, slots: &HashMap<String, SlotDefinition>) -> HashMap<String, SlotValue> {
    if slots.is_empty() {
        return HashMap::new();
    }

    match BACKEND.get().map(|s| s.as_str()).unwrap_or("none") {
        "none" => HashMap::new(),
        _ => match gliner::extract(text, slots) {
            Ok(result) => result,
            Err(e) => {
                error!("GLiNER slot extraction failed: {}", e);
                HashMap::new()
            }
        },
    }
}
