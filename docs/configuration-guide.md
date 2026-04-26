# SubX Configuration Guide

SubX uses a layered configuration system with three sources, applied in
priority order:

1. **Environment variables** (highest priority)
2. **User configuration file** (`~/.config/subx/config.toml` on Linux/macOS,
   `%APPDATA%\subx\config.toml` on Windows)
3. **Built-in defaults** (lowest priority)

You can set a custom config file path with `SUBX_CONFIG_PATH`.

## Quick Start

```bash
# View all settings
subx-cli config list

# View a specific setting
subx-cli config get ai.provider

# Set a value
subx-cli config set ai.provider openai
subx-cli config set ai.api_key "<YOUR_API_KEY>"

# Reset everything to defaults
subx-cli config reset
```

## AI Configuration (`[ai]`)

This section controls AI provider selection and request behavior.

```toml
[ai]
provider = "openai"                            # openai, openrouter, azure-openai, or local
api_key = "<YOUR_API_KEY>"                    # API key (Option<String>)
model = "gpt-4.1-mini"                        # Model identifier
base_url = "https://api.openai.com/v1"        # API endpoint URL
max_sample_length = 3000                      # Max content length sent to AI (100–10000)
temperature = 0.3                             # Response randomness (0.0–2.0)
max_tokens = 10000                            # Max tokens in response (1–100000)
retry_attempts = 3                            # API retry count (1–10)
retry_delay_ms = 1000                         # Retry delay in milliseconds (100–10000)
request_timeout_seconds = 120                 # Request timeout in seconds
api_version = "2025-04-01-preview"            # Azure OpenAI API version (Option<String>)
```

### OpenRouter Provider

OpenRouter acts as a unified gateway to multiple AI models. Set the
`base_url` to the OpenRouter endpoint and choose any model from their
catalog.

```toml
[ai]
provider = "openrouter"
api_key = "<YOUR_API_KEY>"
model = "deepseek/deepseek-r1-0528:free"
base_url = "https://openrouter.ai/api/v1"
```

### Azure OpenAI Provider

Azure OpenAI uses deployment-based routing. The `model` field takes the
Azure deployment name (not the model name), and the `base_url` points to
your Azure resource endpoint. The `api_version` field is required.

```toml
[ai]
provider = "azure-openai"
api_key = "<YOUR_API_KEY>"
model = "your-deployment-id"
base_url = "https://your-resource.openai.azure.com"
api_version = "2025-04-01-preview"
```

### Local / Offline LLM Provider

Set `ai.provider = "local"` to drive subtitle matching and translation
through any OpenAI-compatible HTTP endpoint — including local runtimes such
as **Ollama**, **LM Studio**, **llama.cpp `llama-server`**, **vLLM**, and
text-generation-webui. The string `ollama` is accepted as an alias and is
normalized to `local` at config write time, so the on-disk value is always
canonical.

For the `local` provider:

- `ai.api_key` is **optional** — most local runtimes accept anonymous
  requests. Set it only if your runtime gates access (e.g., a self-hosted
  vLLM behind a shared bearer token).
- `ai.base_url` is **required** and has no default. Point it at the
  runtime's OpenAI-compatible chat-completions root (the path that, with
  `/chat/completions` appended, accepts an OpenAI-style POST).
- `ai.model` is **required** and treated as the local model identifier
  (e.g. `llama3.1:8b-instruct`, `qwen2.5:7b`,
  `Meta-Llama-3.1-8B-Instruct.Q4_K_M.gguf`).

```bash
# Ollama (default port 11434)
subx-cli config set ai.provider local
subx-cli config set ai.base_url "http://localhost:11434/v1"
subx-cli config set ai.model "llama3.1:8b-instruct"

# LM Studio
subx-cli config set ai.provider local
subx-cli config set ai.base_url "http://localhost:1234/v1"
subx-cli config set ai.model "Meta-Llama-3.1-8B-Instruct.Q4_K_M.gguf"

# llama.cpp `llama-server`
subx-cli config set ai.provider local
subx-cli config set ai.base_url "http://localhost:8080/v1"
subx-cli config set ai.model "qwen2.5-7b-instruct"

# vLLM (with optional shared token)
subx-cli config set ai.provider local
subx-cli config set ai.base_url "http://localhost:8000/v1"
subx-cli config set ai.model "Qwen/Qwen2.5-7B-Instruct"
subx-cli config set ai.api_key "<SHARED_TOKEN>"

# Alias: persisted as `local`
subx-cli config set ai.provider ollama
```

The `local` provider is **endpoint-agnostic**. It accepts any reachable URL,
including:

- Loopback: `http://localhost:11434/v1` or `http://127.0.0.1:11434/v1`
- LAN hosts: `http://192.168.1.50:11434/v1`
- VPN / tailnet hosts: `https://ollama.tailnet.ts.net/v1`
- Any other OpenAI-compatible endpoint reachable over HTTP or HTTPS

Both `http://` and `https://` schemes are valid for `local`. SubX still
emits an advisory warning when an API key would be transmitted over plain
HTTP to a non-loopback host (it never blocks the request).

#### Hosted-provider HTTPS rule

The hosted providers (`openai`, `openrouter`, `azure-openai`) **require
`https://`** for `ai.base_url`. Pointing them at `http://localhost:11434/v1`
or any non-HTTPS URL is rejected by configuration validation with an error
that names the offending field, the unsupported scheme, and recommends:

> If you intended to call an OpenAI-compatible local or LAN endpoint, set
> `ai.provider = "local"` (or `ollama`) and configure `ai.base_url` to your
> endpoint.

Default base URLs (`https://api.openai.com/v1`, etc.) are unaffected. The
same hint is appended at runtime if a hosted-provider request fails in a
way that suggests it was misdirected at a local endpoint.

#### Privacy posture

When `ai.provider = "local"`, SubX contacts **only** the configured
`base_url`. The hosted-provider environment variables `OPENAI_API_KEY`,
`OPENAI_BASE_URL`, `OPENROUTER_API_KEY`, and the `AZURE_OPENAI_*` family
are ignored entirely — they cannot silently switch the provider or inject
credentials. There is no telemetry, no analytics, and no fallback to hosted
endpoints.

The dedicated env vars `LOCAL_LLM_BASE_URL` and `LOCAL_LLM_API_KEY` are
honored only when the canonical `ai.provider` is `local`, with lower
precedence than `SUBX_AI_BASE_URL` / `SUBX_AI_APIKEY`. See
[Environment Variables](#environment-variables) below.

## Format Configuration (`[formats]`)

This section controls subtitle file format handling and encoding detection.

```toml
[formats]
default_output = "srt"                        # Default output format: srt, vtt, ass, lrc
preserve_styling = false                      # Preserve format-specific styling on conversion
default_encoding = "utf-8"                    # Default file encoding
encoding_detection_confidence = 0.8           # Encoding auto-detection confidence threshold (0.0–1.0)
```

## Sync Configuration (`[sync]`)

This section controls audio-subtitle synchronization. SubX supports two
methods: local Voice Activity Detection (VAD) for automated alignment, and
manual offset for direct time adjustment.

```toml
[sync]
default_method = "auto"                      # Sync method: auto, vad
max_offset_seconds = 60.0                    # Maximum allowed time offset in seconds
```

### VAD Configuration (`[sync.vad]`)

VAD performs on-device speech detection to calculate subtitle timing offsets.
All processing happens locally — no audio data leaves your machine.

```toml
[sync.vad]
enabled = true                               # Enable VAD-based sync
sensitivity = 0.25                           # Speech detection sensitivity (0.0–1.0)
padding_chunks = 3                           # Padding chunks around detected speech
min_speech_duration_ms = 300                 # Minimum speech segment duration in milliseconds
```

The `sensitivity` parameter controls the trade-off between detection
coverage and false positives. A higher value (e.g., 0.8) catches quieter
speech but may trigger on background noise. A lower value (e.g., 0.1)
requires clearer speech signals.

For audio with significant background noise, increase both `sensitivity`
and `padding_chunks`. For clean recordings with rapid speech, lower the
`min_speech_duration_ms` to avoid clipping short utterances.

## General Configuration (`[general]`)

This section controls overall application behavior.

```toml
[general]
backup_enabled = false                        # Create backup files before modifications
max_concurrent_jobs = 4                       # Maximum concurrent processing tasks
task_timeout_seconds = 300                    # Task execution timeout in seconds
workspace = "."                               # Working directory
enable_progress_bar = true                    # Show progress indicators
worker_idle_timeout_seconds = 60              # Worker thread idle timeout in seconds
```

## Translation Configuration (`[translation]`)

This section controls defaults for the `translate` command. Translation
reuses the configured AI provider in `[ai]` — there is no separate
translation service.

```toml
[translation]
batch_size = 40                               # Cues per AI translation request (1–1000)
default_target_language = ""                  # Optional default for --target-language (e.g., "zh-TW")
```

`batch_size` controls how many subtitle cues are sent to the AI provider in
a single translation request. Smaller batches are more resilient to
malformed responses (a failed batch only invalidates that batch's cues),
while larger batches reduce request count and cost. Configuration validation
rejects `0` and values that exceed the documented ceiling.

When `default_target_language` is set, the `translate` command may use it as
the default for `--target-language`; an explicit CLI flag always wins. Leave
the value empty to require `--target-language` on every invocation.

The terminology extraction pass and the per-cue translation pass both run
through the configured AI provider and inherit the `[ai]` retry, timeout,
and security settings.

## Parallel Processing Configuration (`[parallel]`)

This section controls the worker pool and task scheduling. The default
`max_workers` matches the CPU core count.

```toml
[parallel]
max_workers = 8                               # Maximum worker threads (default: CPU cores)
task_queue_size = 1000                        # Task queue capacity
enable_task_priorities = false                # Enable priority-based task ordering
auto_balance_workers = true                   # Automatically balance worker load
overflow_strategy = "Block"                   # Queue overflow: Block, DropOldest, Reject, Drop, Expand
```

The `overflow_strategy` determines what happens when the task queue is full.
`Block` waits for space (safest), `DropOldest` discards the oldest queued
task, `Reject` refuses the new task, `Drop` discards the new task silently,
and `Expand` grows the queue dynamically.

## Environment Variables

### Provider-Specific Variables

Each AI provider has dedicated environment variables. When set, these
automatically configure the provider and inject credentials.

```bash
# OpenAI
export OPENAI_API_KEY="<YOUR_API_KEY>"
export OPENAI_BASE_URL="https://api.openai.com/v1"

# OpenRouter
export OPENROUTER_API_KEY="<YOUR_API_KEY>"

# Azure OpenAI
export AZURE_OPENAI_API_KEY="<YOUR_API_KEY>"
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
export AZURE_OPENAI_DEPLOYMENT_ID="your-deployment-id"
export AZURE_OPENAI_API_VERSION="2025-04-01-preview"

# Local / OpenAI-compatible runtime (only honored when ai.provider = "local")
export LOCAL_LLM_BASE_URL="http://localhost:11434/v1"
export LOCAL_LLM_API_KEY="<OPTIONAL_SHARED_TOKEN>"
```

When the canonical `ai.provider` is `local`, SubX skips every hosted-provider
env var (`OPENAI_*`, `OPENROUTER_*`, `AZURE_OPENAI_*`) so they cannot
silently override your privacy choice. `LOCAL_LLM_*` env vars are likewise
ignored unless the canonical provider is `local`. `SUBX_AI_BASE_URL` and
`SUBX_AI_APIKEY` outrank `LOCAL_LLM_*` when both are set.

### General Overrides with `SUBX_` Prefix

Any configuration key can be overridden via environment variable by
converting the dotted key path to uppercase with underscores and adding the
`SUBX_` prefix. For example, `ai.api_key` becomes `SUBX_AI_API_KEY`, and
`parallel.max_workers` becomes `SUBX_PARALLEL_MAX_WORKERS`.

```bash
# AI settings
export SUBX_AI_PROVIDER=openai
export SUBX_AI_MODEL=gpt-4o-mini
export SUBX_AI_TEMPERATURE=0.5

# General settings
export SUBX_GENERAL_BACKUP_ENABLED=true

# Parallel processing
export SUBX_PARALLEL_MAX_WORKERS=16

# Format settings
export SUBX_FORMATS_DEFAULT_OUTPUT=vtt

# Sync VAD settings
export SUBX_SYNC_VAD_SENSITIVITY=0.8

# Translation settings
export SUBX_TRANSLATION_BATCH_SIZE=40
export SUBX_TRANSLATION_DEFAULT_TARGET_LANGUAGE=zh-TW

# Custom config file path
export SUBX_CONFIG_PATH="/custom/path/to/config.toml"

# Workspace override
export SUBX_WORKSPACE="/path/to/working/directory"
```

Note that provider-specific variables (like `OPENAI_API_KEY`) are checked
before `SUBX_` prefixed variables. The env-var handling has special cases in
`src/config/service.rs` — if a specific override does not take effect as
expected, check the implementation.

## Security Considerations

SubX handles API credentials and local media files. A few defaults and
recommendations help keep your setup safe.

### API Key Storage

Prefer environment variables over `config set` for API keys. Commands like
`subx-cli config set ai.api_key <key>` write the raw key as an argument to
your shell, which records it in history files such as `~/.bash_history` or
`~/.zsh_history`. Exporting `SUBX_AI_API_KEY` (or a provider-specific
variable like `OPENAI_API_KEY`) avoids this problem and keeps the key out of
persistent command history.

If you must place the key in the configuration file, SubX writes user
configuration files with mode `0600` and creates the config directory with
mode `0700` on Unix systems, so only your user account can read the file.

### File Permissions

When SubX creates or updates the config file, it sets restrictive
permissions automatically: `0600` on the file and `0700` on the containing
directory (Unix only). If you migrate the config from another machine or
edit it with an external tool, verify the permissions with `ls -l` and
tighten them with `chmod 600 ~/.config/subx/config.toml` if needed.

### Shell History

Any command that takes an API key on the command line — including
`subx-cli config set ai.api_key <key>` — will be stored in shell history by
default. Use one of these safer alternatives:

- Export the key as an environment variable in your shell profile.
- Pipe it in from a password manager, e.g.
  `SUBX_AI_API_KEY="$(pass show openai/api-key)" subx-cli match ...`.
- Use your shell's "do not save" prefix (a leading space in Bash with
  `HISTCONTROL=ignorespace`, for example).

### Size Limits

To guard against accidentally processing extremely large files, SubX
enforces two configurable ceilings under `[general]`:

- `max_subtitle_bytes` — maximum size of a subtitle file that will be
  loaded or written. Default: `52_428_800` (50 MiB).
- `max_audio_bytes` — maximum size of an audio file considered for sync
  and VAD processing. Default: `2_147_483_648` (2 GiB).

Files larger than these limits are rejected before any parsing or network
activity happens. Raise the values if you legitimately need to work with
larger media, but keep them in place as a safety net against typos and
runaway scripts.

## Troubleshooting

If `subx-cli config list` fails or shows unexpected values, start by
checking for conflicting environment variables with `env | grep -E
'SUBX_|OPENAI_|AZURE_OPENAI_|OPENROUTER_'`. Environment variables take
precedence over the config file and can silently override your settings.

For TOML syntax errors, `subx-cli config list` reports the parsing failure.
Fix the syntax in your config file, or run `subx-cli config reset` to
restore defaults.

If the config file cannot be written, verify write permissions on the
configuration directory and check available disk space.

Common error messages and their causes:

- **"Configuration validation failed"** — A value is outside its allowed
  range or format. Check the field constraints listed in each section above.
- **"Failed to build configuration"** — The config file has TOML syntax
  errors or is unreadable.
- **"Unable to determine config directory"** — The system cannot resolve the
  user config directory. Set `SUBX_CONFIG_PATH` explicitly.
- **"Unknown configuration key"** — The key name does not match any known
  configuration field.

## Complete Configuration Example

```toml
[ai]
provider = "openai"
model = "gpt-4.1-mini"
base_url = "https://api.openai.com/v1"
max_sample_length = 3000
temperature = 0.3
max_tokens = 10000
retry_attempts = 3
retry_delay_ms = 1000
request_timeout_seconds = 120

[formats]
default_output = "srt"
preserve_styling = false
default_encoding = "utf-8"
encoding_detection_confidence = 0.8

[sync]
default_method = "auto"
max_offset_seconds = 60.0

[sync.vad]
enabled = true
sensitivity = 0.25
padding_chunks = 3
min_speech_duration_ms = 300

[general]
backup_enabled = false
max_concurrent_jobs = 4
task_timeout_seconds = 300
workspace = "."
enable_progress_bar = true
worker_idle_timeout_seconds = 60

[parallel]
max_workers = 8
overflow_strategy = "Block"
task_queue_size = 1000
enable_task_priorities = false
auto_balance_workers = true

[translation]
batch_size = 40
default_target_language = ""
```
