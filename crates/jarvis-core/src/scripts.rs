use std::fs;
use std::path::PathBuf;
use std::process::Command as Proc;
use serde::{Deserialize, Serialize};
use seqdiff::ratio;

use crate::{config, APP_DIR};

const SCRIPTS_DIR: &str = "resources/scripts";

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptStep {
    pub step_type: String,   // "command_ref" | "delay" | "custom"
    #[serde(default)] pub label: String,
    // command_ref
    #[serde(default)] pub pack: String,
    #[serde(default)] pub command_id: String,
    // delay
    #[serde(default)] pub delay_ms: u64,
    // custom
    #[serde(default)] pub cli_cmd: String,
    #[serde(default)] pub cli_args: Vec<String>,
    // spotify
    #[serde(default)] pub spotify_action: String,
    #[serde(default)] pub spotify_track_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Script {
    pub id: String,
    pub name: String,
    #[serde(default)] pub description: String,
    pub mode: String,              // "sequential" | "parallel"
    #[serde(default)] pub steps: Vec<ScriptStep>,
    #[serde(default)] pub phrases_ru: Vec<String>,
    #[serde(default)] pub phrases_en: Vec<String>,
    #[serde(default)] pub patterns: Vec<String>,
    #[serde(default)] pub sounds_ru: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response_sound: String,
}

impl Script {
    /// Return combined phrase list for the given language (falls back to RU).
    pub fn get_phrases(&self, lang: &str) -> Vec<&str> {
        let specific: Vec<&str> = match lang {
            "ru" => self.phrases_ru.iter().map(|s| s.as_str()).collect(),
            "en" => self.phrases_en.iter().map(|s| s.as_str()).collect(),
            _    => vec![],
        };
        if specific.is_empty() {
            self.phrases_ru.iter().map(|s| s.as_str()).collect()
        } else {
            specific
        }
    }

    pub fn get_sounds(&self, _lang: &str) -> Vec<String> {
        if self.sounds_ru.is_empty() {
            vec!["ok1".to_string(), "ok2".to_string(), "ok3".to_string()]
        } else {
            self.sounds_ru.clone()
        }
    }

    /// Count of voice triggers across all languages.
    pub fn trigger_count(&self) -> usize {
        self.phrases_ru.len() + self.phrases_en.len() + self.patterns.len()
    }
}

// ── Loading ───────────────────────────────────────────────────────────────────

pub fn parse_scripts() -> Vec<Script> {
    let dir = scripts_dir();
    info!("[DEBUG_FIX] parse_scripts() scanning: {:?}", dir);
    let mut out = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(e)  => e,
        Err(e) => {
            info!("[DEBUG_FIX] parse_scripts() dir not accessible: {}", e);
            return out;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") { continue; }

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<Script>(&content) {
                Ok(script) => out.push(script),
                Err(e) => warn!("Failed to parse script {}: {}", path.display(), e),
            },
            Err(e) => warn!("Failed to read script {}: {}", path.display(), e),
        }
    }

    out
}

fn scripts_dir() -> PathBuf {
    APP_DIR.join(SCRIPTS_DIR)
}

// ── Matching ──────────────────────────────────────────────────────────────────

/// Returns the first script matching `phrase` — regex first, then fuzzy.
/// Mirrors the logic of `commands::fetch_command`.
pub fn fetch_script<'a>(phrase: &str, scripts: &'a [Script]) -> Option<&'a Script> {
    let phrase = phrase.trim().to_lowercase();
    if phrase.is_empty() { return None; }

    // ── 1. Regex pass ──────────────────────────────────────────────────────────
    #[cfg(feature = "regex")]
    {
        use regex::Regex;
        for script in scripts {
            for pattern in &script.patterns {
                match Regex::new(pattern) {
                    Ok(re) => {
                        if re.is_match(&phrase) {
                            info!("Script regex match: '{}' -> '{}' (pattern: '{}')",
                                  phrase, script.id, pattern);
                            return Some(script);
                        }
                    }
                    Err(e) => warn!("Invalid script regex '{}' in '{}': {}", pattern, script.id, e),
                }
            }
        }
    }

    // ── 2. Exact phrase pass ───────────────────────────────────────────────────
    let lang = crate::i18n::get_language();
    for script in scripts {
        for trigger in script.get_phrases(&lang) {
            if trigger.trim().to_lowercase() == phrase {
                info!("Script exact match: '{}' -> '{}'", phrase, script.id);
                return Some(script);
            }
        }
    }

    // ── 3. Fuzzy pass (high threshold = 95%) ──────────────────────────────────
    let phrase_chars: Vec<char> = phrase.chars().collect();
    let phrase_words: Vec<&str> = phrase.split_whitespace().collect();

    let mut best_script: Option<&Script> = None;
    let mut best_score = config::SCRIPT_RATIO_THRESHOLD;

    for script in scripts {
        for trigger in script.get_phrases(&lang) {
            let t_lower = trigger.trim().to_lowercase();
            let t_chars: Vec<char> = t_lower.chars().collect();

            let char_ratio = ratio(&phrase_chars, &t_chars);
            let t_words: Vec<&str> = t_lower.split_whitespace().collect();
            let word_score = word_overlap_score(&phrase_words, &t_words);
            let score = (char_ratio * 0.6) + (word_score * 0.4);

            if score >= 99.0 {
                debug!("Script perfect match: '{}' -> '{}'", phrase, trigger);
                return Some(script);
            }
            if score > best_score {
                best_score = score;
                best_script = Some(script);
            }
        }
    }

    if let Some(s) = best_script {
        info!("Script fuzzy match: '{}' -> '{}' (score: {:.1}%)", phrase, s.id, best_score);
    } else {
        debug!("No script match for '{}' (best: {:.1}%)", phrase, best_score);
    }

    best_script
}

fn word_overlap_score(input_words: &[&str], cmd_words: &[&str]) -> f64 {
    if input_words.is_empty() || cmd_words.is_empty() { return 0.0; }
    let input_chars: Vec<Vec<char>> = input_words.iter().map(|w| w.chars().collect()).collect();
    let cmd_chars:   Vec<Vec<char>> = cmd_words.iter().map(|w| w.chars().collect()).collect();

    // How well is the TRIGGER covered by the input? (prevent 2-word trigger matching any long phrase)
    let trigger_coverage: f64 = cmd_chars.iter().map(|cw| {
        input_chars.iter().map(|iw| ratio(iw, cw)).fold(0.0_f64, f64::max)
    }).sum::<f64>() / cmd_words.len() as f64;

    // How well is the INPUT covered by the trigger? (penalise very long inputs)
    let input_coverage: f64 = input_chars.iter().map(|iw| {
        cmd_chars.iter().map(|cw| ratio(iw, cw)).fold(0.0_f64, f64::max)
    }).sum::<f64>() / input_words.len() as f64;

    // Use the harmonic mean — both sides must match well
    if trigger_coverage + input_coverage < 1e-9 { return 0.0; }
    // ratio() already returns 0-100, so harmonic mean is already in 0-100 range — no extra scaling needed
    2.0 * trigger_coverage * input_coverage / (trigger_coverage + input_coverage)
}

// ── Execution ─────────────────────────────────────────────────────────────────

/// Execute all steps of a script (sequential or parallel).
/// Returns immediately — spawns background thread for parallel mode.
pub fn execute_script(script: &Script) -> Result<(), String> {
    match script.mode.as_str() {
        "parallel" => run_parallel(script.steps.clone()),
        _          => run_sequential(&script.steps),
    }
}

fn run_sequential(steps: &[ScriptStep]) -> Result<(), String> {
    for step in steps {
        exec_step(step)?;
    }
    Ok(())
}

fn run_parallel(steps: Vec<ScriptStep>) -> Result<(), String> {
    std::thread::spawn(move || {
        let handles: Vec<_> = steps.into_iter().map(|step| {
            std::thread::spawn(move || { let _ = exec_step(&step); })
        }).collect();
        for h in handles { let _ = h.join(); }
    });
    Ok(())
}

fn exec_step(step: &ScriptStep) -> Result<(), String> {
    match step.step_type.as_str() {
        "command_ref" => exec_command_ref(step),
        "delay"       => exec_delay(step),
        "custom"      => exec_custom(step),
        "spotify"     => exec_spotify(step),
        other         => Err(format!("Unknown script step type: '{}'", other)),
    }
}

fn exec_command_ref(step: &ScriptStep) -> Result<(), String> {
    // Look up the command in the already-loaded COMMANDS_LIST
    let cmds_guard = crate::COMMANDS_LIST.read();
    let cmds_list = &*cmds_guard;

    for cmd_list in cmds_list {
        let pack_name = cmd_list.path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if pack_name != step.pack { continue; }

        if let Some(cmd) = cmd_list.commands.iter().find(|c| c.id == step.command_id) {
            return crate::commands::execute_command(&cmd_list.path, cmd, None, None)
                .map(|_| ())
                .map_err(|e| e);
        }
    }

    Err(format!("Script command ref '{}/{}' not found", step.pack, step.command_id))
}

fn exec_delay(step: &ScriptStep) -> Result<(), String> {
    std::thread::sleep(std::time::Duration::from_millis(step.delay_ms));
    Ok(())
}

fn exec_custom(step: &ScriptStep) -> Result<(), String> {
    if step.cli_cmd.is_empty() {
        return Err("Custom script step has no command".into());
    }
    Proc::new("cmd")
        .args(["/C", &step.cli_cmd])
        .args(&step.cli_args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Script custom step failed: {}", e))
}

fn exec_spotify(step: &ScriptStep) -> Result<(), String> {
    let ps_cmd = match step.spotify_action.as_str() {
        "play_track" => {
            if step.spotify_track_id.is_empty() {
                return Err("Spotify step: track ID is empty".into());
            }
            format!("Start-Process 'spotify:track:{}'", step.spotify_track_id)
        }
        "pause" => "(New-Object -ComObject WScript.Shell).SendKeys([char]179)".to_string(),
        "next"  => "(New-Object -ComObject WScript.Shell).SendKeys([char]176)".to_string(),
        other   => return Err(format!("Unknown spotify action: '{}'", other)),
    };
    Proc::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Spotify step failed: {}", e))
}

/// Live version: always reads from disk (no cache).
/// Use this for fuzzy-fallback matching so deleted scripts stop working immediately
/// and newly added scripts are recognized without restarting the app.
pub fn fetch_script_live(phrase: &str) -> Option<Script> {
    let scripts = parse_scripts();
    fetch_script(phrase, &scripts).cloned()
}

/// Load a single script by ID directly from disk.
/// Returns None if the file doesn't exist (e.g. deleted via GUI).
pub fn load_script(id: &str) -> Option<Script> {
    let path = scripts_dir().join(format!("{}.toml", id));
    let content = fs::read_to_string(&path).ok()?;
    toml::from_str::<Script>(&content).ok()
}

// ── Intent integration ────────────────────────────────────────────────────────

/// Convert scripts to virtual JCommandsList entries so the intent classifier
/// can learn their phrases. Each script becomes a virtual command with
/// cmd_type = "script_ref". Scripts without any phrases are skipped.
#[cfg(test)]
mod tests {
    use super::*;

    fn make_script(id: &str, phrases_ru: Vec<&str>) -> Script {
        Script {
            id: id.to_string(),
            name: id.to_string(),
            description: String::new(),
            mode: "sequential".to_string(),
            steps: vec![],
            phrases_ru: phrases_ru.iter().map(|s| s.to_string()).collect(),
            phrases_en: vec![],
            patterns: vec![],
            sounds_ru: vec![],
            response_sound: String::new(),
        }
    }

    #[test]
    fn test_fetch_script_empty_list_returns_none() {
        assert!(fetch_script("открой ночной режим", &[]).is_none());
    }

    #[test]
    fn test_fetch_script_exact_match() {
        let scripts = vec![make_script("night_mode", vec!["включи ночной режим"])];
        let result = fetch_script("включи ночной режим", &scripts);
        assert!(result.is_some());
        assert_eq!(result.unwrap().id, "night_mode");
    }

    #[test]
    fn test_fetch_script_no_match_for_unrelated_text() {
        let scripts = vec![make_script("night_mode", vec!["включи ночной режим"])];
        // Latin vs Cyrillic — char_ratio near 0, well below 88% threshold
        assert!(fetch_script("open browser settings now", &scripts).is_none());
    }

    #[test]
    fn test_load_script_returns_none_for_missing_id() {
        let result = load_script("definitely_nonexistent_id_xyz_abc_12345");
        assert!(result.is_none());
    }
}

pub fn as_virtual_commands(scripts: &[Script]) -> Vec<crate::JCommandsList> {
    let dir = scripts_dir();
    scripts.iter().filter_map(|s| {
        if s.phrases_ru.is_empty() && s.phrases_en.is_empty() {
            return None;
        }
        let mut phrases = std::collections::HashMap::new();
        if !s.phrases_ru.is_empty() {
            phrases.insert("ru".to_string(), s.phrases_ru.clone());
        }
        if !s.phrases_en.is_empty() {
            phrases.insert("en".to_string(), s.phrases_en.clone());
        }
        let mut sounds = std::collections::HashMap::new();
        if !s.sounds_ru.is_empty() {
            sounds.insert("ru".to_string(), s.sounds_ru.clone());
        }
        let cmd = crate::commands::JCommand::new_script_ref(s.id.clone(), phrases, sounds);
        Some(crate::JCommandsList {
            path: dir.clone(),
            commands: vec![cmd],
        })
    }).collect()
}
