use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

// ── Hardcoded fallback paths ──────────────────────────────────────────────────
#[cfg(target_os = "windows")]
pub const CHROME_EXE: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
#[cfg(not(target_os = "windows"))]
pub const CHROME_EXE: &str = "/usr/bin/google-chrome";

#[cfg(target_os = "windows")]
pub const STEAM_EXE: &str = r"C:\Program Files (x86)\Steam\steam.exe";

use seqdiff::ratio;

mod structs;
pub use structs::*;

use crate::{config, i18n, APP_DIR};

#[cfg(feature = "lua")]
use crate::lua::{self, CommandContext, SandboxLevel};

/// Core merge logic — reads command packs from two directories.
/// User packs override bundled packs with the same folder name.
/// Missing directories are silently skipped (no panic).
pub fn parse_commands_from_dirs(bundled_dir: &Path, user_dir: &Path) -> Vec<JCommandsList> {
    let mut packs: std::collections::HashMap<String, JCommandsList> =
        std::collections::HashMap::new();

    // Load bundled packs first (lower priority)
    if let Ok(entries) = fs::read_dir(bundled_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let toml_file = path.join("command.toml");
            if !toml_file.exists() {
                continue;
            }
            let pack_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if let Some(list) = load_pack(&path, &toml_file) {
                packs.insert(pack_name, list);
            }
        }
    }

    // Load user packs second — overrides bundled packs with same name
    if user_dir.exists() {
        if let Ok(entries) = fs::read_dir(user_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let toml_file = path.join("command.toml");
                if !toml_file.exists() {
                    continue;
                }
                let pack_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if let Some(list) = load_pack(&path, &toml_file) {
                    packs.insert(pack_name, list);
                }
            }
        }
    }

    packs.into_values().collect()
}

fn load_pack(pack_path: &Path, toml_file: &Path) -> Option<JCommandsList> {
    let content = match fs::read_to_string(toml_file) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to read {}: {}", toml_file.display(), e);
            return None;
        }
    };
    match toml::from_str::<JCommandsList>(&content) {
        Ok(file) => Some(JCommandsList {
            path: pack_path.to_path_buf(),
            commands: file.commands,
        }),
        Err(e) => {
            warn!("Failed to parse {}: {}", toml_file.display(), e);
            None
        }
    }
}

pub fn parse_commands() -> Result<Vec<JCommandsList>, String> {
    let bundled_dir = APP_DIR.join(config::COMMANDS_PATH);
    let user_dir = config::user_commands_dir();

    let commands = parse_commands_from_dirs(&bundled_dir, &user_dir);

    if commands.is_empty() {
        Err("No commands found".into())
    } else {
        info!("Loaded {} command pack(s)", commands.len());
        Ok(commands)
    }
}

pub fn commands_hash(commands: &[JCommandsList]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();

    let lang = i18n::get_language();
    hasher.update(lang.as_bytes());
    hasher.update(b"|");

    // collect all command ids and phrases for current language, sorted
    let mut all_data: Vec<(&str, _)> = commands
        .iter()
        .flat_map(|ac| {
            ac.commands
                .iter()
                .map(|c| (c.id.as_str(), c.get_phrases(&lang)))
        })
        .collect();
    all_data.sort_by_key(|(id, _)| *id);

    for (id, phrases) in all_data {
        hasher.update(id.as_bytes());
        for phrase in phrases.iter() {
            hasher.update(phrase.as_bytes());
        }
    }

    format!("{:x}", hasher.finalize())
}

pub fn fetch_command<'a>(
    phrase: &str,
    commands: &'a [JCommandsList],
) -> Option<(&'a PathBuf, &'a JCommand)> {
    let lang = i18n::get_language();

    let phrase = phrase.trim().to_lowercase();
    if phrase.is_empty() {
        return None;
    }

    // Regex pass — takes priority over fuzzy matching
    #[cfg(feature = "regex")]
    {
        use regex::Regex;
        for cmd_list in commands {
            for cmd in &cmd_list.commands {
                for pattern in &cmd.patterns {
                    match Regex::new(pattern) {
                        Ok(re) => {
                            if re.is_match(&phrase) {
                                info!(
                                    "Regex match: '{}' -> cmd '{}' (pattern: '{}')",
                                    phrase, cmd.id, pattern
                                );
                                return Some((&cmd_list.path, cmd));
                            }
                        }
                        Err(e) => {
                            warn!(
                                "Invalid regex pattern '{}' in cmd '{}': {}",
                                pattern, cmd.id, e
                            );
                        }
                    }
                }
            }
        }
    }

    let phrase_chars: Vec<char> = phrase.chars().collect();
    let phrase_words: Vec<&str> = phrase.split_whitespace().collect();

    let mut result: Option<(&PathBuf, &JCommand)> = None;
    let mut best_score = config::CMD_RATIO_THRESHOLD;

    for cmd_list in commands {
        for cmd in &cmd_list.commands {
            let cmd_phrases = cmd.get_phrases(&lang);

            for cmd_phrase in cmd_phrases.iter() {
                let cmd_phrase_lower = cmd_phrase.trim().to_lowercase();
                let cmd_phrase_chars: Vec<char> = cmd_phrase_lower.chars().collect();

                // character-level similarity
                let char_ratio = ratio(&phrase_chars, &cmd_phrase_chars);

                // word-level similarity
                let cmd_words: Vec<&str> = cmd_phrase_lower.split_whitespace().collect();
                let word_score = word_overlap_score(&phrase_words, &cmd_words);

                // combined score
                let score = (char_ratio * 0.6) + (word_score * 0.4);

                // early exit on perfect match
                if score >= 99.0 {
                    debug!("Perfect match: '{}' -> '{}'", phrase, cmd_phrase_lower);
                    return Some((&cmd_list.path, cmd));
                }

                if score > best_score {
                    best_score = score;
                    result = Some((&cmd_list.path, cmd));
                }
            }
        }
    }

    if let Some((_, cmd)) = result {
        info!(
            "Fuzzy match: '{}' -> cmd '{}' (score: {:.1}%)",
            phrase, cmd.id, best_score
        );
    } else {
        debug!("No match for '{}' (best: {:.1}%)", phrase, best_score);
    }

    result
}

fn word_overlap_score(input_words: &[&str], cmd_words: &[&str]) -> f64 {
    if input_words.is_empty() || cmd_words.is_empty() {
        return 0.0;
    }

    let mut matched = 0.0;

    // pre-compute cmd word chars to avoid repeated allocations
    let cmd_word_chars: Vec<Vec<char>> = cmd_words.iter().map(|w| w.chars().collect()).collect();

    for input_word in input_words {
        let input_chars: Vec<char> = input_word.chars().collect();

        let best_word_match = cmd_word_chars
            .iter()
            .map(|cw| ratio(&input_chars, cw))
            .fold(0.0_f64, f64::max);

        if best_word_match > 70.0 {
            matched += best_word_match / 100.0;
        }
    }

    let max_words = input_words.len().max(cmd_words.len()) as f64;
    (matched / max_words) * 100.0
}

pub fn execute_exe(exe: &str, args: &[String]) -> std::io::Result<Child> {
    Command::new(exe).args(args).spawn()
}

/// Launch Chrome with an optional URL.
/// Falls back to CHROME_EXE constant if the system default isn't available.
pub fn launch_browser(url: Option<&str>) -> std::io::Result<Child> {
    let chrome = Path::new(CHROME_EXE);
    let mut cmd = Command::new(chrome);
    if let Some(u) = url {
        cmd.arg(u);
    }
    if let Some(dir) = chrome.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn()
}

pub fn execute_cli(cmd: &str, args: &[String]) -> std::io::Result<Child> {
    debug!("Spawning: cmd /C {} {:?}", cmd, args);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("cmd")
            .arg("/C")
            .arg(cmd)
            .args(args)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
    }

    #[cfg(not(target_os = "windows"))]
    Command::new("sh").arg("-c").arg(cmd).args(args).spawn()
}

pub fn execute_command(
    cmd_path: &Path,
    cmd_config: &JCommand,
    _phrase: Option<&str>,
    _slots: Option<&HashMap<String, SlotValue>>,
) -> Result<bool, String> {
    // execute command by the type
    match cmd_config.cmd_type.as_str() {
        // BRUH
        "voice" => Ok(cmd_config.allow_chaining),

        // LUA command
        #[cfg(feature = "lua")]
        "lua" => execute_lua_command(cmd_path, cmd_config, _phrase, _slots),

        // Direct executable launch (games, apps)
        // Uses exe's own directory as working dir so DLL lookups succeed
        "exe" | "ahk" => {
            let exe_path_absolute = Path::new(&cmd_config.exe_path);
            let exe_path_local = cmd_path.join(&cmd_config.exe_path);

            let exe_path = if exe_path_absolute.exists() {
                exe_path_absolute.to_path_buf()
            } else {
                exe_path_local.clone()
            };

            let work_dir = exe_path
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| exe_path.clone());

            info!(
                "Launching exe: {} (cwd: {})",
                exe_path.display(),
                work_dir.display()
            );

            Command::new(&exe_path)
                .args(&cmd_config.exe_args)
                .current_dir(&work_dir)
                .spawn()
                .map(|_| cmd_config.allow_chaining)
                .map_err(|e| format!("Exe spawn error: {}", e))
        }

        // URL — opens URL in Chrome directly (no cmd /C start, no about:blank issues)
        // Store the URL in cli_cmd field: cli_cmd = "https://youtube.com"
        "url" => {
            let url = cmd_config.cli_cmd.trim();
            let url_opt = if url.is_empty() { None } else { Some(url) };
            info!("Opening URL via Chrome: {:?}", url_opt);
            launch_browser(url_opt)
                .map(|_| cmd_config.allow_chaining)
                .map_err(|e| format!("Browser launch error: {}", e))
        }

        // CLI command type — cmd /C <cli_cmd> [args...], window hidden on Windows
        "cli" => {
            info!("CLI: {} {:?}", cmd_config.cli_cmd, cmd_config.cli_args);
            execute_cli(&cmd_config.cli_cmd, &cmd_config.cli_args)
                .map(|_| cmd_config.allow_chaining)
                .map_err(|e| format!("CLI command error: {}", e))
        }

        // TERMINATOR command (T1000)
        "terminate" => {
            std::thread::sleep(Duration::from_secs(2));
            std::process::exit(0);
        }

        // STOP CHANING
        "stop_chaining" => Ok(false),

        // other
        _ => {
            error!("Command type unknown: {}", cmd_config.cmd_type);
            Err(format!("Command type unknown: {}", cmd_config.cmd_type))
        }
    }
}

// look up a command by its ID
pub fn get_command_by_id<'a>(
    commands: &'a [JCommandsList],
    id: &str,
) -> Option<(&'a PathBuf, &'a JCommand)> {
    for cmd_list in commands {
        for cmd in &cmd_list.commands {
            if cmd.id == id {
                return Some((&cmd_list.path, cmd));
            }
        }
    }
    None
}

pub fn list_paths(commands: &[JCommandsList]) -> Vec<&Path> {
    commands.iter().map(|x| x.path.as_path()).collect()
}

#[cfg(feature = "lua")]
fn execute_lua_command(
    cmd_path: &Path,
    cmd_config: &JCommand,
    phrase: Option<&str>,
    slots: Option<&HashMap<String, SlotValue>>,
) -> Result<bool, String> {
    // get script path

    let script_name = if cmd_config.script.is_empty() {
        "script.lua"
    } else {
        &cmd_config.script
    };

    let script_path = cmd_path.join(script_name);

    if !script_path.exists() {
        return Err(format!("Lua script not found: {}", script_path.display()));
    }

    // parse sandbox level
    let sandbox = cmd_config
        .sandbox
        .parse::<SandboxLevel>()
        .unwrap_or_default();

    // create context
    let context = CommandContext {
        phrase: phrase.unwrap_or("").to_string(),
        command_id: cmd_config.id.clone(),
        command_path: cmd_path.to_path_buf(),
        language: i18n::get_language(),
        slots: slots.cloned(),
    };

    // get timeout
    let timeout = Duration::from_millis(cmd_config.timeout);

    info!(
        "Executing Lua command: {} (sandbox: {:?}, timeout: {:?})",
        cmd_config.id, sandbox, timeout
    );

    // execute
    match lua::execute(&script_path, context, sandbox, timeout) {
        Ok(result) => {
            info!(
                "Lua command {} completed (chain: {})",
                cmd_config.id, result.chain
            );
            Ok(result.chain)
        }
        Err(e) => {
            error!("Lua command {} failed: {}", cmd_config.id, e);
            Err(e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn cmd_from_toml(s: &str) -> JCommand {
        toml::from_str(s).unwrap_or_else(|e| panic!("TOML parse failed: {}\n---\n{}", e, s))
    }

    fn cmd_list(cmds: Vec<JCommand>) -> Vec<JCommandsList> {
        vec![JCommandsList {
            path: PathBuf::from("/test"),
            commands: cmds,
        }]
    }

    #[test]
    fn test_fuzzy_match_russian() {
        let list = cmd_list(vec![cmd_from_toml(
            r#"
            id = "greet"
            type = "voice"
            phrases.ru = ["привет джарвис", "здравствуй джарвис"]
        "#,
        )]);
        let r = fetch_command("привет джарвис", &list);
        assert!(r.is_some(), "должно находить по fuzzy");
        assert_eq!(r.unwrap().1.id, "greet");
    }

    #[test]
    fn test_no_match_returns_none() {
        let list = cmd_list(vec![cmd_from_toml(
            r#"
            id = "greet"
            type = "voice"
            phrases.ru = ["привет"]
        "#,
        )]);
        assert!(fetch_command("абракадабра xyz", &list).is_none());
    }

    #[cfg(feature = "regex")]
    #[test]
    fn test_regex_priority_over_fuzzy() {
        let list = cmd_list(vec![
            cmd_from_toml(
                r#"
                id = "youtube"
                type = "voice"
                phrases.ru = ["открой видео"]
                patterns = ["ютуб", "открой.*ютуб"]
            "#,
            ),
            cmd_from_toml(
                r#"
                id = "other"
                type = "voice"
                phrases.ru = ["другая команда"]
            "#,
            ),
        ]);
        let r = fetch_command("открой ютуб пожалуйста", &list);
        assert!(r.is_some());
        assert_eq!(
            r.unwrap().1.id,
            "youtube",
            "regex должен иметь приоритет над fuzzy"
        );
    }

    #[cfg(feature = "regex")]
    #[test]
    fn test_invalid_regex_no_panic() {
        let list = cmd_list(vec![cmd_from_toml(
            r#"
            id = "cmd"
            type = "voice"
            phrases.ru = ["тест"]
            patterns = ["[bad("]
        "#,
        )]);
        let _ = fetch_command("тест", &list); // не должен паниковать
    }

    #[test]
    fn test_voice_cmd_returns_false_by_default() {
        let cmd = cmd_from_toml("id = \"noop\"\ntype = \"voice\"\n");
        assert_eq!(
            execute_command(&PathBuf::from("/t"), &cmd, None, None),
            Ok(false)
        );
    }

    #[test]
    fn test_voice_cmd_with_allow_chaining_returns_true() {
        let cmd = cmd_from_toml("id = \"noop\"\ntype = \"voice\"\nallow_chaining = true\n");
        assert_eq!(
            execute_command(&PathBuf::from("/t"), &cmd, None, None),
            Ok(true)
        );
    }

    #[test]
    fn test_allow_chaining_default_is_false() {
        for t in &["voice", "exe", "url", "cli"] {
            let toml = format!("id = \"x\"\ntype = \"{}\"\n", t);
            let cmd: JCommand = toml::from_str(&toml).unwrap();
            assert!(
                !cmd.allow_chaining,
                "type={} should default to no chaining",
                t
            );
        }
    }

    #[test]
    fn test_allow_chaining_opt_in() {
        let cmd: JCommand =
            toml::from_str("id = \"x\"\ntype = \"voice\"\nallow_chaining = true\n").unwrap();
        assert!(cmd.allow_chaining);
    }

    #[test]
    fn test_stop_chaining_returns_false() {
        let cmd = cmd_from_toml("id = \"stop\"\ntype = \"stop_chaining\"\n");
        assert_eq!(
            execute_command(&PathBuf::from("/t"), &cmd, None, None),
            Ok(false)
        );
    }

    #[test]
    fn test_unknown_type_is_err() {
        let cmd = cmd_from_toml("id = \"unk\"\ntype = \"totally_unknown\"\n");
        assert!(execute_command(&PathBuf::from("/t"), &cmd, None, None).is_err());
    }

    fn write_command_toml(dir: &std::path::Path, id: &str) {
        let content = format!(
            "[[commands]]\nid = \"{}\"\ntype = \"cli\"\ncli_cmd = \"echo\"\nsounds.ru = [\"ok1\"]\n",
            id
        );
        fs::write(dir.join("command.toml"), content).unwrap();
    }

    /// User pack with same name as bundled must override bundled.
    #[test]
    fn user_pack_overrides_bundled_same_name() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("user");

        let b_games = bundled.join("games");
        fs::create_dir_all(&b_games).unwrap();
        write_command_toml(&b_games, "witcher");

        let u_games = user.join("games");
        fs::create_dir_all(&u_games).unwrap();
        write_command_toml(&u_games, "cyberpunk");

        let result = parse_commands_from_dirs(&bundled, &user);
        let ids: Vec<_> = result
            .iter()
            .flat_map(|l| l.commands.iter().map(|c| c.id.as_str()))
            .collect();
        assert!(
            ids.contains(&"cyberpunk"),
            "user pack must override bundled"
        );
        assert!(
            !ids.contains(&"witcher"),
            "bundled pack must be overridden by user"
        );
    }

    /// Unique user pack (not in bundled) must appear in merged result.
    #[test]
    fn unique_user_pack_included() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("user");
        fs::create_dir_all(&bundled).unwrap();

        let u_custom = user.join("my-custom");
        fs::create_dir_all(&u_custom).unwrap();
        write_command_toml(&u_custom, "my-cmd");

        let b_default = bundled.join("default");
        fs::create_dir_all(&b_default).unwrap();
        write_command_toml(&b_default, "default-cmd");

        let result = parse_commands_from_dirs(&bundled, &user);
        let ids: Vec<_> = result
            .iter()
            .flat_map(|l| l.commands.iter().map(|c| c.id.as_str()))
            .collect();
        assert!(ids.contains(&"my-cmd"), "user-only pack must be included");
        assert!(
            ids.contains(&"default-cmd"),
            "bundled-only pack must be included"
        );
    }

    /// Non-existent user dir must not panic — silently skip.
    #[test]
    fn missing_user_dir_is_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let bundled = tmp.path().join("bundled");
        let user = tmp.path().join("nonexistent_user");
        fs::create_dir_all(&bundled).unwrap();

        let b_pack = bundled.join("pack");
        fs::create_dir_all(&b_pack).unwrap();
        write_command_toml(&b_pack, "cmd");

        let result = parse_commands_from_dirs(&bundled, &user);
        assert_eq!(result.len(), 1);
    }
}
