---
name: jarvis-doc-writer
description: "Update Jarvis documentation: architecture.md, plugin.md, sdk.md, CHANGELOG.md, README.md based on code changes. Use after implementing new features."
---

## Doc Writer

При активации:
1. Проанализируй последние изменения (git diff HEAD~1 или описание задачи)
2. Обнови соответствующие файлы в docs/
3. Добавь запись в CHANGELOG.md в формате: ## [version] - date / ### Added|Changed|Fixed
4. Обнови README.md если изменился публичный API или фичи
5. Создай ветку docs/update-<area> и предложи PR
