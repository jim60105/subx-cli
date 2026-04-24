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
subx-cli config set ai.api_key "sk-your-api-key-here"

# Reset everything to defaults
subx-cli config reset
```

## AI Configuration (`[ai]`)

This section controls AI provider selection and request behavior.

```toml
[ai]
provider = "openai"                            # openai, openrouter, or azure-openai
api_key = "sk-your-api-key-here"              # API key (Option<String>)
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
api_key = "your-openrouter-api-key"
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
api_key = "your-azure-api-key"
model = "your-deployment-id"
base_url = "https://your-resource.openai.azure.com"
api_version = "2025-04-01-preview"
```

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
export OPENAI_API_KEY="sk-your-api-key-here"
export OPENAI_BASE_URL="https://api.openai.com/v1"

# OpenRouter
export OPENROUTER_API_KEY="your-openrouter-api-key"

# Azure OpenAI
export AZURE_OPENAI_API_KEY="your-azure-api-key"
export AZURE_OPENAI_ENDPOINT="https://your-resource.openai.azure.com"
export AZURE_OPENAI_DEPLOYMENT_ID="your-deployment-id"
export AZURE_OPENAI_API_VERSION="2025-04-01-preview"
```

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

# Custom config file path
export SUBX_CONFIG_PATH="/custom/path/to/config.toml"

# Workspace override
export SUBX_WORKSPACE="/path/to/working/directory"
```

Note that provider-specific variables (like `OPENAI_API_KEY`) are checked
before `SUBX_` prefixed variables. The env-var handling has special cases in
`src/config/service.rs` — if a specific override does not take effect as
expected, check the implementation.

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
```
