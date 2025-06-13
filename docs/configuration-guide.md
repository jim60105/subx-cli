# SubX Configuration Guide

This guide explains the configuration options for the SubX subtitle utility, helping you customize the application behavior according to your needs.

## View Configuration

### Quick Configuration Check

```bash
# View all configuration settings
subx config list

# View specific configuration item
subx config get ai.provider

# View configuration file path
subx config list --show-path
```

### Set Configuration Items

```bash
# Set AI provider
subx config set ai.provider openai

# Set API key
subx config set ai.api_key "sk-your-api-key-here"

# Reset to default values
subx config reset
```

## Configuration Overview

SubX uses a layered configuration system supporting multiple configuration sources in priority order:

1. **Environment Variables** (highest priority)
2. **User Configuration File** (`~/.config/subx/config.toml`)
3. **Default Configuration** (lowest priority)

## AI Configuration (`[ai]`)

Controls AI-related functionality settings.

```toml
[ai]
provider = "openai"                            # AI provider: openai, anthropic, local
api_key = "sk-your-api-key-here"              # API key (Option<String>)
model = "gpt-4o-mini"                         # AI model to use (String)
base_url = "https://api.openai.com/v1"        # API endpoint URL (String)
max_sample_length = 3000                      # Maximum content length sent to AI (usize, 100-10000)
temperature = 0.3                             # Response randomness control (f32, 0.0-2.0)
retry_attempts = 3                            # API request retry count (u32, 1-10)
retry_delay_ms = 1000                         # Retry delay in milliseconds (u64, 100-10000)
```

## Format Configuration (`[formats]`)

Controls file format processing options.

```toml
[formats]
default_output = "srt"                        # Default output format: srt, vtt, ass, lrc (String)
preserve_styling = false                      # Whether to preserve format styling (bool)
default_encoding = "utf-8"                    # Default file encoding (String)
encoding_detection_confidence = 0.8           # Encoding detection confidence threshold (f32, 0.0-1.0)
```

## Sync Configuration (`[sync]`)

Controls audio-subtitle synchronization functionality.

```toml
[sync]
max_offset_seconds = 10.0                     # Maximum offset range in seconds (f32)
correlation_threshold = 0.8                   # Correlation analysis threshold (f32, 0.0-1.0)
dialogue_detection_threshold = 0.6            # Dialogue detection threshold (f32, 0.0-1.0)
min_dialogue_duration_ms = 500                # Minimum dialogue duration in milliseconds (u32)
dialogue_merge_gap_ms = 200                   # Dialogue merge gap in milliseconds (u32)
enable_dialogue_detection = true              # Whether to enable dialogue detection (bool)
audio_sample_rate = 44100                     # Audio sample rate in Hz (u32)
auto_detect_sample_rate = true                # Whether to auto-detect sample rate (bool)
```

## General Configuration (`[general]`)

Controls general application behavior.

```toml
[general]
backup_enabled = false                        # Whether to enable file backup (bool)
max_concurrent_jobs = 4                       # Maximum concurrent tasks (usize)
task_timeout_seconds = 300                    # Task execution timeout in seconds (u64)
enable_progress_bar = true                    # Whether to show progress bar (bool)
worker_idle_timeout_seconds = 60              # Worker thread idle timeout in seconds (u64)
```

## Parallel Processing Configuration (`[parallel]`)

Controls parallel processing behavior.

```toml
[parallel]
max_workers = 8                               # Maximum worker threads (usize, default: CPU cores)
task_queue_size = 1000                        # Task queue size (usize)
enable_task_priorities = false                # Whether to enable task priorities (bool)
auto_balance_workers = true                   # Whether to auto-balance load (bool)
overflow_strategy = "block"                   # Queue overflow strategy: block, drop, expand (String)
```

## Environment Variable Support

### Special AI Configuration Environment Variables

SubX supports the following OpenAI environment variables for compatibility:

```bash
export OPENAI_API_KEY="sk-your-api-key-here"
export OPENAI_BASE_URL="https://api.openai.com/v1"
```

### Universal SUBX_ Prefix Override

Any configuration item can be overridden using environment variables with the `SUBX_` prefix. The configuration system automatically loads these variables and applies them with the highest priority.

#### Environment Variable Examples

```bash
# Override file backup setting
export SUBX_GENERAL_BACKUP_ENABLED=true

# Override AI-related configuration
export SUBX_AI_PROVIDER=openai
export SUBX_AI_MODEL=gpt-4o-mini
export SUBX_AI_TEMPERATURE=0.5

# Override parallel processing configuration
export SUBX_PARALLEL_MAX_WORKERS=16
export SUBX_PARALLEL_TASK_QUEUE_SIZE=2000

# Override configuration file path
export SUBX_CONFIG_PATH="/custom/path/to/config.toml"
```

#### Environment Variable Naming Rules

The environment variable naming follows these rules:
- Use `SUBX_` prefix
- Convert nested configuration paths to uppercase with underscores
- Examples:
  - `ai.api_key` → `SUBX_AI_API_KEY`
  - `general.backup_enabled` → `SUBX_GENERAL_BACKUP_ENABLED`
  - `parallel.max_workers` → `SUBX_PARALLEL_MAX_WORKERS`

#### Additional Environment Variable Examples

```bash
# Format configuration
export SUBX_FORMATS_DEFAULT_OUTPUT=vtt
export SUBX_FORMATS_PRESERVE_STYLING=true
export SUBX_FORMATS_DEFAULT_ENCODING=utf-16

# Sync configuration  
export SUBX_SYNC_MAX_OFFSET_SECONDS=15.0
export SUBX_SYNC_CORRELATION_THRESHOLD=0.9
export SUBX_SYNC_ENABLE_DIALOGUE_DETECTION=false
```

## Configuration File Locations

- **Linux/macOS**: `~/.config/subx/config.toml`
- **Windows**: `%APPDATA%\subx\config.toml`
- **Custom Path**: Specify via `SUBX_CONFIG_PATH` environment variable

## Complete Configuration File Example

```toml
[ai]
provider = "openai"
api_key = "sk-your-api-key-here"
model = "gpt-4.1-mini"
base_url = "https://api.openai.com/v1"
max_sample_length = 3000
temperature = 0.3
retry_attempts = 3
retry_delay_ms = 1000

[formats]
default_output = "srt"
preserve_styling = false
default_encoding = "utf-8"
encoding_detection_confidence = 0.8

[sync]
max_offset_seconds = 10.0
correlation_threshold = 0.8
dialogue_detection_threshold = 0.6
min_dialogue_duration_ms = 500
dialogue_merge_gap_ms = 200
enable_dialogue_detection = true
audio_sample_rate = 44100
auto_detect_sample_rate = true

[general]
backup_enabled = false
max_concurrent_jobs = 4
task_timeout_seconds = 300
enable_progress_bar = true
worker_idle_timeout_seconds = 60

[parallel]
max_workers = 8
task_queue_size = 1000
enable_task_priorities = false
auto_balance_workers = true
overflow_strategy = "block"
```

## Error Messages

When configuration issues occur, you may encounter these errors:

- **"Configuration validation failed"**: Configuration values don't meet required format or range
- **"Failed to build configuration"**: Configuration file has format errors or cannot be read
- **"Unable to determine config directory"**: Cannot determine configuration directory location
- **"Unknown configuration key"**: Used a non-existent configuration key

## Troubleshooting

### Configuration Loading Issues

1. **Check configuration file syntax**:
   ```bash
   # Check if TOML syntax is correct
   subx config list
   ```

2. **Check environment variables**:
   ```bash
   # Check for conflicting environment variables
   env | grep SUBX_
   env | grep OPENAI_
   ```

3. **Reset configuration**:
   ```bash
   # Reset to default values
   subx config reset
   ```

### Permission Issues

If the configuration file cannot be written, check:
- Write permissions for the configuration directory
- Available disk space
- Whether antivirus software is blocking file writes
