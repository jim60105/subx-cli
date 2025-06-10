# SubX-CLI

<div align="center">
  <img src="assets/logo.svg" alt="SubX CLI Logo" width="800" height="300">

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli) [![codecov](https://codecov.io/gh/jim60105/subx-cli/graph/badge.svg?token=2C53RSNNAL)](https://codecov.io/gh/jim60105/subx-cli)

English | [中文](./README.zh-TW.md)

An subtitle processing CLI tool that powered by AI technology to automatically match, rename, and process subtitle files.

</div>

> [!NOTE]  
> This project is currently in early development. Basic functionality has been implemented, but there may still be room for improvement. If you find any issues please feel free to submit an Issue.

## Features

- 🤖 **AI Smart Matching** - Uses AI technology to automatically identify video-subtitle correspondence and rename files
- 🔄 **Format Conversion** - Supports conversion between mainstream subtitle formats like SRT, ASS, VTT, SUB
- ⏰ **Timeline Correction** - Automatically detects and corrects subtitle timing offset issues
- 🏃 **Batch Processing** - Process entire folders of media files at once
- 🔍 **Dry-run Mode** - Preview operation results for safety and reliability
- 📦 **Cache Management** - Reuse previous analysis results for repeated dry-runs to improve efficiency

## Installation

### Linux

#### Method 1: Download and run installation script
```bash
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/master/scripts/install.sh | bash
```

#### Method 2: Direct download of pre-compiled binaries
```bash
# Download latest version
curl -L "https://github.com/jim60105/subx-cli/releases/latest/download/subx-linux-x86_64" -o subx-cli
chmod +x subx-cli
sudo mv subx-cli /usr/local/bin/
```

#### Method 3: Install using Cargo compilation
```bash
# Install from crates.io
cargo install subx-cli

# Or compile from source
git clone https://github.com/jim60105/subx-cli.git
cd subx-cli
cargo build --release
sudo cp target/release/subx-cli /usr/local/bin/
```

## Quick Start

### 1. Configure API Keys
```bash
# Set OpenAI API Key (for AI matching functionality)
export OPENAI_API_KEY="your-api-key-here"

# Optional: Set custom OpenAI Base URL (for OpenAI API or private deployment)
export OPENAI_BASE_URL="https://api.openai.com/v1"

# Or set through configuration commands
subx-cli config set ai.api_key "your-api-key-here"
subx-cli config set ai.base_url "https://api.openai.com/v1"
subx-cli config set ai.model "gpt-4o-mini"
subx-cli config set general.backup_enabled true
```

### 2. Basic Usage

**Subtitle Matching and Renaming**
```bash
# Process a single folder
subx-cli match /path/to/media/folder

# Preview mode (no actual execution)
subx-cli match --dry-run /path/to/media/folder

# Recursively process subfolders
subx-cli match --recursive /path/to/media/folder
```

**Format Conversion**
```bash
# Convert single file
subx-cli convert subtitle.ass --format srt

# Batch conversion
subx-cli convert --format srt /path/to/subtitles/

# Convert while keeping original file
subx-cli convert --keep-original subtitle.vtt --format srt
```

**Timeline Correction**
```bash
# Auto-detect offset
subx-cli sync video.mp4 subtitle.srt

# Manually specify offset
subx-cli sync --offset -2.5 video.mp4 subtitle.srt

# Batch sync entire folder
subx-cli sync --batch /path/to/media/folder
```

**Cache Management**
```bash
# Clear dry-run cache
subx-cli cache clear
```

## Usage Examples

### Typical Workflow
```bash
# 1. Process downloaded videos and subtitles
cd ~/Downloads/TV_Show_S01/

# 2. AI match and rename subtitles
subx-cli match --dry-run .  # Preview first
subx-cli match .            # Execute after confirmation

# 3. Unify conversion to SRT format
subx-cli convert --format srt .

# 4. Fix time synchronization issues
subx-cli sync --batch .
```

### Folder Structure Example
```
Before processing:
TV_Show_S01/
├── S01E01.mkv
├── S01E02.mkv
├── subtitle_from_internet_1.ass
└── subtitle_from_internet_2.ass

After processing:
TV_Show_S01/
├── S01E01.mkv
├── S01E01.ass          # Matched and renamed
├── S01E02.mkv
└── S01E02.ass          # Matched and renamed
```

## Configuration Options

### Configuration File Location
- Linux/macOS: `~/.config/subx/config.toml`
- Windows: `%APPDATA%\subx\config.toml`

### Configuration Example
```toml
[ai]
# AI service provider, currently supports "openai"
provider = "openai"
# AI model to use
model = "gpt-4o-mini"
# API endpoint, can be overridden by OPENAI_BASE_URL environment variable
base_url = "https://api.openai.com/v1"
# API key, can be overridden by OPENAI_API_KEY environment variable
api_key = "your-api-key-here"
# AI response randomness control (0.0-2.0)
temperature = 0.3
# Maximum content length sent to AI
max_sample_length = 2000
# Number of retry attempts for API request failures
retry_attempts = 3
# Retry interval (milliseconds)
retry_delay_ms = 1000

[formats]
# Default output format
default_output = "srt"
# Whether to preserve styling during conversion
preserve_styling = true
# Default text encoding
default_encoding = "utf-8"
# Encoding detection confidence threshold (0.0-1.0)
encoding_detection_confidence = 0.7

[sync]
# Maximum offset range for audio sync (seconds)
max_offset_seconds = 30.0
# Audio correlation analysis threshold (0.0-1.0)
correlation_threshold = 0.7
# Audio energy threshold for dialogue detection
dialogue_detection_threshold = 0.01
# Minimum dialogue segment duration (milliseconds)
min_dialogue_duration_ms = 500
# Dialogue segment merge gap (milliseconds)
dialogue_merge_gap_ms = 500
# Whether to enable dialogue detection feature
enable_dialogue_detection = true
# Audio processing sample rate (Hz)
audio_sample_rate = 16000
# Whether to auto-detect audio sample rate
auto_detect_sample_rate = true

[general]
# Whether to enable file backup, can be overridden by SUBX_BACKUP_ENABLED environment variable
backup_enabled = false
# Maximum number of concurrent jobs
max_concurrent_jobs = 4
# Task execution timeout (seconds)
task_timeout_seconds = 3600
# Whether to display progress bar
enable_progress_bar = true
# Worker thread idle timeout (seconds)
worker_idle_timeout_seconds = 300

[parallel]
# Task queue size limit
task_queue_size = 100
# Whether to enable task priority scheduling
enable_task_priorities = true
# Whether to enable automatic load balancing
auto_balance_workers = true
# Queue overflow strategy ("block" | "drop_oldest" | "reject")
queue_overflow_strategy = "block"
```

## Command Reference

### `subx-cli match` - AI Matching and Renaming
```
Options:
  <PATH>                Target folder path
  --dry-run             Preview mode, no actual execution
  --confidence <NUM>    Minimum confidence threshold (0-100, default: 80)
  --recursive           Recursively process subfolders
  --backup              Backup original files before renaming

Configuration Support:
  - AI Settings: Support custom API endpoints, models, temperature, etc.
  - Parallel Processing: Support max concurrency, task queue size, priority scheduling, etc.
  - General Settings: Support backup, progress bar, timeout control, etc.
```

### `subx-cli convert` - Format Conversion
```
Options:
  <INPUT>               Input file or folder path
  --format <FORMAT>     Target format (srt|ass|vtt|sub)
  --output, -o <FILE>   Output filename
  --keep-original       Keep original file
  --encoding <ENC>      Specify text encoding (default: utf-8)

Configuration Support:
  - Format Settings: Default output format, style preservation, encoding detection, etc.
```

### `subx-cli detect-encoding` - File Encoding Detection
```
Options:
  <FILES>...             Target file paths
  -v, --verbose          Show detailed sample text

Configuration Support:
  - Format Settings: Encoding detection confidence threshold, default encoding, etc.
```

### `subx-cli sync` - Timeline Correction
```
Options:
  <VIDEO>               Video file path
  <SUBTITLE>            Subtitle file path
  --offset <SECONDS>    Manually specify offset
  --batch               Batch processing mode
  --range <SECONDS>     Offset detection range (default: max_offset_seconds from config)
  --threshold <THRESHOLD>  Correlation threshold (0-1, default: correlation_threshold from config)

Configuration Support:
  - Sync Settings: Maximum offset range, correlation threshold, dialogue detection, etc.
  - Audio Processing: Sample rate, dialogue detection threshold, segment merging, etc.
```

### `subx-cli config` - Configuration Management
```
Usage:
  subx-cli config set <KEY> <VALUE>   Set configuration value
  subx-cli config get <KEY>           Get configuration value
  subx-cli config list                List all configurations
  subx-cli config reset               Reset configuration
```

### `subx-cli cache` - Dry-run Cache Management
```
Options:
  clear                 Clear all dry-run cache files
```

### `subx-cli generate-completion` - Generate Shell Completion Scripts
```
Usage:
  subx-cli generate-completion <SHELL>  Supported shells: bash, zsh, fish, powershell, elvish
```

## Supported Formats

| Format | Read | Write | Description |
|--------|------|-------|-------------|
| SRT    | ✅   | ✅    | SubRip subtitle format |
| ASS    | ✅   | ✅    | Advanced SubStation Alpha format |
| VTT    | ✅   | ✅    | WebVTT format |
| SUB    | ✅   | ⚠️    | Various SUB variant formats |

## Troubleshooting

### Common Issues

**Q: What to do if AI matching accuracy is low?**
A: Ensure filenames contain sufficient identifying information (like show name, season, episode numbers). You can also try adjusting the `--confidence` parameter or configure AI model temperature: `subx-cli config set ai.temperature 0.1`

**Q: Timeline sync fails?**
A: Ensure video files are accessible and check if file formats are supported. If automatic sync isn't ideal, you can try:
- Manually specify offset: `subx-cli sync --offset <seconds> video.mp4 subtitle.srt`
- Adjust sync configuration: `subx-cli config set sync.correlation_threshold 0.6`
- Enable dialogue detection: `subx-cli config set sync.enable_dialogue_detection true`

**Q: Poor performance when processing large numbers of files?**
A: You can adjust parallel processing configuration:
```bash
subx-cli config set general.max_concurrent_jobs 8     # Increase concurrency
subx-cli config set parallel.task_queue_size 200     # Increase queue size
subx-cli config set parallel.auto_balance_workers true # Enable load balancing
```

**Q: Inaccurate encoding detection?**
A: Adjust detection confidence threshold: `subx-cli config set formats.encoding_detection_confidence 0.8`

**Q: Cache files taking up too much space?**
A: Use the `subx-cli cache clear` command to clear all cache files.

**Q: How to re-match when new videos and subtitles are added?**
A: Clear cache first with `subx-cli cache clear`, then re-run the match command.

**Q: What to do about task execution timeouts?**
A: Increase timeout duration: `subx-cli config set general.task_timeout_seconds 7200`  # Set to 2 hours

---

> [!NOTE]  
> This project is fully developed using GitHub Copilot and Codex CLI, with an attempt to maintain the maintainability of the software architecture. My goal is to practice controlling and planning professional software engineering work entirely through prompt engineering with AI. This is what professional vibe coding should be.
