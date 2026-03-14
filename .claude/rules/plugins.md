# Plugin & External Tools Rules

## Когда можно устанавливать внешние инструменты
Claude МОЖЕТ устанавливать внешние инструменты если:
1. Они нужны для задачи и нет стандартной альтернативы
2. Источник официальный (crates.io, npm official, GitHub official release)
3. Перед установкой Claude ОБЯЗАН сообщить пользователю: что, зачем, откуда

## Разрешённые операции
- cargo install <tool> — Rust утилиты (cargo-audit, cargo-criterion, cargo-expand)
- npm install / npx — Node/MCP серверы (@modelcontextprotocol/server-*)
- curl | sh — ТОЛЬКО официальные установщики с проверкой хеша
- pip install — Python утилиты для скриптов CI

## Запрещено без явного подтверждения пользователя
- Установка любых бинарников из неофициальных источников
- Изменение системных конфигов (PATH, registry, systemd)
- Установка глобальных npm пакетов без --prefix

## MCP серверы для подключения (при необходимости)
```
claude mcp add github-mcp -- npx -y @modelcontextprotocol/server-github
claude mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem .
claude mcp add memory -- npx -y @modelcontextprotocol/server-memory
```

## Plugin Manifest (plugin.json) — обязательные поля
```json
{
  "id": "unique-plugin-id",
  "version": "1.0.0",
  "name": "Human Readable Name",
  "description": "What this plugin does",
  "author": "username",
  "commands": [],
  "agents": [],
  "capabilities": [],
  "permissions": {
    "filesystem": false,
    "network": false,
    "processes": false
  },
  "endpoint": null,
  "signature": null
}
```
