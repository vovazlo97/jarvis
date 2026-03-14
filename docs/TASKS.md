# Jarvis Task Board

## 🎯 Current Sprint — Phase A: Core Stabilization

### In Progress

- [ ] TASK-003: Define Plugin Manifest schema (plugin.json)
- [ ] TASK-004: Define Agent Registry schema
- [ ] TASK-005: Extract Fast Path boundary into separate module
- [ ] TASK-006: Hot-reload команд и скриптов без перезапуска
  - ⚠️ Partial fix: frontend вызывает reload_jarvis_commands после CRUD (коммит 98a6704),
    ReloadCommands handler включает script virtual cmds (коммит e92a040),
    НО реальный hot-reload всё равно не работает — команда не подхватывается без перезапуска.
    Требует глубокого исследования: возможно WebSocket-соединение не устанавливается,
    или IpcAction::ReloadCommands не доходит до jarvis-app во время работы GUI.

### Done

- [x] TASK-000: Create .claude/ infrastructure + CLAUDE.md + MEMORY.md + TASKS.md
- [x] TASK-001: Implement Event Bus (tokio broadcast channels)
- [x] TASK-002: Refactor Command Registry with atomic writes

## 📋 Backlog — Phase B: Documentation

- [ ] TASK-010: Write architecture.md
- [ ] TASK-011: Write plugin.md (SDK docs)
- [ ] TASK-012: Write sdk.md
- [ ] TASK-013: Create docs/tasks/roadmap.md

## 📋 Backlog — Phase C: Skills + MCP

- [ ] TASK-020: Create skill jarvis-architecture
- [ ] TASK-021: Create skill jarvis-fast-path-guardian
- [ ] TASK-022: Create skill jarvis-doc-writer
- [ ] TASK-023: Create skill jarvis-commit-bot
- [ ] TASK-024: Create skill jarvis-plugin-scaffold
- [ ] TASK-025: Configure GitHub MCP server
- [ ] TASK-026: Configure Filesystem MCP server

## 📋 Backlog — Phase D: GitOps

_(To be defined)_

## 📋 Backlog — Phase E: Community & Marketplace

_(To be defined)_
