# Jarvis Project Memory

## Architectural Decisions (ADR)

| # | Decision | Rationale | Date |
|---|---|---|---|
| ADR-001 | Event Bus via tokio broadcast | Decoupling modules, async-first | 2026-03-13 |
| ADR-002 | Fast Path: NO LLM calls | Latency guarantee <250ms | 2026-03-13 |
| ADR-003 | Plugin Manifest required | Standardized plugin lifecycle | 2026-03-13 |
| ADR-004 | Scripts via live disk reads | No stale cache, instant delete takes effect | 2026-03-08 |
| ADR-005 | Hot-reload via IpcAction::ReloadCommands | Existing WebSocket IPC path used — no new Event Bus events needed; scripts virtual cmds merged into intent reinit; EmbeddingClassifier supports hot-swap via RwLock | 2026-03-14 |

## Current State

- **Version:** 0.1.0
- **Active branch:** develop/phase-a-core-stabilization
- **Phase:** A — Core Stabilization
- **Last milestone:** TASK-006 Hot-reload complete — EmbeddingClassifier reinit implemented (OnceCell→RwLock), intent::reinit routes embedding arm; all 36 core tests pass (2026-03-14)

## Known Issues & Technical Debt

| Task | Issue | Notes | Status |
|---|---|---|---|
| ~~TASK-006~~ | ~~Hot-reload не работает~~ | Root cause найден: intent::reinit() silently skipped embedding backend | Закрыт |

## Key Contacts & Context

- **Repo owner:** vovazlo97
- **Original fork:** Priler/jarvis
- **Goal:** 100K GitHub stars — open plugin marketplace + AI automation OS
- **Stack:** Rust 1.75+ / Tauri / Vosk / Vite+Svelte+TS
