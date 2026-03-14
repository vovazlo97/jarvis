use jarvis_core::{config, APP_DIR};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as Proc;

// ── Data model ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptStep {
    pub step_type: String, // "command_ref" | "delay" | "custom"
    #[serde(default)]
    pub label: String,
    // command_ref
    #[serde(default)]
    pub pack: String,
    #[serde(default)]
    pub command_id: String,
    // delay
    #[serde(default)]
    pub delay_ms: u64,
    // custom
    #[serde(default)]
    pub cli_cmd: String,
    #[serde(default)]
    pub cli_args: Vec<String>,
    // spotify
    #[serde(default)]
    pub spotify_action: String,
    #[serde(default)]
    pub spotify_track_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Script {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub mode: String, // "sequential" | "parallel"
    #[serde(default)]
    pub steps: Vec<ScriptStep>,
    #[serde(default)]
    pub phrases_ru: Vec<String>,
    #[serde(default)]
    pub phrases_en: Vec<String>,
    #[serde(default)]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub sounds_ru: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub response_sound: String,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

/// Return all scripts from disk (fresh, not cached).
#[tauri::command]
pub fn list_scripts() -> Vec<Script> {
    let dir = scripts_dir();
    let mut out = Vec::new();

    let entries = match fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return out,
    };

    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&p) {
            if let Ok(script) = toml::from_str::<Script>(&content) {
                out.push(script);
            }
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Create or update a script (identified by script.id).
/// Pass `old_id` when renaming so the old file gets deleted after the new one is written.
#[tauri::command]
pub fn save_script(script: Script, old_id: Option<String>) -> Result<(), String> {
    validate_script_id(&script.id)?;
    fs::create_dir_all(scripts_dir()).map_err(|e| format!("Cannot create scripts dir: {}", e))?;

    let content =
        toml::to_string_pretty(&script).map_err(|e| format!("Cannot serialize script: {}", e))?;

    fs::write(script_path(&script.id), content)
        .map_err(|e| format!("Cannot write script: {}", e))?;

    // Delete the old file if the ID was renamed
    if let Some(old) = old_id {
        if old != script.id {
            let old_path = script_path(&old);
            if old_path.exists() {
                fs::remove_file(&old_path)
                    .map_err(|e| format!("Cannot delete old script '{}': {}", old, e))?;
            }
        }
    }

    Ok(())
}

/// Remove a script file.
#[tauri::command]
pub fn delete_script(script_id: String) -> Result<(), String> {
    validate_script_id(&script_id)?;
    let p = script_path(&script_id);
    if !p.exists() {
        return Err(format!("Script '{}' not found", script_id));
    }
    fs::remove_file(&p).map_err(|e| format!("Cannot delete script: {}", e))
}

/// Execute a script — returns immediately (all processes are spawned detached).
#[tauri::command]
pub fn run_script(script_id: String) -> Result<(), String> {
    validate_script_id(&script_id)?;
    let content = fs::read_to_string(script_path(&script_id))
        .map_err(|e| format!("Cannot read script '{}': {}", script_id, e))?;
    let script: Script =
        toml::from_str(&content).map_err(|e| format!("Cannot parse script: {}", e))?;

    match script.mode.as_str() {
        "parallel" => run_parallel(&script.steps),
        _ => run_sequential(&script.steps),
    }
}

// ── Execution ─────────────────────────────────────────────────────────────────

fn run_sequential(steps: &[ScriptStep]) -> Result<(), String> {
    for step in steps {
        exec_step(step)?;
    }
    Ok(())
}

fn run_parallel(steps: &[ScriptStep]) -> Result<(), String> {
    use std::thread;
    let steps_owned: Vec<ScriptStep> = steps.to_vec();
    thread::spawn(move || {
        let mut handles = Vec::new();
        for step in steps_owned {
            let h = thread::spawn(move || {
                let _ = exec_step(&step);
            });
            handles.push(h);
        }
        for h in handles {
            let _ = h.join();
        }
    });
    Ok(())
}

fn exec_step(step: &ScriptStep) -> Result<(), String> {
    match step.step_type.as_str() {
        "command_ref" => exec_command_ref(step),
        "delay" => exec_delay(step),
        "custom" => exec_custom(step),
        "spotify" => exec_spotify(step),
        other => Err(format!("Unknown step type: {}", other)),
    }
}

fn exec_command_ref(step: &ScriptStep) -> Result<(), String> {
    let toml_path = APP_DIR
        .join(config::COMMANDS_PATH)
        .join(&step.pack)
        .join("command.toml");

    let content = fs::read_to_string(&toml_path)
        .map_err(|e| format!("Pack '{}' not found: {}", step.pack, e))?;

    #[derive(Deserialize)]
    struct CommandsList {
        commands: Vec<RawCmd>,
    }
    #[derive(Deserialize)]
    struct RawCmd {
        id: String,
        #[serde(rename = "type", default)]
        cmd_type: String,
        #[serde(default)]
        exe_path: String,
        #[serde(default)]
        exe_args: Vec<String>,
        #[serde(default)]
        cli_cmd: String,
        #[serde(default)]
        cli_args: Vec<String>,
    }

    let list: CommandsList = toml::from_str(&content)
        .map_err(|e| format!("Cannot parse pack '{}': {}", step.pack, e))?;

    let cmd = list
        .commands
        .into_iter()
        .find(|c| c.id == step.command_id)
        .ok_or_else(|| {
            format!(
                "Command '{}' not found in pack '{}'",
                step.command_id, step.pack
            )
        })?;

    match cmd.cmd_type.as_str() {
        "exe" | "url" => spawn_exe(&cmd.exe_path, &cmd.exe_args),
        "cli" => spawn_cli(&cmd.cli_cmd, &cmd.cli_args),
        other => Err(format!("Unsupported command type '{}' in script", other)),
    }
}

fn exec_delay(step: &ScriptStep) -> Result<(), String> {
    std::thread::sleep(std::time::Duration::from_millis(step.delay_ms));
    Ok(())
}

fn exec_custom(step: &ScriptStep) -> Result<(), String> {
    if step.cli_cmd.is_empty() {
        return Err("Custom step has no command".into());
    }
    spawn_cli(&step.cli_cmd, &step.cli_args)
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
        "next" => "(New-Object -ComObject WScript.Shell).SendKeys([char]176)".to_string(),
        other => return Err(format!("Unknown spotify action: '{}'", other)),
    };
    Proc::new("powershell")
        .args(["-NoProfile", "-Command", &ps_cmd])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Spotify step failed: {}", e))
}

fn spawn_exe(path: &str, args: &[String]) -> Result<(), String> {
    let dir = Path::new(path).parent().unwrap_or(Path::new("."));
    Proc::new(path)
        .args(args)
        .current_dir(dir)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to spawn '{}': {}", path, e))
}

fn spawn_cli(cmd: &str, args: &[String]) -> Result<(), String> {
    Proc::new("cmd")
        .args(["/C", cmd])
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Failed to run '{}': {}", cmd, e))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn scripts_dir() -> PathBuf {
    jarvis_core::config::user_scripts_dir()
}

fn script_path(id: &str) -> PathBuf {
    scripts_dir().join(format!("{}.toml", id))
}

fn validate_script_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.contains("..") || id.contains('/') || id.contains('\\') {
        Err("Invalid script ID".into())
    } else {
        Ok(())
    }
}
