# Jarvis Task Board

## 🎯 Current Sprint — Phase A: Core Stabilization

### Done

- [x] TASK-000: Create .claude/ infrastructure + CLAUDE.md + MEMORY.md + TASKS.md
- [x] TASK-001: Implement Event Bus (tokio broadcast channels)
- [x] TASK-002: Refactor Command Registry with atomic writes
- [x] TASK-003: Define Plugin Manifest schema (plugin.json)
- [x] TASK-004: Define Agent Registry schema
- [x] TASK-005: Extract Fast Path boundary into separate module
- [x] TASK-006: Hot-reload команд и скриптов без перезапуска
- [x] TASK-007: Fast Path pipeline stabilization — LFS detection, model catalog `available` flag, intent fallback chain, slots normalization, `list_available_models` Tauri command (2026-03-14)
- [x] TASK-008: User data persistence — two-layer data model (bundled + user), `user_commands_dir()`/`user_scripts_dir()` in config.rs, `parse_commands_from_dirs()` + `parse_scripts_from_dirs()` merge, GUI CRUD writes to user layer, first-run seeding; 67 tests pass (2026-03-14)
  - [x] TASK-008-1: `user_commands_dir()` / `user_scripts_dir()` in config.rs
  - [x] TASK-008-2: `parse_commands_from_dirs()` merge in jarvis-core
  - [x] TASK-008-3: `parse_scripts_from_dirs()` merge in jarvis-core
  - [x] TASK-008-4: GUI commands CRUD → user dir + `seed_user_commands()`
  - [x] TASK-008-5: GUI scripts CRUD → user dir + graceful fallback in `parse_scripts()`

## 🎯 Current Sprint — Phase B: Documentation

### In Progress

### Done

- [x] TASK-009: Fix 40 `cargo clippy -D warnings` violations — MutexGuard across await, &PathBuf→&Path, derivable_impls, dead code, unused imports; commit dd67850 (2026-03-14)
- [x] TASK-010: Write architecture.md — crate map, Fast Path pipeline, Event Bus, Command Registry, Plugin System, Data Storage, Model Catalog, ADR summary (2026-03-14)
- [x] TASK-011: Write plugin.md — command packs, Lua scripts, full API reference, sandbox levels, plugin.json schema, agent registry, slots, hot-reload (2026-03-14)
- [x] TASK-012: Write sdk.md — WebSocket IPC protocol, Tauri invoke API, Rust crate integration, Event Bus, Command/Agent Registry, build commands (2026-03-14)

## 🎯 Current Sprint — Phase C: Skills + MCP

### In Progress

### Done

- [x] TASK-020: Skill jarvis-architecture — load ADRs + Fast Path context before core changes (2026-03-14)
- [x] TASK-021: Skill jarvis-fast-path-guardian — enforce Fast Path invariants before audio/STT changes (2026-03-14)
- [x] TASK-022: Skill jarvis-doc-writer — update docs after feature implementation (2026-03-14)
- [x] TASK-023: Skill jarvis-commit-bot — generate conventional commit messages (2026-03-14)
- [x] TASK-024: Skill jarvis-plugin-scaffold — scaffold new plugin skeleton (2026-03-14)
- [x] TASK-013: Create docs/roadmap.md — Phase A–E milestones, versioning, future Local AI section (2026-03-14)

## 📋 Backlog — Phase C: Skills + MCP

- [ ] TASK-025: Configure GitHub MCP server
- [ ] TASK-026: Configure Filesystem MCP server

## 📋 Backlog — Phase D: GitOps

_(To be defined)_

## 📋 Backlog — Phase E: Community & Marketplace

_(To be defined)_
