use std::path::PathBuf;
use jarvis_core::{voices::{self, VoiceConfig}, config, SOUND_DIR};

#[tauri::command]
pub fn list_voices() -> Vec<VoiceConfig> {
    voices::list_voices().to_vec()
}

#[tauri::command]
pub fn get_voice(voice_id: String) -> Option<VoiceConfig> {
    voices::get_voice(&voice_id).cloned()
}

#[tauri::command]
pub fn preview_voice(voice_id: String) {
    voices::play_preview(&voice_id);
}

/// List all audio files inside SOUND_DIR/voices (recursively, up to 2 levels).
/// Returns relative paths from SOUND_DIR (e.g. "voices/jarvis-og/ru/ok1.wav").
#[tauri::command]
pub fn list_sound_files() -> Vec<String> {
    let voices_dir = SOUND_DIR.join(config::VOICES_PATH);
    let mut result = Vec::new();
    collect_audio_files_rel(&voices_dir, &SOUND_DIR, &mut result);
    result.sort();
    result
}

fn collect_audio_files_rel(dir: &PathBuf, base: &PathBuf, out: &mut Vec<String>) {
    const EXTS: &[&str] = &["wav", "mp3", "ogg"];
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect_audio_files_rel(&p, base, out);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if EXTS.contains(&ext) {
                if let Ok(rel) = p.strip_prefix(base) {
                    if let Some(s) = rel.to_str() {
                        out.push(s.replace('\\', "/"));
                    }
                }
            }
        }
    }
}

/// Copy a sound file into SOUND_DIR/voices/user_custom/<category>/.
/// Returns the relative path from SOUND_DIR on success.
#[tauri::command]
pub fn import_sound_file(src_path: String, category: String) -> Result<String, String> {
    let safe_cat: String = category.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    let safe_cat = if safe_cat.is_empty() { "general".to_string() } else { safe_cat };

    let dest_dir = SOUND_DIR.join(config::VOICES_PATH)
        .join("user_custom").join(&safe_cat);
    std::fs::create_dir_all(&dest_dir)
        .map_err(|e| format!("Cannot create dir: {}", e))?;

    let src = PathBuf::from(&src_path);
    let filename = src.file_name().ok_or("Invalid source path")?;
    let dest = dest_dir.join(filename);
    std::fs::copy(&src_path, &dest)
        .map_err(|e| format!("Cannot copy: {}", e))?;

    dest.strip_prefix(&*SOUND_DIR)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .map_err(|_| "Non-UTF8 path".to_string())
}