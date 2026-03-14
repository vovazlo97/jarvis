---
name: jarvis-plugin-scaffold
description: "Scaffold a complete new Jarvis plugin — creates plugin.json manifest, Rust crate structure, Event Bus integration boilerplate, and test templates. ALWAYS invoke when: adding a new plugin, creating a new automation agent, or extending Jarvis with new capabilities via the plugin system. Do not create plugin files manually — this skill ensures correct manifest schema, proper Event Bus subscription pattern, and all required files are generated. Use even if you just need 'a quick plugin' — the template is the standard."
---

## Plugin Scaffold — создание нового плагина

### Шаг 1 — интервью с пользователем

Задай эти вопросы перед генерацией файлов:

1. **Название** плагина (snake_case, например `weather_fetch`)
2. **Описание** — что делает плагин (одна фраза)
3. **Команды** — какие голосовые фразы он обрабатывает? (примеры)
4. **Разрешения** — нужен ли сетевой доступ? файловая система? запуск процессов?
5. **Slow Path или Fast Path?** — плагин реагирует мгновенно (<250ms) или может работать async?

### Шаг 2 — структура файлов

```
plugins/<name>/
├── plugin.json          ← манифест (ОБЯЗАТЕЛЕН)
├── src/
│   └── lib.rs           ← основной код с Event Bus подпиской
├── tests/
│   └── integration.rs   ← тест подписки
└── README.md            ← описание команд и примеры
```

### Шаг 3 — plugin.json (заполни по ответам из интервью)

```json
{
  "id": "<name>",
  "version": "0.1.0",
  "name": "<Human Readable Name>",
  "description": "<что делает плагин>",
  "author": "jarvis-team",
  "commands": [
    {
      "id": "<name>.<command>",
      "trigger": "<голосовая фраза>",
      "description": "<что делает команда>"
    }
  ],
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

### Шаг 4 — src/lib.rs (Event Bus шаблон)

Плагины общаются только через Event Bus — никаких прямых вызовов других модулей:

```rust
//! Jarvis plugin: <name>
//! Handles <command> commands via Event Bus subscription.

use jarvis_core::eventbus::{self, JarvisEvent};

pub async fn run() -> anyhow::Result<()> {
    let mut rx = eventbus::subscribe()?;

    while let Ok(event) = rx.recv().await {
        if let JarvisEvent::CommandRecognized { id, text } = event {
            if id.starts_with("<name>.") {
                handle_command(&id, &text).await?;
            }
        }
    }
    Ok(())
}

async fn handle_command(id: &str, text: &str) -> anyhow::Result<()> {
    tracing::info!("Plugin <name>: handling {id} — text: {text}");
    // TODO: implement command logic
    // Publish result back to Event Bus if needed:
    // eventbus::publish(JarvisEvent::CommandExecuted { id: id.to_string(), success: true })?;
    Ok(())
}
```

### Шаг 5 — tests/integration.rs

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_plugin_can_subscribe_to_event_bus() {
        // Verify plugin can subscribe without panicking
        let rx = jarvis_core::eventbus::subscribe();
        assert!(rx.is_ok(), "Plugin must be able to subscribe to Event Bus");
    }

    #[tokio::test]
    async fn test_plugin_json_is_valid() {
        let manifest = std::fs::read_to_string("plugin.json").unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        assert!(parsed["id"].is_string(), "plugin.json must have string id");
        assert!(parsed["version"].is_string(), "plugin.json must have version");
    }
}
```

### Шаг 6 — напомни о следующих шагах

После создания файлов:

1. Зарегистрируй плагин в `src-tauri/src/main.rs` (или `app.rs`) — вызови `tokio::spawn(plugins::<name>::run())`
2. Если плагин реагирует на аудио или STT → вызови `/jarvis-fast-path-guardian`
3. После реализации логики → вызови `/jarvis-doc-writer`
4. Перед коммитом → вызови `/jarvis-commit-bot`

### Вывод

```
## Плагин '<name>' создан

Файлы:
- plugins/<name>/plugin.json
- plugins/<name>/src/lib.rs
- plugins/<name>/tests/integration.rs
- plugins/<name>/README.md

Следующие шаги:
1. Реализуй handle_command() в src/lib.rs
2. Зарегистрируй run() в main.rs
3. /jarvis-doc-writer → /jarvis-commit-bot
```
