---
name: jarvis-architecture
description: "Load Jarvis architecture context: Event Bus, Command Registry, Fast/Slow Path, Plugin Manifest, Agent Registry. Use when designing or refactoring core modules."
---

## Контекст архитектуры Jarvis

При активации этого скила:
1. Прочитай docs/MEMORY.md секцию Architectural Decisions
2. Прочитай .claude/rules/fast-path.md
3. Держи в уме: Fast Path = NO LLM, <250ms. Slow Path = async agents.
4. Event Bus — единственный способ коммуникации между модулями
5. Каждый плагин ОБЯЗАН иметь plugin.json manifest
