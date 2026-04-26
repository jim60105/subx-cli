## Context

SubX-CLI's AI layer sits behind the `AIProvider` async trait (`src/services/ai/mod.rs`) and is selected by `ComponentFactory::create_ai_provider` (`src/core/factory.rs`). Three hosted providers exist today: `openai`, `openrouter`, `azure-openai`. Each uses an OpenAI-style `POST /chat/completions` request and shares the `PromptBuilder`, `ResponseParser`, and `HttpRetryClient` traits, plus `retry::HttpRetryClient`, `cache.rs`, `error_sanitizer.rs`, and `security::warn_on_insecure_http_str`. The configuration system (`src/config/mod.rs`, `service.rs`, `validator.rs`, `field_validator.rs`) is dependency-injected; `ai.provider` is already in the field validator's allow-list as `local` but no concrete client is wired.

Local LLM runtimes have converged on OpenAI-compatible chat-completion endpoints:

| Runtime | Default base URL | Notes |
|---|---|---|
| Ollama | `http://localhost:11434/v1` | Native `/api/generate` plus OpenAI-compat `/v1/chat/completions`. Key not required. |
| LM Studio | `http://localhost:1234/v1` | OpenAI-compat. Optional API key. |
| llama.cpp `llama-server` | `http://localhost:8080/v1` | OpenAI-compat. Optional `--api-key`. |
| vLLM | `http://localhost:8000/v1` | OpenAI-compat, often deployed with a shared key. |
| text-generation-webui | `http://localhost:5000/v1` (`--api`) | OpenAI-compat. |

SubX's match and translate flows already abstract over the trait, so a new client only needs to implement `analyze_content`, `verify_match`, and `chat_completion` and integrate with the shared retry/prompt/response code path. The interesting design surface is therefore configuration, validation, error mapping, and privacy — not the trait itself.

## Goals / Non-Goals

**Goals:**
- Provide a first-class `local` AI provider that targets any OpenAI-compatible local, LAN, VPN, or remote endpoint with no provider-specific request/response divergence visible to upstream code. The `local` provider SHALL be **endpoint-agnostic**: `http://localhost:11434/v1`, `http://192.168.1.50:11434/v1`, `https://ollama.tailnet.ts.net/v1`, and `https://my-vllm.example.com/v1` are all valid configurations.
- Allow `ai.api_key` to be **absent** without breaking validation when `ai.provider = "local"`.
- Require `ai.base_url` for `local` (no implicit default; ambiguity here would silently route traffic to the wrong server or hosted provider). Both `http://` and `https://` are accepted; SubX SHALL NOT force a scheme upgrade for `local`.
- For hosted providers (`openai`, `openrouter`, `azure-openai`), require `https://` for any user-set `base_url` and reject `http://` at validation time. The error message SHALL direct the user toward `ai.provider = "local"` (or alias `"ollama"`) when they likely intended an OpenAI-compatible local endpoint.
- Preserve the privacy guarantee: when `local` is selected, no hosted provider is contacted, no telemetry is sent, and hosted-provider env vars (`OPENAI_API_KEY`, `OPENROUTER_API_KEY`, `AZURE_OPENAI_*`) MUST NOT silently switch the active provider away from `local`.
- Surface clear, actionable error messages distinct from generic AI errors: connection refused, DNS failure, model not found, server returned non-OpenAI JSON, server timed out.
- Reuse `HttpRetryClient`, `PromptBuilder`, `ResponseParser`, and `cache.rs` so that retry, JSON-schema parsing, and caching behavior match the hosted providers.
- Cover the new path with unit tests and wiremock-backed integration tests; do not depend on any real local LLM in CI.

**Non-Goals:**
- Native (non-OpenAI-compatible) Ollama API support (e.g. `/api/generate`, `/api/chat`, `/api/embeddings`). Users who need it can run Ollama's OpenAI-compat layer; native API may be a follow-up change.
- Bundling, downloading, launching, or supervising local LLM runtimes. SubX assumes the server is already running.
- Function-calling / tool-use / structured-output schemas beyond what hosted providers already require.
- Streaming responses. Match and translate flows accumulate full responses today.
- GPU / CPU resource management.
- Telemetry or usage metering for the local provider.

## Decisions

### Decision 1: Treat `local` as an OpenAI-compatible adapter, not a separate request schema

**Choice:** Implement `LocalLLMClient` as an OpenAI-style `POST {base_url}/chat/completions` client, mirroring `OpenRouterClient`'s structure, and reuse `PromptBuilder` / `ResponseParser` / `HttpRetryClient`.

**Rationale:** All five target runtimes converge on OpenAI's request/response shape. A separate request schema would fork the prompt/parsing pipeline for negligible benefit. Keeping the schema identical means matching behavior, JSON validation, and translation prompts work without engine-side conditionals.

**Alternatives considered:**
- Native Ollama API (`/api/chat`): rejected as primary path because it forks JSON parsing and excludes LM Studio / llama.cpp / vLLM. Possible follow-up.
- Generic "custom HTTP" provider with user-supplied request template: rejected as too open-ended for a first-class capability.

### Decision 2: Provider identifier is `local`, with `ollama` as a recognized alias

**Choice:** The canonical string is `local`. A single helper `normalize_ai_provider(value: &str) -> String` (in `src/config/field_validator.rs`) lowercases/trims its input and maps `"ollama"` → `"local"`. This helper is the **only** alias-resolution point in the codebase and is invoked by:

1. `subx config set ai.provider <value>` — persists the canonical form.
2. `subx config get ai.provider` — returns the canonical form.
3. `ProductionConfigService` env-var loading — `SUBX_AI_PROVIDER=ollama` is normalized **before** any precedence/scoping decision (including the hosted-provider env-var carve-out from Decision 4).
4. `validate_ai_config` — the validation arm match keys off the canonical value.
5. `ComponentFactory::create_ai_provider` — the dispatch match arm only ever sees the canonical value.

The factory therefore has a single `"local"` arm, never an `"ollama"` arm. `SUBX_AI_PROVIDER=ollama` is fully supported and behaves identically to `SUBX_AI_PROVIDER=local`.

**Rationale:** `local` is already in `field_validator.rs`'s allow-list and is runtime-agnostic. Many users will reach for `ollama` first; aliasing avoids a footgun without proliferating provider IDs. Concentrating canonicalization in one helper eliminates the ambiguity of "normalized at validation time" and prevents drift between read sites (e.g. CLI accepting `ollama` but factory not seeing it because env-var loading bypassed validation).

**Alternatives considered:**
- Multiple provider IDs (`ollama`, `lm-studio`, `llamacpp`): rejected — the wire protocol is identical; differentiating only confuses configuration.
- `openai-compatible`: rejected as too generic; it would invite confusion with hosted OpenAI.
- Per-call-site normalization without a shared helper: rejected — high risk of one site forgetting to normalize, leaving the canonical/alias divergence the original critique flagged.

### Decision 3: `ai.api_key` is optional for `local`; `ai.base_url` is required; scheme is unrestricted for `local` and HTTPS-only for hosted providers

**Choice:** `validate_ai_config` introduces a `local` arm that:
- Accepts `api_key` as `None` or any non-empty string passing `validate_api_key` (which is permissive).
- Requires `base_url` to be a non-empty, syntactically valid URL.
- Accepts BOTH `http://` and `https://` schemes — the `local` provider is endpoint-agnostic and may target loopback, LAN, VPN, or any reachable host.
- Validates `model`, `temperature`, `max_tokens` with the same helpers as hosted providers.

For hosted providers (`openai`, `openrouter`, `azure-openai`), the same validator SHALL require any user-set `base_url` to use the `https://` scheme. A non-`https` URL configured for a hosted provider SHALL fail validation with a message that names the offending field, the unsupported scheme, and explicitly directs the user to `ai.provider = "local"` (or alias `"ollama"`) when they intended an OpenAI-compatible local/LAN endpoint. Default base URLs for hosted providers (e.g. `https://api.openai.com/v1`) are unaffected.

**Rationale:** Local runtimes typically have no auth and frequently sit on plain HTTP within a trusted network segment (loopback, home LAN, container network, Tailscale tailnet). Forcing HTTPS for `local` would block legitimate setups. Conversely, hosted providers always speak HTTPS in production; a plain-HTTP `base_url` for `openai` is almost certainly either a misconfigured local endpoint or a security mistake — failing fast with a hint toward the `local` provider helps both cases. A missing `base_url` on `local` cannot be defaulted safely (different runtimes use different ports), so we fail fast with an actionable message.

### Decision 4: Privacy posture — no cross-provider env-var leakage

**Choice:** When the resolved `ai.provider = "local"` (after env-var application), `ProductionConfigService` SHALL NOT overwrite `ai.api_key` from `OPENAI_API_KEY` / `OPENROUTER_API_KEY` / `AZURE_OPENAI_API_KEY`, and SHALL NOT switch `ai.provider` to a hosted value because of those env vars.

**Implementation sketch:** In `src/config/service.rs`, after applying `SUBX_AI_*` and config-file values, if the resolved provider (post-`SUBX_AI_PROVIDER`) is `local`, skip the hosted-provider env-var application loop. New `LOCAL_LLM_API_KEY` and `LOCAL_LLM_BASE_URL` overrides are honored only when the resolved provider is `local`.

**Rationale:** A user who explicitly selects `local` is making a privacy choice. Allowing `OPENAI_API_KEY` (often present from unrelated tools) to silently flip the provider to `openai` would violate that intent.

**Alternatives considered:**
- Always apply hosted env vars; document the gotcha: rejected — privacy is the headline feature; surprises here are unacceptable.
- Require `--offline` flag to opt out of env-var leakage: rejected — breaks the "set it once and forget" config UX.

### Decision 5: Error mapping

**Choice:** `LocalLLMClient` SHALL map low-level errors into `SubXError::AiService` variants with messages prefixed by a stable tag so users (and tests) can distinguish them:
- `local LLM endpoint unreachable: connection refused at {sanitized_base_url}` — `reqwest::Error::is_connect()` / `ECONNREFUSED`.
- `local LLM endpoint timed out after {N}s: {sanitized_base_url}` — `reqwest::Error::is_timeout()`.
- `local LLM endpoint returned HTTP {status}: {sanitized_body}` — non-2xx, body sanitized via `error_sanitizer`.
- `local LLM model not found: {model}` — when the server returns a 404 or a body matching common "model not loaded" / "no such model" patterns.
- `local LLM response was not OpenAI-compatible JSON: {parse_error}` — JSON shape mismatch.

`{sanitized_base_url}` is produced by a dedicated helper `sanitize_base_url(&str) -> String` that strips userinfo (`user:password@`), query strings, and fragments, leaving only `scheme://host[:port]/path`. This conforms to the existing `error-handling` capability rule that error messages SHALL NOT include full URLs with query parameters or credentials. The helper is unit-tested independently and reused across all variants above.

**Rationale:** Users can act on these distinctions: start the server, load the model, fix the URL. They also enable scenario-based tests against wiremock. Sanitizing the base URL prevents accidental leakage when a user has embedded a token in the URL (some self-hosted runtimes accept `?api_key=` query auth, and our spec must not echo that back in errors or logs).

### Decision 5b: Hosted-provider errors hint toward the `local` provider when the failure pattern suggests a local-endpoint misconfiguration

**Choice:** Hosted-provider clients (`OpenAIClient`, `OpenRouterClient`, `AzureOpenAIClient`) SHALL classify their failures and append the following one-line advisory to the error message when ANY of these patterns matches:

1. **Configuration-time HTTPS mismatch** — `validate_ai_config` rejected the configured `base_url` because it was not `https://`. (Surfaced by the validator itself; no client roundtrip required.)
2. **Connection refused / DNS failure to a non-public host** — `reqwest::Error::is_connect()` succeeded against a hostname that resolves to a loopback or RFC1918 / RFC4193 / link-local address (i.e. a clearly private network), implying the user pointed a hosted provider at a local server.
3. **HTTP 200 with non-OpenAI-canonical body** — the response parsed as JSON but lacked the `choices[0].message.content` shape that all four target runtimes also produce; this rarely happens against the genuine hosted endpoints but is common against misconfigured local servers.

The appended hint reads (single line, English, suffix to the existing message):

> *"If you intended to call an OpenAI-compatible local or LAN endpoint, set `ai.provider = "local"` (or `ollama`) and configure `ai.base_url` to your endpoint."*

The hint SHALL be appended via the same `error_sanitizer` pipeline so credentials in the offending URL are not echoed. The hint is **advisory only** — it SHALL NOT auto-switch the provider, SHALL NOT retry against `local`, and SHALL NOT be emitted for genuine upstream failures (HTTP 401 / 429 / 5xx from the real hosted endpoint, where the user clearly does intend to use the hosted provider).

**Rationale:** A common failure mode for new users is to leave `ai.provider = "openai"` (the default) while pointing `ai.base_url` at `http://localhost:11434/v1`. Today this produces an opaque "connection refused" or "AI response parsing failed" error. Surfacing the `local` provider in the error gives the user a one-line fix instead of a documentation hunt.

**Alternatives considered:**
- Auto-switch the hosted provider to `local` on these failures: rejected — silent provider switching violates the privacy posture (Decision 4) symmetrically and surprises users who really did mean to call the hosted endpoint.
- Emit the hint in a separate log channel rather than the error message: rejected — JSON-mode users (per `machine-readable-output`) would lose it; the error envelope is the right surface.
- Limit the hint to validation-time only: rejected — runtime connection-refused against private addresses is the most common path users hit.

### Decision 6: Caching and retry parity

**Choice:** Reuse the existing `cache.rs` and `HttpRetryClient` as-is. The cache key already includes provider, model, and prompt, so `local` entries cannot collide with hosted entries.

**Rationale:** Free correctness; identical retry semantics across providers were an explicit goal of the existing `Shared HTTP Retry Abstraction` requirement.

### Decision 7: Documentation surface

**Choice:** A new "Local / Offline LLM" section in `docs/configuration-guide.md` and a new sub-section in `docs/ai-provider-integration-guide.md` with concrete `subx config set` snippets for Ollama, LM Studio, llama.cpp, and vLLM, plus README quick-start updates in both `README.md` and `README.zh-TW.md`. Document known compatibility limits: small models can fail strict-JSON parsing, some servers ignore `max_tokens` or `temperature`, `verify_match` accuracy depends on model capability.

## Risks / Trade-offs

- [Risk] **Small local models fail to produce valid JSON** for `analyze_content` / `verify_match`, causing cascading parse errors. → **Mitigation:** Document recommended models (≥7B instruct-tuned), surface the existing `AI response parsing failed` error verbatim, and recommend `subx match --confidence 0.0 --dry-run` as a smoke test.
- [Risk] **OpenAI-compat layer drift across runtimes** (e.g. Ollama dropping fields, vLLM rejecting unknown parameters). → **Mitigation:** Send only the OpenAI-canonical fields (`model`, `messages`, `temperature`, `max_tokens`); avoid provider-specific knobs; integration tests use a strict wiremock that mirrors the canonical OpenAI schema.
- [Risk] **Insecure HTTP warning false positives** for users running local servers on a non-loopback LAN address. → **Mitigation:** The existing `warn_on_insecure_http_str` already exempts loopback; LAN HTTP for `local` still warns (which is correct — operators on an untrusted network should know), but the warning is informational and SHALL NOT block the request. For hosted providers, plain HTTP is rejected at validation time with a hint toward `local`.
- [Risk] **User misconfiguration silently routes to hosted OpenAI** because of a stray `OPENAI_API_KEY`. → **Mitigation:** Privacy posture (Decision 4) eliminates this for `provider=local`.
- [Risk] **Performance** — local models are often 10–100× slower than hosted APIs, and the default `request_timeout_seconds=120` may be too low for large prompts. → **Mitigation:** Document recommended timeouts; `request_timeout_seconds` already supports up to 600.
- [Risk] **No streaming** means very large translation jobs hold a full response in memory. → **Mitigation:** Existing `max_tokens` cap and `max_sample_length` truncation already bound memory; revisit if user reports surface.
- [Trade-off] **Aliasing `ollama` → `local`** could mislead users into thinking native Ollama features (embeddings, model pull) are supported. → **Mitigation:** Documentation explicitly scopes the adapter to OpenAI-compat chat-completions.

## Migration Plan

This is purely additive. No existing config values change meaning. Rollout steps:

1. Land `LocalLLMClient`, factory wiring, validator branch, and env-var scoping behind no feature flag (no risk to existing providers because the dispatch is by `ai.provider` string).
2. Add tests: unit tests for `from_config` and validator; wiremock integration tests for `analyze_content`, `verify_match`, `chat_completion`, retry, and each error-mapping case.
3. Update documentation (`docs/configuration-guide.md`, `docs/ai-provider-integration-guide.md`, `README.md`, `README.zh-TW.md`) and `CHANGELOG.md` under `### Added`.
4. No database migration, no on-disk format change. Users opt in by setting `ai.provider = "local"` and `ai.base_url`.

**Rollback:** revert the change set; existing config files remain valid because `local` was already in the field-validator allow-list and is silently rejected at factory time on prior versions (already producing a clear "Unsupported AI provider" error).

## Open Questions

- Should we ship a curated list of "known-good" model names per runtime in documentation, or stay model-agnostic? **Tentative answer:** stay model-agnostic, link to each runtime's model registry.
- Should `chat_completion` for `local` fall back to `/api/chat` (Ollama native) on a 404 from `/v1/chat/completions`? **Tentative answer:** no — keep the contract single-protocol; revisit if users report runtimes that don't expose `/v1`.
- Should we introduce a `ai.offline = true` config flag as a belt-and-braces guard against any future hosted call paths? **Tentative answer:** defer; Decision 4 already enforces the same property by construction.
