# Jarvis Project Memory

## Architectural Decisions (ADR)

| # | Decision | Rationale | Date |
|---|---|---|---|
| ADR-001 | Event Bus via tokio broadcast | Decoupling modules, async-first | 2026-03-13 |
| ADR-002 | Fast Path: NO LLM calls | Latency guarantee <250ms | 2026-03-13 |
| ADR-003 | Plugin Manifest required | Standardized plugin lifecycle | 2026-03-13 |
| ADR-004 | Scripts via live disk reads | No stale cache, instant delete takes effect | 2026-03-08 |
| ADR-005 | Hot-reload via IpcAction::ReloadCommands | Existing WebSocket IPC path used — no new Event Bus events needed; scripts virtual cmds merged into intent reinit; EmbeddingClassifier supports hot-swap via RwLock | 2026-03-14 |
| ADR-006 | Fast Path lives in fast_path.rs module | Clear latency boundary; all <250ms code is auditable in one module with hard-constraint doc block | 2026-03-14 |
| ADR-007 | Model catalog filters Git LFS pointers at scan time | Prevents ONNX parse crash at runtime; `available: bool` propagated to GUI via `list_available_models` Tauri command | 2026-03-14 |
| ADR-008 | Intent EmbeddingClassifier falls back to all-MiniLM-L6-v2 | Multilingual model may be LFS pointer (not downloaded); English fallback always present as real binary | 2026-03-14 |
| ADR-009 | slots_backend normalized case-insensitively | Settings file stores "None" (capital N); GLiNER loader was triggered incorrectly; normalize early in init() | 2026-03-14 |
| ADR-010 | User data in app_data_dir(), NEVER in resources/ | Two-layer model: bundled (read-only, git) + user (persistent, %APPDATA%\com.priler.jarvis\); merged at startup, user wins; GUI CRUD writes only to user layer; first-run seeding copies bundled defaults | 2026-03-14 |

## Current State

- **Version:** 0.2.0
- **Active branch:** docs/phase-b-architecture
- **Phase:** B — Documentation
- **Last milestone:** TASK-010 architecture.md written — crate map, Fast Path pipeline, Event Bus, Command Registry, Plugin System, Data Storage, Model Catalog, ADR summary (2026-03-14)

## Known Issues & Technical Debt

| Task | Issue | Notes | Status |
|---|---|---|---|
| ~~TASK-006~~ | ~~Hot-reload не работает~~ | Root cause найден: intent::reinit() silently skipped embedding backend | Закрыт |
| TASK-009 (new) | `cargo clippy -D warnings` fails — 40 violations in jarvis-core | &PathBuf→&Path, derivable_impls, redundant &*, MutexGuard across await (HIGH), unused imports; GUI/app: dead code, unused imports | В работе (rust-engineer) |

## Key Contacts & Context

- **Repo owner:** vovazlo97
- **Original fork:** Priler/jarvis
- **Goal:** 100K GitHub stars — open plugin marketplace + AI automation OS
- **Stack:** Rust 1.75+ / Tauri / Vosk / Vite+Svelte+TS
