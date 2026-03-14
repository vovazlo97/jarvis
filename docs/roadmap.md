# Jarvis Roadmap

> Goal: 100K GitHub stars — open plugin marketplace + offline AI automation OS.

---

## Vision

Jarvis is a **100% offline, privacy-first** voice assistant and automation platform for power users. No cloud subscriptions, no telemetry, no vendor lock-in. Everything runs locally on your machine.

Long-term pillars:
1. **Fast, reliable voice control** — <250ms P50 end-to-end
2. **Open plugin marketplace** — community-driven command packs and agents
3. **Local AI** — LLM-powered Slow Path for complex automation, no cloud required
4. **Cross-platform** — Windows first, Linux and macOS next

---

## Phase A — Core Stabilization ✅ `v0.2.0`

**Status: Complete (2026-03-14)**

Establish a solid, tested foundation before building features on top.

| Task | Status |
|---|---|
| Event Bus (tokio broadcast) | ✅ Done |
| Command Registry (atomic RwLock) | ✅ Done |
| Plugin Manifest schema (plugin.json) | ✅ Done |
| Agent Registry | ✅ Done |
| Fast Path boundary module | ✅ Done |
| Hot-reload (commands + scripts, no restart) | ✅ Done |
| Fast Path pipeline stabilization (LFS guard, model catalog, intent fallback) | ✅ Done |
| User data persistence (two-layer model, APP_CONFIG_DIR) | ✅ Done |
| Clippy gate (-D warnings): 0 violations | ✅ Done |

---

## Phase B — Documentation `v0.3.0`

**Status: In Progress (2026-03-14)**

Write the docs that make Jarvis approachable for contributors and plugin authors.

| Task | Status |
|---|---|
| `docs/architecture.md` — system design, ADRs, crate map | ✅ Done |
| `docs/plugin.md` — command packs, Lua scripts, Lua API, plugin.json | ✅ Done |
| `docs/sdk.md` — WebSocket IPC, Tauri invoke, Rust crate API | ✅ Done |
| `docs/roadmap.md` — this file | ✅ Done |

---

## Phase C — Skills + MCP `v0.4.0`

**Status: Backlog**

Give Claude Code (and future agents) structured, reusable skills for working on the Jarvis codebase.

| Task | Description |
|---|---|
| Skill: `jarvis-architecture` | Load ADRs and architecture context before core changes |
| Skill: `jarvis-fast-path-guardian` | Enforce Fast Path constraints before audio/STT changes |
| Skill: `jarvis-doc-writer` | Update docs after feature implementation |
| Skill: `jarvis-commit-bot` | Generate conventional commit messages |
| Skill: `jarvis-plugin-scaffold` | Scaffold new plugin skeleton |
| MCP: GitHub server | PR review, issue tracking, CI status via Claude |
| MCP: Filesystem server | Extended file operations for Claude agents |

---

## Phase D — GitOps `v0.5.0`

**Status: Backlog — to be defined**

Automate the development workflow: CI/CD, automated releases, changelog generation.

Planned areas:
- GitHub Actions CI (build + test on every PR)
- Automated release tagging on phase completion
- `CHANGELOG.md` generation from conventional commits
- PR template with TASK-ID linking
- Branch protection rules for `main`

---

## Phase E — Community & Marketplace `v1.0.0`

**Status: Backlog — to be defined**

Launch the open plugin marketplace and grow the community to 100K stars.

Planned areas:
- Plugin registry (discovery, install, update)
- Plugin signing and verification
- Community command pack repository
- Marketplace UI in the Jarvis GUI
- Contributor guide and plugin SDK tutorial
- Localization infrastructure (i18n beyond EN/RU)

---

## Future — Local AI (Slow Path) `post-v1.0`

| Feature | Notes |
|---|---|
| Companion LLM | Local Ollama / llama.cpp integration for natural language automation |
| Agent orchestration | Route complex requests through Agent Registry to LLM-backed agents |
| Slot extraction improvements | GLiNER or LLM-based NER as alternative to current template matching |
| Multilingual STT | Better language support beyond Russian (Vosk multilingual models) |
| Custom wake words | User-trainable wake word via Rustpotter |

**Hard constraint**: LLM calls are **Slow Path only**. Fast Path (<250ms) will never call an LLM.

---

## Versioning

| Version | Phase | Description |
|---|---|---|
| v0.1.x | Bootstrap | Initial fork + infrastructure |
| v0.2.0 | Phase A | Core Stabilization complete |
| v0.3.0 | Phase B | Documentation complete |
| v0.4.0 | Phase C | Skills + MCP complete |
| v0.5.0 | Phase D | GitOps complete |
| v1.0.0 | Phase E | Community-ready, marketplace launched |

---

## Contributing

See `docs/plugin.md` to start building command packs and scripts.
See `docs/sdk.md` for IPC and Rust crate integration.
See `docs/architecture.md` for system design context.
See `CLAUDE.md` for AI agent routing and project conventions.
