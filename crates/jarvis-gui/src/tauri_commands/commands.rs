use jarvis_core::commands::{self, JCommand, JCommandsList};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

static COMMANDS: Lazy<Vec<JCommandsList>> =
    Lazy::new(|| commands::parse_commands().unwrap_or_default());

#[tauri::command]
pub fn get_commands_count() -> usize {
    COMMANDS.iter().map(|list| list.commands.len()).sum()
}

#[tauri::command]
pub fn get_commands_list() -> Vec<JCommand> {
    COMMANDS
        .iter()
        .flat_map(|list| list.commands.clone())
        .collect()
}

// ── Editor API ────────────────────────────────────────────────────────────────

#[derive(Serialize, Debug)]
pub struct CommandPackInfo {
    pub pack_name: String,
    pub commands: Vec<JCommand>,
}

/// Flat representation of a command — used for both creating and updating.
#[derive(Deserialize, Debug)]
pub struct NewCommandInput {
    pub id: String,
    #[serde(rename = "type")]
    pub cmd_type: String,
    pub description: String,
    pub phrases_ru: Vec<String>,
    pub phrases_en: Vec<String>,
    pub exe_path: String,
    pub exe_args: Vec<String>,
    pub cli_cmd: String,
    pub cli_args: Vec<String>,
    pub patterns: Vec<String>,
    pub sounds_ru: Vec<String>,
    #[serde(default)]
    pub response_sound: String,
}

/// Return all packs from disk (always fresh, never cached).
#[tauri::command]
pub fn list_command_packs() -> Vec<CommandPackInfo> {
    let commands_path = jarvis_core::config::user_commands_dir();
    let mut packs = Vec::new();

    let entries = match fs::read_dir(&commands_path) {
        Ok(e) => e,
        Err(_) => return packs,
    };

    for entry in entries.flatten() {
        let pack_path = entry.path();
        let toml_file = pack_path.join("command.toml");
        if !toml_file.exists() {
            continue;
        }
        let content = match fs::read_to_string(&toml_file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let file: JCommandsList = match toml::from_str(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };
        packs.push(CommandPackInfo {
            pack_name: entry.file_name().to_string_lossy().into_owned(),
            commands: file.commands,
        });
    }

    packs
}

/// Create a brand new pack folder + command.toml with the first command.
#[tauri::command]
pub fn create_command_pack(pack_name: String, command: NewCommandInput) -> Result<(), String> {
    let safe = sanitize_pack_name(&pack_name)?;
    let pack_path = jarvis_core::config::user_commands_dir().join(&safe);
    fs::create_dir_all(&pack_path)
        .map_err(|e| format!("Cannot create directory '{}': {}", safe, e))?;

    let toml_path = pack_path.join("command.toml");
    let cmd = input_to_jcommand(&command)?;
    save_commands(&[cmd], &toml_path)
}

/// Append a command to an existing pack without touching other commands.
#[tauri::command]
pub fn append_command_to_pack(pack_name: String, command: NewCommandInput) -> Result<(), String> {
    validate_name(&pack_name)?;
    let toml_path = pack_toml_path(&pack_name);

    let content = fs::read_to_string(&toml_path).map_err(|e| format!("Cannot read pack: {}", e))?;
    let mut list: JCommandsList =
        toml::from_str(&content).map_err(|e| format!("Cannot parse pack: {}", e))?;

    if list.commands.iter().any(|c| c.id == command.id) {
        return Err(format!(
            "Command '{}' already exists in '{}'",
            command.id, pack_name
        ));
    }

    list.commands.push(input_to_jcommand(&command)?);
    save_commands(&list.commands, &toml_path)
}

/// Replace a single command (identified by old_id) inside a pack.
#[tauri::command]
pub fn update_command(
    pack_name: String,
    old_id: String,
    command: NewCommandInput,
) -> Result<(), String> {
    validate_name(&pack_name)?;
    let toml_path = pack_toml_path(&pack_name);

    let content = fs::read_to_string(&toml_path).map_err(|e| format!("Cannot read pack: {}", e))?;
    let mut list: JCommandsList =
        toml::from_str(&content).map_err(|e| format!("Cannot parse pack: {}", e))?;

    let pos = list
        .commands
        .iter()
        .position(|c| c.id == old_id)
        .ok_or_else(|| format!("Command '{}' not found in '{}'", old_id, pack_name))?;

    list.commands[pos] = input_to_jcommand(&command)?;
    save_commands(&list.commands, &toml_path)
}

/// Delete a single command from a pack. Removes the pack folder if it becomes empty.
#[tauri::command]
pub fn delete_command(pack_name: String, command_id: String) -> Result<(), String> {
    validate_name(&pack_name)?;
    let toml_path = pack_toml_path(&pack_name);

    let content = fs::read_to_string(&toml_path).map_err(|e| format!("Cannot read pack: {}", e))?;
    let mut list: JCommandsList =
        toml::from_str(&content).map_err(|e| format!("Cannot parse pack: {}", e))?;

    let before = list.commands.len();
    list.commands.retain(|c| c.id != command_id);
    if list.commands.len() == before {
        return Err(format!(
            "Command '{}' not found in '{}'",
            command_id, pack_name
        ));
    }

    if list.commands.is_empty() {
        let pack_path = jarvis_core::config::user_commands_dir().join(&pack_name);
        let _ = fs::remove_dir_all(&pack_path);
        return Ok(());
    }

    save_commands(&list.commands, &toml_path)
}

/// Delete an entire pack folder.
#[tauri::command]
pub fn delete_command_pack(pack_name: String) -> Result<(), String> {
    validate_name(&pack_name)?;
    let pack_path = jarvis_core::config::user_commands_dir().join(&pack_name);
    if !pack_path.exists() {
        return Err(format!("Pack '{}' not found", pack_name));
    }
    fs::remove_dir_all(&pack_path).map_err(|e| format!("Cannot delete pack '{}': {}", pack_name, e))
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn sanitize_pack_name(name: &str) -> Result<String, String> {
    let safe: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if safe.is_empty() {
        Err("Invalid pack name: use letters, digits, _ or -".into())
    } else {
        Ok(safe)
    }
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
        Err("Invalid pack name".into())
    } else {
        Ok(())
    }
}

fn pack_toml_path(pack_name: &str) -> PathBuf {
    jarvis_core::config::user_commands_dir()
        .join(pack_name)
        .join("command.toml")
}

/// Build a JCommand from user input by round-tripping through TOML.
/// This correctly sets all default fields without needing to touch private cache fields.
fn input_to_jcommand(input: &NewCommandInput) -> Result<JCommand, String> {
    let toml_str = build_toml(input);
    let list: JCommandsList =
        toml::from_str(&toml_str).map_err(|e| format!("Cannot build command from input: {}", e))?;
    list.commands
        .into_iter()
        .next()
        .ok_or_else(|| "Empty command list after build".to_string())
}

/// Write all commands back to a TOML file using serde serialization.
/// This guarantees the file always round-trips correctly with toml::from_str.
fn save_commands(commands: &[JCommand], path: &Path) -> Result<(), String> {
    let list = JCommandsList {
        path: std::path::PathBuf::new(),
        commands: commands.to_vec(),
    };
    let content =
        toml::to_string_pretty(&list).map_err(|e| format!("Cannot serialize commands: {}", e))?;
    fs::write(path, content).map_err(|e| format!("Cannot write TOML: {}", e))
}

/// Build a fresh [[commands]] TOML block from user input.
fn build_toml(cmd: &NewCommandInput) -> String {
    fn esc(s: &str) -> String {
        format!("{:?}", s)
    }
    fn esc_arr(v: &[String]) -> String {
        format!(
            "[{}]",
            v.iter()
                .map(|s| format!("{:?}", s))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    let mut lines = vec![
        "[[commands]]".to_string(),
        format!("id = {}", esc(&cmd.id)),
        format!("type = {}", esc(&cmd.cmd_type)),
    ];

    if !cmd.description.is_empty() {
        lines.push(format!("description = {}", esc(&cmd.description)));
    }

    match cmd.cmd_type.as_str() {
        "exe" => {
            lines.push(format!("exe_path = {}", esc(&cmd.exe_path)));
            lines.push(format!("exe_args = {}", esc_arr(&cmd.exe_args)));
        }
        "url" => {
            lines.push(format!("cli_cmd = {}", esc(&cmd.exe_path)));
        }
        "cli" => {
            lines.push(format!("cli_cmd = {}", esc(&cmd.cli_cmd)));
            if !cmd.cli_args.is_empty() {
                lines.push(format!("cli_args = {}", esc_arr(&cmd.cli_args)));
            }
        }
        _ => {}
    }

    let sounds = if cmd.sounds_ru.is_empty() {
        vec!["ok1".to_string(), "ok2".to_string(), "ok3".to_string()]
    } else {
        cmd.sounds_ru.clone()
    };
    lines.push(format!("sounds.ru = {}", esc_arr(&sounds)));

    if !cmd.phrases_ru.is_empty() {
        lines.push(format!("phrases.ru = {}", esc_arr(&cmd.phrases_ru)));
    }
    if !cmd.phrases_en.is_empty() {
        lines.push(format!("phrases.en = {}", esc_arr(&cmd.phrases_en)));
    }
    if !cmd.patterns.is_empty() {
        lines.push(format!("patterns = {}", esc_arr(&cmd.patterns)));
    }
    if !cmd.response_sound.is_empty() {
        lines.push(format!("response_sound = {}", esc(&cmd.response_sound)));
    }

    lines.join("\n") + "\n"
}

/// Copy all bundled command packs to the user commands directory on first run.
/// Skips seeding if user dir already has any content.
/// This allows the user to see and edit default packs from the GUI.
pub fn seed_user_commands() {
    use jarvis_core::config::{user_commands_dir, COMMANDS_PATH};
    use jarvis_core::APP_DIR;

    let user_dir = user_commands_dir();

    // Skip if user dir already has packs
    if user_dir.exists() {
        let has_content = fs::read_dir(&user_dir)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        if has_content {
            return;
        }
    }

    let bundled_dir = APP_DIR.join(COMMANDS_PATH);
    if !bundled_dir.exists() {
        return;
    }

    if let Err(e) = fs::create_dir_all(&user_dir) {
        eprintln!("Seed: cannot create user commands dir: {}", e);
        return;
    }

    let entries = match fs::read_dir(&bundled_dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let src_pack = entry.path();
        let pack_name = entry.file_name();
        let dst_pack = user_dir.join(&pack_name);
        if src_pack.is_dir() && src_pack.join("command.toml").exists() {
            if let Err(e) = copy_dir_recursive(&src_pack, &dst_pack) {
                eprintln!("Seed: failed to copy pack {:?}: {}", pack_name, e);
            }
        }
    }

    eprintln!("[STARTUP] Seeded user commands directory from bundled defaults");
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dst_path = dst.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}
