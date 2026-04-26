## Why

SubX-CLI currently ships AI-powered matching and translation only against hosted, network-based providers (`openai`, `openrouter`, `azure-openai`). Users who must run subtitles through an LLM under privacy, air-gapped, regulatory, or cost constraints have no first-class option: they must either skip AI features or repurpose `base_url` overrides whose semantics, validation, and error messages were never designed for local servers. Local LLM runtimes (Ollama, LM Studio, llama.cpp's `llama-server`, vLLM, text-generation-webui) now expose stable OpenAI-compatible HTTP endpoints, so a dedicated provider adapter can give SubX users offline, no-API-key, low-cost AI matching and translation without rewriting downstream engines.

## What Changes

- Introduce a new built-in AI provider identifier `local` (alias accepted: `ollama`) that targets any OpenAI-compatible local chat-completions endpoint (`POST {base_url}/chat/completions`).
- Add `LocalLLMClient` under `src/services/ai/local.rs` implementing `AIProvider` (`analyze_content`, `verify_match`, `chat_completion`) by reusing the shared `PromptBuilder`, `ResponseParser`, and `HttpRetryClient` traits, mirroring the structure of `openrouter.rs`.
- Wire the new provider through `ComponentFactory::create_ai_provider` (`src/core/factory.rs`) and update the unsupported-provider error message and tests to include `local`.
- Extend configuration:
  - `ai.provider = "local"` is fully supported (already permitted by the field validator's allow-list, but never wired).
  - `ai.api_key` becomes **optional** for `local`; missing key is normal, not an error.
  - `ai.base_url` becomes **required** for `local` (no public default; example: `http://localhost:11434/v1` for Ollama, `http://localhost:8080/v1` for llama.cpp/`llama-server`, `http://localhost:1234/v1` for LM Studio).
  - `ai.model` is required and treated as the local model identifier (e.g. `llama3.1:8b-instruct`, `qwen2.5:7b`, `Meta-Llama-3.1-8B-Instruct.Q4_K_M.gguf`).
- Privacy posture: when `ai.provider = "local"`, the client SHALL only contact the configured `base_url`. The `api_key` field SHALL NOT be read from `OPENAI_API_KEY` / `OPENROUTER_API_KEY` / `AZURE_OPENAI_*` (no cross-provider leakage), and no telemetry, no analytics, and no fallback to hosted endpoints SHALL occur.
- Add `SUBX_AI_BASE_URL` (already general-purpose) and a new optional `LOCAL_LLM_BASE_URL` env var honored only when provider is `local`, plus optional `LOCAL_LLM_API_KEY` for runtimes that gate access with a shared token (e.g. some self-hosted vLLM deployments).
- Validation:
  - `validate_ai_config` (`src/config/validator.rs`) accepts provider `local`, requires non-empty `base_url`, validates URL format, rejects empty `model`, accepts empty/missing `api_key`, and routes through the same `validate_temperature` / `validate_positive_number(max_tokens)` rules as other providers.
  - `field_validator.rs` continues to allow `local` and adds documentation/help strings; `set_config_value` / `get_config_value` in `src/config/service.rs` SHALL handle the same keys without provider-specific branching.
- Error handling: surface clear, actionable messages when the local server is unreachable, refuses connection, returns non-OpenAI-compatible JSON, or returns an unsupported model — distinct from generic `AiService` errors so users can tell "server not running" from "model not loaded."
- Insecure-HTTP warning: rely on the existing `warn_on_insecure_http_str` behavior, which already suppresses the warning for `localhost` / `127.0.0.1` base URLs (the expected configuration for local runtimes); no change to `secrets-protection` is required.
- Documentation: update `docs/configuration-guide.md`, `docs/ai-provider-integration-guide.md`, `README.md`, and `README.zh-TW.md` with a "Local / Offline LLM" section covering Ollama, LM Studio, llama.cpp, and vLLM examples; document known compatibility limits (e.g. some servers ignore `temperature`, `max_tokens`, or fail strict-JSON parsing on small models).
- Tests: add unit tests for `LocalLLMClient::from_config`, validator branches, env-var precedence, and integration tests using a wiremock-backed local endpoint covering match flow, translation `chat_completion`, retry behavior, and error mapping.
- Backward compatibility: existing `openai`/`openrouter`/`azure-openai` users are unaffected. No config keys are removed or renamed; only the `local` value gains real behavior.

## Capabilities

### New Capabilities
- `local-llm-provider`: First-class adapter for OpenAI-compatible local LLM runtimes (Ollama, LM Studio, llama.cpp `llama-server`, vLLM, text-generation-webui), including provider identification, endpoint/model resolution, optional-API-key semantics, offline-only network policy, and integration with the existing `AIProvider` trait, retry stack, prompt builder, and response parser.

### Modified Capabilities
- `ai-provider-integration`: Add `local` to the set of providers selectable via `config.ai.provider` and resolved by `ComponentFactory::create_ai_provider`. Update the unsupported-provider error message and the shared HTTP retry / prompt / response contract to include the new client.
- `configuration-management`: Treat `ai.api_key` as optional and `ai.base_url` as required when `ai.provider = "local"`; add provider-specific validation rules and `LOCAL_LLM_BASE_URL` / `LOCAL_LLM_API_KEY` environment overrides, scoped strictly to the local provider.

## Impact

- **Code:**
  - New: `src/services/ai/local.rs` (`LocalLLMClient`).
  - Modified: `src/services/ai/mod.rs` (re-export), `src/core/factory.rs` (`create_ai_provider` dispatch + unsupported-provider error string + tests), `src/config/mod.rs` (no schema change; documentation comment for `ai.provider`), `src/config/validator.rs` (new `local` branch), `src/config/field_validator.rs` (help strings; allow-list already includes `local`), `src/config/service.rs` (env-var handling for `LOCAL_LLM_*`, ensure no cross-provider key leakage).
  - Tests: `tests/` integration coverage with a wiremock-backed OpenAI-compatible mock; unit tests in `src/services/ai/local.rs` and `src/config/validator.rs`.
- **APIs:** No breaking changes to the `AIProvider` trait or to `AIConfig`. The set of accepted `ai.provider` values gains `local`.
- **Dependencies:** No new crates; reuses `reqwest`, `serde_json`, `async_trait`, `wiremock` (dev).
- **Docs:** `docs/configuration-guide.md`, `docs/ai-provider-integration-guide.md`, `README.md`, `README.zh-TW.md`, repo `AGENTS.md` (Module Guide, Supported Environment Variables, AI Provider System sections), plus a CHANGELOG entry under `### Added`.
- **CI / Quality:** Existing `scripts/quality_check.sh` covers the new tests; coverage threshold (75%) preserved. No new external services required by CI (mock server runs in-process).
- **User-facing:** Privacy-preserving, offline subtitle matching and translation; lower cost (no per-token billing); consistent UX with hosted providers via the same `subx config` keys and `subx match` / `subx translate` commands.
