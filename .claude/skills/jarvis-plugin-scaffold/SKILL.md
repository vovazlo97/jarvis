---
name: jarvis-plugin-scaffold
description: "Scaffold a new Jarvis plugin from template. Creates plugin.json, basic Rust structure, tests. Use when adding a new plugin."
---

## Plugin Scaffold

При активации попроси пользователя:
1. Название плагина (snake_case)
2. Описание что делает
3. Нужен ли сетевой доступ (y/n)
4. Нужен ли доступ к файловой системе (y/n)

Затем создай структуру:
```
plugins/<name>/
  plugin.json       (заполненный манифест)
  src/lib.rs        (базовый Rust код с трейтом Plugin)
  README.md         (описание + примеры)
  tests/            (integration test шаблон)
```
