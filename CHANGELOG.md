# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added — Phase D: GitOps
- GitHub Actions CI workflow (`fmt`, `clippy`, `test-linux`, `test-windows`)
- GitHub Actions release workflow (Windows binary + ZIP artifact on tag push)
- Dependabot configuration for weekly Cargo dependency updates
- Branch protection rules documentation (`.github/branch-protection.md`)
- `CODEOWNERS` file assigning @vovazlo97 as owner of all files
- `CHANGELOG.md` (this file)
- CI status badge in `README.md`

---

## [0.4.0] — 2026-03-14

### Phase C: Skills + MCP

### Added
- Project skill `jarvis-architecture` — loads ADRs and Fast Path context before any core change
- Project skill `jarvis-fast-path-guardian` — enforces Fast Path <250ms invariants before audio/STT changes
- Project skill `jarvis-doc-writer` — triggers docs update after feature implementation
- Project skill `jarvis-commit-bot` — generates Conventional Commit messages before every commit
- Project skill `jarvis-plugin-scaffold` — scaffolds new plugin skeleton with correct manifest
- Rules files: `.claude/rules/fast-path.md`, `.claude/rules/git-workflow.md`, `.claude/rules/plugins.md`
- GitHub MCP server and Filesystem MCP server configured in `.claude.json`
- Project roadmap (`docs/roadmap.md`) with Phase A–E milestones and versioning plan

---

## [0.3.0] — 2026-03-14

### Phase B: Documentation

### Added
- `docs/architecture.md` — crate map, Fast Path pipeline, Event Bus, Command Registry, Plugin System, Data Storage, Model Catalog, ADR summary
- `docs/plugin.md` — command packs, Lua scripts, full API reference, sandbox levels, `plugin.json` schema, agent registry, slots, hot-reload
- `docs/sdk.md` — WebSocket IPC protocol, Tauri invoke API, Rust crate integration, Event Bus, Command/Agent Registry, build commands

### Fixed
- 40 `cargo clippy -D warnings` violations: MutexGuard across await, `&PathBuf`→`&Path`, `derivable_impls`, dead code, unused imports (commit `dd67850`)

---

## [0.2.0] — 2026-03-14

### Phase A: Core Stabilization

### Added
- Event Bus via `tokio::broadcast` channels — decoupled async communication between modules (ADR-001)
- Command Registry with atomic `RwLock`-based reads/writes
- Plugin Manifest schema (`plugin.json`) — standardized plugin lifecycle definition (ADR-003)
- Agent Registry schema for automation agents
- Fast Path boundary module (`fast_path.rs`) — all <250ms code isolated and auditable (ADR-006)
- Hot-reload of commands and scripts without restart via `IpcAction::ReloadCommands` (ADR-005)
- Model catalog `available` flag — Git LFS pointers auto-excluded at scan time (ADR-007)
- Intent EmbeddingClassifier fallback to `all-MiniLM-L6-v2` when multilingual model unavailable (ADR-008)
- `slots_backend` case-insensitive normalization to prevent GLiNER mis-trigger (ADR-009)
- Two-layer user data model: bundled (read-only, git) + user (`%APPDATA%\com.priler.jarvis\`) (ADR-010)
  - `user_commands_dir()` / `user_scripts_dir()` in `config.rs`
  - `parse_commands_from_dirs()` + `parse_scripts_from_dirs()` merge at startup
  - GUI CRUD writes to user layer; first-run seeding copies bundled defaults
- `list_available_models` Tauri command — GUI reads model catalog dynamically
- AssistantState enum and `StateChanged` IPC events at all state transitions
- Wake-word Idle-only guarantee with regression tests
- Audio echo drain after command execution to restore wake-word sensitivity

### Fixed
- Script intent hijacking — strict confidence threshold enforcement
- Relative sound paths resolved against `SOUND_DIR`
- Audio state reset and stale frame drain after command execution

---

## [0.1.0] — 2026-03-08

### Bootstrap

### Added
- Initial public release — Commands & Scripts system (fork of [Priler/jarvis](https://github.com/Priler/jarvis))
- Commands GUI: create, edit, delete voice commands through a visual interface
- Scripts Engine: chain multiple commands into sequential or parallel workflows
- Voice-activated workflows: trigger any script with a spoken phrase or regex pattern
- Sound Manager: browse, import and preview voice pack sounds from Settings
- Vosk-based offline Speech-to-Text (Russian, English, Ukrainian models)
- fastembed intent classification (`all-MiniLM-L6-v2`)
- Rustpotter wake-word detection
- Two-process architecture: `jarvis-app` (voice engine) + `jarvis-gui` (Tauri configuration window)
- WebSocket IPC between backend and GUI
- `setup.ps1` one-command setup script

---

[Unreleased]: https://github.com/vovazlo97/jarvis/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/vovazlo97/jarvis/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/vovazlo97/jarvis/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/vovazlo97/jarvis/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/vovazlo97/jarvis/releases/tag/v0.1.0
