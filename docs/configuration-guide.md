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
default_method = "whisper"           # 預設同步方法 ("whisper", "vad")
analysis_window_seconds = 30         # 分析時間窗口：第一句字幕前後的秒數 (u32)
max_offset_seconds = 60.0            # 最大允許的時間偏移量（秒） (f32)

# Whisper API configuration
[sync.whisper]
enabled = true                       # 是否啟用 Whisper API 方法 (bool)
model = "whisper-1"                  # Whisper 模型名稱 (String)
language = "auto"                    # 語言設定 ("auto" 為自動檢測) (String)
temperature = 0.0                    # API 溫度參數 (0.0-1.0) (f32)
timeout_seconds = 30                 # API 請求超時時間（秒） (u32)
max_retries = 3                      # 最大重試次數 (u32)
retry_delay_ms = 1000                # 重試間隔（毫秒） (u64)
fallback_to_vad = true               # 當 Whisper 失敗時是否回退到 VAD (bool)
min_confidence_threshold = 0.7       # 最低信心度閾值，低於此值回退到 VAD (f32)

# VAD configuration
[sync.vad]
enabled = true                       # 是否啟用本地 VAD 方法 (bool)
sensitivity = 0.75                   # 語音檢測敏感度 (0.0-1.0) (f32)
chunk_size = 512                     # 音訊塊大小（樣本數） (usize)
sample_rate = 16000                  # 處理採樣率（Hz） (u32)
padding_chunks = 3                   # 語音檢測前後填充塊數 (u32)
min_speech_duration_ms = 100         # 最小語音持續時間（毫秒） (u32)
speech_merge_gap_ms = 200            # 語音段合併間隔（毫秒） (u32)
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
export SUBX_SYNC_DEFAULT_METHOD=whisper
export SUBX_SYNC_ANALYSIS_WINDOW_SECONDS=45
export SUBX_SYNC_WHISPER_ENABLED=false
export SUBX_SYNC_WHISPER_MODEL=whisper-1
export SUBX_SYNC_VAD_SENSITIVITY=0.8
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
default_method = "whisper"
analysis_window_seconds = 30
max_offset_seconds = 60.0

[sync.whisper]
enabled = true
model = "whisper-1"
language = "auto"
temperature = 0.0
timeout_seconds = 30
max_retries = 3
retry_delay_ms = 1000
fallback_to_vad = true
min_confidence_threshold = 0.7

[sync.vad]
enabled = true
sensitivity = 0.75
chunk_size = 512
sample_rate = 16000
padding_chunks = 3
min_speech_duration_ms = 100
speech_merge_gap_ms = 200

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
