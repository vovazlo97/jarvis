# JARVIS — Справочник команд

## Управление через GUI

Раздел **Commands** в приложении Jarvis — визуальный редактор команд. Открой GUI → нажми **Commands** в меню.

### Что умеет GUI

| Действие | Как |
|---|---|
| Посмотреть все команды | Открыть раздел Commands — таблица загружается автоматически |
| Добавить игру (Cyberpunk, Witcher) | Кнопка **Add Command** → тип **EXE App** → указать путь к .exe |
| Добавить сайт в Chrome (YouTube, Twitch) | **Add Command** → тип **Chrome / URL** → вставить URL |
| Добавить системное действие | **Add Command** → тип **CLI / PowerShell** |
| Удалить команду | Кнопка **Delete pack** → подтвердить повторным кликом |
| Обновить список | Кнопка ↺ Refresh |

### Пример: добавить игру

1. **Add Command**
2. Pack name: `god_of_war`
3. Type: **EXE App**
4. EXE path: `C:\Games\GodOfWar\GoW.exe`
5. Phrases RU: `запусти кратоса, кратос, бог войны`
6. Regex: `кратос|god.of.war`
7. **Add Command** → готово, перезапусти Jarvis-app

### Пример: добавить сайт

1. **Add Command**
2. Pack name: `twitch`
3. Type: **Chrome / URL**
4. URL: `https://twitch.tv`
5. Phrases RU: `открой твич, твич`
6. Regex: `твич|twitch`
7. **Add Command**

> После добавления команды через GUI нужно **перезапустить jarvis-app** (не GUI),
> чтобы он подхватил новый `command.toml`.

---

## Быстрый старт

1. Скажи **"Джарвис"** — услышишь звуковой сигнал готовности
2. После сигнала произнеси команду (не торопясь)
3. Jarvis воспроизведёт звук подтверждения и выполнит действие

---

## Все команды

### Системные

| Голосовой триггер | ID | Результат |
|---|---|---|
| "привет" / "здравствуй" / "как дела" | `greet` | Звук подтверждения |
| "который час" / "сколько времени" / "скажи время" | `tell_time` | Всплывающее окно с текущим временем |
| "выключись" / "выключи джарвис" / "стоп джарвис" | `terminate` | Завершение работы Jarvis через 2 сек |

### Браузер

| Голосовой триггер | ID | Результат |
|---|---|---|
| "открой браузер" / "нужен браузер" | `browser_open` | Chrome (новая вкладка) |
| "ютуб" / "открой ютуб" / "ютубчик" | `open_youtube` | Chrome → youtube.com |
| "гугл" / "открой гугл" / "гугл поиск" | `open_google` | Chrome → google.com |
| "гитхаб" / "открой гитхаб" | `open_github` | Chrome → github.com |
| "закрой браузер" / "выключи браузер" | `browser_close` | Закрывает Chrome / Edge / Firefox |

### Игры

| Голосовой триггер | ID | Результат |
|---|---|---|
| "стим" / "стима" / "запусти стим" / "открой стим" | `launch_steam` | `C:\Program Files (x86)\Steam\steam.exe` |
| "ведьмак" / "ведьмака" / "ведьмак три" / "запусти ведьмака" | `launch_witcher3` | `D:\Games\TheWitcher3\REDprelauncher.exe` |
| "киберпанк" / "запусти киберпанк" / "cyberpunk" | `launch_cyberpunk` | `D:\Games\Cyberpunk 2077\bin\x64\Cyberpunk2077.exe` |

### Приложения

| Голосовой триггер | ID | Результат |
|---|---|---|
| "блокнот" / "нотпад" / "открой блокнот" | `launch_notepad` | `C:\Program Files\Notepad++\notepad++.exe` |

---

## Характер Jarvis

Jarvis не молчит, когда не понял — у него есть характер.

### Фраза-заглушка (Fallback)

Если и Intent Recognition, и Fuzzy-матчинг не дали результата — Jarvis воспроизводит фразу из файла `not_found.*`:

> **«Чего вы пытаетесь добиться, сэр?»**

Эта фраза звучит **только** когда команда не распознана. Если команда понятна — сначала играет `ok*.wav`, потом выполняется действие. Никаких «добиться, сэр» при открытии YouTube.

### Как изменить фразу fallback

Замени файлы в `resources/sound/jarvis-remaster/` — файлы вида `not_found1.wav`, `not_found2.wav` и т.д. Jarvis выберет случайный.

---

## Как добавлять новые команды

### Структура папок

```
resources/
  commands/
    my_app/           ← новая папка (любое имя)
      command.toml    ← файл команды
```

### Шаблон: запуск .exe

```toml
[[commands]]
id = "launch_myapp"            # уникальный ID
type = "exe"
exe_path = "C:\\Path\\To\\App.exe"   # двойной \\ обязателен!
exe_args = []                  # аргументы (обычно пусто)
sounds.ru = ["ok1", "ok2", "ok3"]
phrases.ru = ["открой приложение", "запусти программу"]
patterns = ["приложение|my.?app"]   # regex (приоритет над fuzzy)
```

### Шаблон: Chrome с URL

```toml
[[commands]]
id = "open_mysite"
type = "exe"
exe_path = "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe"
exe_args = ["https://example.com"]
sounds.ru = ["ok1", "ok2", "ok3"]
phrases.ru = ["открой сайт", "зайди на сайт"]
patterns = ["сайт|example"]
```

### Шаблон: PowerShell (системные действия)

```toml
[[commands]]
id = "my_script"
type = "cli"
cli_cmd = "powershell"
cli_args = ["-NoProfile", "-WindowStyle", "Hidden", "-Command", "Ваш-Скрипт-Здесь"]
sounds.ru = ["ok1", "ok2"]
phrases.ru = ["запусти скрипт"]
```

### Hardcoded пути (в commands.rs)

В коде прописаны константы-запасные пути на случай если TOML-путь не найден:

```rust
// crates/jarvis-core/src/commands.rs
pub const CHROME_EXE: &str = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
pub const STEAM_EXE:  &str = r"C:\Program Files (x86)\Steam\steam.exe";
```

Если Chrome переехал — измени `exe_path` в `browser/command.toml`. Если нужно изменить запасной путь — правь эти константы.

### Типы команд

| Тип | Поведение |
|---|---|
| `exe` | Прямой запуск .exe; рабочий каталог = папка exe (DLL найдутся) |
| `cli` | `cmd /C` скрытое окно; для PowerShell и batch-скриптов |
| `url` | Открывает URL через Chrome напрямую (без `cmd /C start`) |
| `voice` | Только звук, никаких действий (заглушка / тест) |
| `lua` | Выполняет `script.lua` рядом с command.toml |
| `terminate` | Завершает Jarvis через 2 секунды |
| `stop_chaining` | Прерывает цепочку команд |

### Правила TOML для путей Windows

| Вариант | Пример | Работает? |
|---|---|---|
| Двойной бэкслеш | `"C:\\Games\\app.exe"` | Да |
| Прямой слеш | `"C:/Games/app.exe"` | Да |
| Одиночный бэкслеш | `"C:\Games\app.exe"` | Нет — ошибка парсинга |

### После правок — ресинхронизация (без пересборки)

```powershell
Remove-Item -Recurse -Force target\debug\resources\commands
Copy-Item -Recurse -Force resources\commands target\debug\resources\commands
```

Перезапусти Jarvis — изменения применятся без `cargo build`.

---

## Как работает распознавание

Три уровня в порядке приоритета:

1. **Regex** (`patterns`) — мгновенно, ловит любую форму слова: "ютуб", "ютубчика", "стима"
2. **Intent-классификатор** (ML, порог 75%) — понимает смысл всей фразы
3. **Fuzzy Levenshtein** (порог 75%) — нечёткое сравнение с `phrases`

Если все три не дали результата — звучит **«Чего вы пытаетесь добиться, сэр?»**
