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

/// Universal fallback embedding model — always available as a real binary (not LFS pointer).
/// Used when the language-preferred model is unavailable.
const FALLBACK_EMBEDDING_MODEL: &str = "all-MiniLM-L6-v2";

/// Select the preferred embedding model ID for the given language.
/// For English: lightweight English-only model.
/// For other languages: multilingual model (may be unavailable if not downloaded).
fn select_embedding_model_id(lang: &str) -> &'static str {
    match lang {
        "en" => "all-MiniLM-L6-v2",
        _ => "paraphrase-multilingual-MiniLM-L12-v2",
    }
}

pub async fn init(commands: &[JCommandsList]) -> Result<(), String> {
    if BACKEND.get().is_some() {
        return Ok(());
    }

    let backend = DB.get().unwrap().read().intent_backend.clone();

    // Helper: set BACKEND to "none" and log fallback reason.
    // Used when a model binary is unavailable so the app continues with regex/fuzzy matching.
    let set_fallback = |reason: &str| {
        warn!(
            "Intent classifier falling back to disabled mode: {}. \
             Regex/fuzzy matching will handle command routing.",
            reason
        );
        BACKEND.set("none".to_string()).ok(); // OnceCell — ignore if already set
    };

    match backend.as_str() {
        "none" => {
            info!("Intent recognition disabled");
            BACKEND.set(backend).map_err(|_| "Backend already set")?;
        }
        "intent-classifier" | "IntentClassifier" => {
            info!("Initializing IntentClassifier backend.");
            intentclassifier::init(commands).await?;
            BACKEND.set(backend).map_err(|_| "Backend already set")?;
            info!("IntentClassifier backend initialized.");
        }
        // Legacy enum value — auto-select model by language (restores pre-registry behavior)
        "EmbeddingClassifier" => {
            let lang = crate::i18n::get_language();
            let preferred = select_embedding_model_id(&lang);
            info!(
                "EmbeddingClassifier (auto) → preferred model '{}' (language: {}).",
                preferred, lang
            );
            // Try preferred model, then fall back to the universal English model
            // if the preferred one is unavailable (e.g. multilingual LFS pointer not downloaded).
            let model_result =
                models::embedding::load(models::registry(), preferred).or_else(|e| {
                    if preferred != FALLBACK_EMBEDDING_MODEL {
                        warn!(
                            "Preferred model '{}' unavailable ({}). \
                             Trying universal fallback '{}'...",
                            preferred, e, FALLBACK_EMBEDDING_MODEL
                        );
                        models::embedding::load(models::registry(), FALLBACK_EMBEDDING_MODEL)
                    } else {
                        Err(e)
                    }
                });
            match model_result {
                Ok(model) => {
                    embeddingclassifier::init_with_model(model, commands)?;
                    BACKEND.set(backend).map_err(|_| "Backend already set")?;
                    info!("EmbeddingClassifier backend initialized.");
                }
                Err(e) => {
                    set_fallback(&e);
                }
            }
        }
        // any other value is treated as an explicit model ID for embedding classification
        model_id => {
            info!(
                "Initializing EmbeddingClassifier with model '{}'.",
                model_id
            );
            let model_id_owned = model_id.to_string();
            match models::embedding::load(models::registry(), &model_id_owned) {
                Ok(model) => {
                    embeddingclassifier::init_with_model(model, commands)?;
                    BACKEND.set(backend).map_err(|_| "Backend already set")?;
                    info!("EmbeddingClassifier backend initialized.");
                }
                Err(e) => {
                    set_fallback(&e);
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// select_embedding_model_id must return the lightweight English model for "en".
    #[test]
    fn selects_minilm_for_english() {
        assert_eq!(select_embedding_model_id("en"), "all-MiniLM-L6-v2");
    }

    /// select_embedding_model_id must return the multilingual model for non-English languages.
    #[test]
    fn selects_multilingual_for_russian() {
        assert_eq!(
            select_embedding_model_id("ru"),
            "paraphrase-multilingual-MiniLM-L12-v2"
        );
    }

    /// select_embedding_model_id must return the multilingual model for unknown languages.
    #[test]
    fn selects_multilingual_for_unknown_lang() {
        assert_eq!(
            select_embedding_model_id("xx"),
            "paraphrase-multilingual-MiniLM-L12-v2"
        );
    }

    /// FALLBACK_EMBEDDING_MODEL must be the lightweight English model that is always
    /// available as a real binary (not an LFS pointer).
    #[test]
    fn fallback_model_is_minilm() {
        assert_eq!(FALLBACK_EMBEDDING_MODEL, "all-MiniLM-L6-v2");
    }

    /// Fallback must differ from the multilingual preferred model so the
    /// or_else branch is actually reachable for non-English languages.
    #[test]
    fn fallback_differs_from_multilingual() {
        assert_ne!(
            FALLBACK_EMBEDDING_MODEL,
            select_embedding_model_id("ru"),
            "fallback must be a different model than the Russian preferred model"
        );
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
