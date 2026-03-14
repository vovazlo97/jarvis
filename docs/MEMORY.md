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
| ADR-011 | Linux CI scope: jarvis-core + jarvis-cli --no-default-features only | pv_recorder, rustpotter, kira, rodio require Windows DLLs or ALSA; jarvis-app and jarvis-gui excluded from Linux CI; Windows CI covers full workspace; Phase E target: abstract audio layer for cross-platform | 2026-03-15 |

## Current State

- **Version:** 0.4.0 (Phase D in progress → v0.5.0 target)
- **Active branch:** feature/phase-c-skills-mcp
- **Phase:** D — GitOps
- **Last milestone:** Phase D complete — .github/workflows/ci.yml + release.yml, dependabot.yml, branch-protection.md, CODEOWNERS, CHANGELOG.md created; CI badge added to README.md (2026-03-15)

## Known Issues & Technical Debt

| Task | Issue | Notes | Status |
|---|---|---|---|
| ~~TASK-006~~ | ~~Hot-reload не работает~~ | Root cause найден: intent::reinit() silently skipped embedding backend | Закрыт |
| ~~TASK-009~~ | ~~`cargo clippy -D warnings` fails — 40 violations~~ | Исправлено: MutexGuard await, &PathBuf→&Path, derivable_impls, dead code; commit dd67850 | Закрыт |

## Key Contacts & Context

- **Repo owner:** vovazlo97
- **Original fork:** Priler/jarvis
- **Goal:** 100K GitHub stars — open plugin marketplace + AI automation OS
- **Stack:** Rust 1.75+ / Tauri / Vosk / Vite+Svelte+TS
