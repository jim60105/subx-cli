# Design

## Context

See `proposal.md` for the failure and its evidence. Six facts from the current code shape the approach:

- `tests/integration_tests.rs` is a synchronous test target. `test_full_workflow` calls three helpers that each spawn `subx-cli` through `assert_cmd::Command`. Today only `test_match_command` sets any environment, and convert/sync resolve `dirs::config_dir()/subx/config.toml` (`subx-core/src/config/service.rs:256-261`), which `run_with_config` validates before dispatch.
- The `local` provider is the only one whose validator accepts a plain `http://` base URL (`subx-core/src/config/validator.rs:120-145`); hosted providers go through `require_https_for_hosted_provider`. It appends `/chat/completions` to `ai.base_url` and sends an `Authorization` header only when `ai.api_key` is set.
- `ProductionConfigService` reads the file named by `SUBX_CONFIG_PATH` first, then `SUBX_*` environment variables, and starts from `Config::default()` when the file omits fields (`subx-core/src/config/service.rs:217-224`, `load_and_validate`). `SUBX_AI_*` variables outrank the file, which is why they must be removed rather than merely not set.
- `display_match_results` returns before printing anything when the operation list is empty, and returns in JSON mode too (`src/cli/ui.rs:335-341`); only a non-empty list reaches the `Preview mode` banner at `:343-348`. `SUBX_OUTPUT=json` therefore silently defeats the assertion.
- `MatchEngine::get_cache_file_path` consults `XDG_CONFIG_HOME` before `dirs::config_dir()` (`subx-core/src/core/matcher/engine.rs:2536-2544`), so that variable redirects the match cache on all three platforms.
- `wiremock` 0.6.5 runs each `MockServer` on a dedicated OS thread with its own runtime (`bare_server.rs:76-90`); servers are pooled and recording is on by default, so `received_requests()` starts empty and is `Some`.

The constraint that decides most of the design: the AI endpoint must be reachable from a **separate child process**, and the mock server must be alive while that blocking child process runs.

## Goals / Non-Goals

**Goals:**

- `test_full_workflow` passes on Linux, Windows and macOS with no outbound network, no credentials and no user configuration, for all three of its legs.
- The match leg still exercises `analyze_content` → operation assembly → preview, not a short-circuit.
- One definition of "which AI variables must be stripped", shared by the two spawn sites this change touches.
- A failing CI leg is reported on each platform it affects.

**Non-Goals:**

- No production code, no production dependency and no `subx-core` change.
- No change to `match --dry-run` semantics; the preview must not skip AI matching.
- No new test framework; `wiremock` 0.6 and `regex` are already `[dev-dependencies]`.
- Migrating the other binary-spawning harnesses that select `local` through `SUBX_AI_*`/`LOCAL_LLM_*` variables, and folding their nine inline copies of the thirteen-name list into the constant. They are named in `proposal.md` and left for a follow-up; each needs its own harness shim wired through `mod common;`.
- Not touching the nextest profile, the coverage floors, or the CI scripts.

## Decisions

### 1. One isolated child environment for all three legs

A private helper in `tests/integration_tests.rs` builds each child command with: `current_dir(dir)`, `SUBX_CONFIG_PATH` = `dir/test_config.toml`, `XDG_CONFIG_HOME` = `dir/config-home`, `NO_PROXY`/`no_proxy` = `127.0.0.1,localhost`, and `env_remove` for `SUBX_OUTPUT` and every name in `ISOLATED_AI_ENV_VARS`. Match, convert and sync all use it; convert and sync never call the AI, but they do load and validate the config, so one valid file serves all three.

`SUBX_OUTPUT` is removed separately because it changes stdout shape, not provider selection, and folding it into the AI list would make that name a lie. The proxy pin is there because `reqwest`'s default client honours `HTTP(S)_PROXY`; loopback is not automatically exempt in every configuration.

Alternatives rejected:

- Isolating only the match leg: leaves `test_full_workflow` dependent on the developer's config in two of its three legs, which is the same class of defect this change exists to remove.
- `env_clear()`: removes `PATH`/`HOME`, breaking the child's own runtime lookups on some runners.

### 2. Start the mock server on a runtime the synchronous test owns

`test_match_command` stays a plain synchronous helper. It creates `tokio::runtime::Runtime::new()` (multi-thread, enabled by the crate's `tokio` dependency), starts `MockServer` and mounts the mock through `runtime.block_on(...)`, then spawns and asserts the child outside `block_on`. The runtime binding is held until after the request log has been read.

The mock server itself does not share that runtime: wiremock spawns it on its own OS thread with its own current-thread runtime, so a blocked test thread cannot starve it. The runtime is only needed to drive `MockServer::start()`'s async startup and the async mount/request-log calls. Keeping it in the synchronous helper is the smallest change; the file's three helpers and its `#[test]` entry point stay synchronous, and no `#[tokio::test]` restructuring is needed.

Alternatives rejected:

- Converting the test target to `#[tokio::test]`: the repository does use blocking `assert()` inside `#[tokio::test]` elsewhere (`tests/cli/match_command_json_silence.rs`), so it is workable, but it means rewriting `test_full_workflow` and its helpers' signatures for no behavioural gain. The synchronous shape is kept because that is the existing harness.

### 3. Select the `local` provider with a temporary `SUBX_CONFIG_PATH` file

The config file is written after `MockServer::start()` (the port is not known before) and contains only the AI section:

```toml
[ai]
provider = "local"
model = "mock-model"
base_url = "http://127.0.0.1:<port>/v1"
```

`<port>` is `MockServer::uri()`, never a hard-coded port. `Config` has no struct-level `serde(default)` and its sections are non-optional (`subx-core/src/config/mod.rs:113-133`), so `try_deserialize::<Config>()` fails and `load_and_validate` takes its partial path, copying `provider`/`model`/`base_url` onto `Config::default()`. The resulting `AIConfig` has `api_key = None`, so the local client sends no `Authorization` header and the mock does not require one. If `subx-core` ever gains serde defaults, the full-deserialize path wins and produces the same three values, so the contract does not depend on which branch runs.

The mock is mounted at `/v1/chat/completions` while `base_url` ends in `/v1`, because the provider appends `/chat/completions` verbatim. Sibling harnesses that pass a bare `server.uri()` mount `/chat/completions`; copying that path here would leave the mock unmatched.

Alternatives rejected:

- `SUBX_AI_PROVIDER` / `SUBX_AI_BASE_URL` env vars: they outrank the file and are exactly what the isolation rule removes, so the test would depend on a variable it forbids.
- A complete serialized `Config`: pins every default value in the test and turns a schema addition into an unrelated failure.
- Provider `openai` with `OPENAI_BASE_URL`: a hosted provider with an `http://` base URL fails validation, and relaxing that rule would change production behaviour to suit a test.

### 4. The mock derives its answer from the request prompt

A free function `fn echo_match_response(request: &wiremock::Request) -> ResponseTemplate` reads `messages[last].content`, extracts the file IDs inside the `<video_files>` block and inside the `<subtitle_files>` block, pairs them 1:1 at confidence `0.95` with `language: "en"`, and returns the standard OpenAI envelope (`choices[0].message.content` = the `MatchResult` JSON, plus a `usage` object). `wiremock`'s `Respond` blanket impl accepts the function pointer directly.

`fn ids_in_block(prompt, open, close) -> Vec<String>` slices between the two tags and matches the anchored regex `<file id="(file_[^"]+)"` within that slice. The anchor matters: `<content_samples>` also carries `subtitle_file_id="file_…"`, and an unanchored `id="…"` pattern would match that tail if the blocks were ever reordered. The responder panics when the video and subtitle counts differ and when either is empty, so a prompt-schema regression fails loudly instead of silently producing fewer pairs.

Pairing is block-scoped rather than "first N IDs in the prompt", which makes it independent of sample presence and inventory order.

Alternatives rejected:

- Hard-coded `file_<uuid>` values: break whenever ID generation or the prompt schema changes.
- `matches: []`: it still exercises the AI call, but `display_match_results` returns early on an empty list without printing `Preview mode`, so the existing assertion fails. That is the desired outcome — it keeps pairing coverage mandatory — but it means the responder cannot be empty as a shortcut.

### 5. One exported list of ambient AI variables

`tests/common/cli_helpers.rs` gains `pub const ISOLATED_AI_ENV_VARS: &[&str]` holding the thirteen names currently spelled inline in `run_command_with_config` (`OPENAI_API_KEY`, `OPENAI_BASE_URL`, `OPENROUTER_API_KEY`, the five `AZURE_OPENAI_*` names, `LOCAL_LLM_BASE_URL`, `LOCAL_LLM_API_KEY`, and the four `SUBX_AI_*` names). `run_command_with_config` iterates it, and `tests/integration_tests.rs` gains `mod common;` and iterates the same constant. A `&[&str]` constant is shared without coupling the two spawn types (`std::process::Command` vs `assert_cmd::Command`); each site keeps its own loop.

`XDG_CONFIG_HOME` is pointable on all three platforms because the match cache's resolver checks it first (`engine.rs:2538`). It does not redirect `ProductionConfigService`'s user-config fallback, but that fallback is never reached once `SUBX_CONFIG_PATH` is set.

`AGENTS.md`'s "Always use `TestConfigService`. Never use `ProductionConfigService`" is an in-process rule: a test that has no production binary cannot use the production loader. This test necessarily spawns the shipped binary, which always runs the production path; no test code calls `ProductionConfigService`.

Alternatives rejected:

- duplicating the name list in `tests/integration_tests.rs`: two copies of a security-relevant list drift, and the second copy is the one forgotten when a provider is added.
- using `HOME`: unnecessary once `SUBX_CONFIG_PATH` and `XDG_CONFIG_HOME` are set.

### 6. `fail-fast: false` on the superproject test matrix

The `coverage` job and `subx-core`'s test job already set it; the superproject's `test` job does not. The key is added to the `test` job's `strategy` block only. It does not disable nextest's own fail-fast inside a leg — that behaviour is unchanged, and with the failing test fixed there is nothing for it to abort. No `subx-core` file changes, so the submodule pointer stays put.

## Risks / Trade-offs

- **[A zero-pair response fails the test]** → Intended: `Preview mode` is only printed for a non-empty operation list. The request log assertion (`received.len() == 1`, path `/v1/chat/completions`) distinguishes "config fell back to a hosted provider / wrong path" (zero requests) from "matcher sent more than one request"; the responder's count panic distinguishes a prompt-shape regression.
- **[Proxy interception of loopback]** → `NO_PROXY`/`no_proxy` are pinned to `127.0.0.1,localhost` in every child; recorded because `reqwest`'s default client reads the proxy environment.
- **[The test now depends on the `local` provider's prompt/response contract]** → That contract is `subx-core`'s AI surface and is already covered by its own tests; this test derives its response from the prompt rather than pinning prompt text.
- **[Other binary-spawning harnesses remain non-hermetic]** → Accepted and recorded in `proposal.md`; they are a follow-up change. The new requirement is scoped to the full-workflow test so the spec does not claim what the tree does not do.
- **[Mock server startup failure is opaque]** → `MockServer::start()` panics with the bind error rather than surfacing later as a 401, because the mock is mounted before the child is spawned.

## Migration Plan

No deployed surface. Reverting is restoring the previous `test_match_command` body and dropping the workflow key; the constant and the three-leg child helper can stay, since they only deduplicate existing behaviour. The change must land as one commit because a partially applied version either fails the test (mock without config) or leaves the matrix cancellation in place.

## Open Questions

None.