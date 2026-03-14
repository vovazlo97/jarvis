mod embeddingclassifier;
mod intentclassifier;

use std::path::PathBuf;

use crate::{
    commands::{self, JCommand, JCommandsList},
    config, models,
};
use once_cell::sync::OnceCell;

use crate::DB;

static BACKEND: OnceCell<String> = OnceCell::new();

pub async fn init(commands: &Vec<JCommandsList>) -> Result<(), String> {
    if BACKEND.get().is_some() {
        return Ok(());
    }

    let backend = DB.get().unwrap().read().intent_backend.clone();

    BACKEND
        .set(backend.clone())
        .map_err(|_| "Backend already set")?;

    match backend.as_str() {
        "none" => {
            info!("Intent recognition disabled");
        }
        "intent-classifier" | "IntentClassifier" => {
            info!("Initializing IntentClassifier backend.");
            intentclassifier::init(&commands).await?;
            info!("IntentClassifier backend initialized.");
        }
        // Legacy enum value — auto-select model by language (restores pre-registry behavior)
        "EmbeddingClassifier" => {
            let model_id = match crate::i18n::get_language().as_str() {
                "en" => "all-MiniLM-L6-v2",
                _ => "paraphrase-multilingual-MiniLM-L12-v2",
            };
            info!(
                "EmbeddingClassifier (auto) → model '{}' (language: {}).",
                model_id,
                crate::i18n::get_language()
            );
            let model = models::embedding::load(models::registry(), model_id)?;
            embeddingclassifier::init_with_model(model, &commands)?;
            info!("EmbeddingClassifier backend initialized.");
        }
        // any other value is treated as an explicit model ID for embedding classification
        model_id => {
            info!(
                "Initializing EmbeddingClassifier with model '{}'.",
                model_id
            );
            let model = models::embedding::load(models::registry(), model_id)?;
            embeddingclassifier::init_with_model(model, &commands)?;
            info!("EmbeddingClassifier backend initialized.");
        }
    }

    Ok(())
}

/// Retrain the intent classifier with a new commands list.
/// Safe to call at runtime — replaces the model without restarting audio pipeline.
/// Supports both intent-classifier and embedding backends.
pub async fn reinit(commands: &[JCommandsList]) -> Result<(), String> {
    match BACKEND.get().map(|s| s.as_str()) {
        Some("intent-classifier") => {
            info!("Retraining IntentClassifier with updated commands...");
            intentclassifier::reinit(commands).await?;
            info!("IntentClassifier retrained successfully.");
        }
        Some("none") | None => {
            // intent recognition disabled or not yet initialised — skip
        }
        _ => {
            // embedding backend: rebuild intent vectors, keep loaded model
            embeddingclassifier::reinit(commands)?;
        }
    }
    Ok(())
}

pub async fn classify(text: &str) -> Option<(String, f64)> {
    match BACKEND.get()?.as_str() {
        "none" => None,
        "intent-classifier" => match intentclassifier::classify(text).await {
            Ok(prediction) => {
                let confidence = prediction.confidence.value();
                if confidence >= config::INTENT_CLASSIFIER_MIN_CONFIDENCE {
                    Some((prediction.intent.to_string(), confidence))
                } else {
                    None
                }
            }
            Err(e) => {
                error!("Intent classification error: {}", e);
                None
            }
        },
        _ => match embeddingclassifier::classify(text) {
            Ok((intent_id, confidence)) => {
                if confidence >= config::EMBEDDING_MIN_CONFIDENCE {
                    Some((intent_id, confidence))
                } else {
                    None
                }
            }
            Err(e) => {
                error!("Embedding classification error: {}", e);
                None
            }
        },
    }
}

// unified command lookup by intent ID - works for all backends
pub fn get_command_by_intent<'a>(
    commands: &'a [JCommandsList],
    intent_id: &str,
) -> Option<(&'a PathBuf, &'a JCommand)> {
    if matches!(BACKEND.get().map(|s| s.as_str()), Some("none")) {
        return None;
    }
    commands::get_command_by_id(commands, intent_id)
}
