# Fast Path Rules — HARD CONSTRAINTS

These rules apply to ALL code in crates/jarvis-core/src/fast_path/ and audio pipeline.

1. NO async calls to LLM APIs (OpenAI, Anthropic, Ollama, etc.)
2. NO blocking HTTP/network I/O
3. NO file I/O heavier than config reads
4. All processing must complete in <250ms P50
5. If you need LLM: emit an event to Event Bus → Slow Path handles it → result returned asynchronously

VIOLATION = automatic PR rejection. Always run `cargo bench fast_path` before committing fast-path changes.
