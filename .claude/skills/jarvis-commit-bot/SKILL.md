---
name: jarvis-commit-bot
description: "Generate conventional commit messages and PR descriptions. Use after completing any task before committing."
disable-model-invocation: false
---

## Commit Bot

При активации:
1. Выполни: git diff --staged (или git diff HEAD если нет staged)
2. Проанализируй изменения и классифицируй: feat/fix/refactor/docs/chore/test/perf
3. Сгенерируй сообщение по формату: type(scope): description
   - scope = крейт или модуль (core, registry, fast-path, plugin, docs)
   - description = что изменилось, не как
4. Предложи пользователю подтвердить сообщение перед git commit
5. Создай описание PR: ## Summary / ## Changes / ## Testing / ## Related Task
