# Jarvis Project Memory

## Architectural Decisions (ADR)

| # | Decision | Rationale | Date |
|---|---|---|---|
| ADR-001 | Event Bus via tokio broadcast | Decoupling modules, async-first | 2026-03-13 |
| ADR-002 | Fast Path: NO LLM calls | Latency guarantee <250ms | 2026-03-13 |
| ADR-003 | Plugin Manifest required | Standardized plugin lifecycle | 2026-03-13 |
| ADR-004 | Scripts via live disk reads | No stale cache, instant delete takes effect | 2026-03-08 |

## Current State

- **Version:** 0.1.0
- **Active branch:** develop/phase-a-core-stabilization
- **Phase:** A — Core Stabilization
- **Last milestone:** TASK-001 Event Bus implemented — JarvisEvent + broadcast channel in jarvis-core (2026-03-13)

## Known Issues & Technical Debt

_(Populated as issues are discovered)_

## Key Contacts & Context

- **Repo owner:** vovazlo97
- **Original fork:** Priler/jarvis
- **Goal:** 100K GitHub stars — open plugin marketplace + AI automation OS
- **Stack:** Rust 1.75+ / Tauri / Vosk / Vite+Svelte+TS
