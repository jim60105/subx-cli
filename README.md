# SubX-CLI

<div align="center">
  <img src="assets/logo.svg" alt="SubX CLI Logo" width="800" height="300">

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli) [![codecov](https://codecov.io/gh/jim60105/subx-cli/graph/badge.svg?token=2C53RSNNAL)](https://codecov.io/gh/jim60105/subx-cli)

English | [中文](./README.zh-TW.md)

AI subtitle processing CLI tool, which automatically matches, renames, and converts subtitle files.

</div>

## Features

AI subtitle processing CLI tool, which automatically matches, renames, and converts subtitle files.

</div>

## Features

- 🤖 **AI Smart Matching** - Uses AI technology to automatically identify video-subtitle correspondence and rename files
- 📁 **File Organization** - Automatically copy or move matched subtitle files to video folders for seamless playback
- 🔄 **Format Conversion** - Supports conversion between mainstream subtitle formats like SRT, ASS, VTT, SUB
- 🔊 **Audio Synchronization** - Directly decode various audio container formats (MP4, MKV, WebM, OGG, WAV) for VAD-based synchronization without intermediate transcoding
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

### 1. Configure Settings
```bash
# Set OpenAI API Key (for AI matching functionality only)
export OPENAI_API_KEY="your-api-key-here"

# Configure VAD settings
subx-cli config set sync.vad.sensitivity 0.8
subx-cli config set sync.vad.enabled true

# Enable general backup feature
subx-cli config set general.backup_enabled true

# Configure parallel processing
subx-cli config set parallel.max_workers 8
subx-cli config set parallel.task_queue_size 1000
```

### 2. Basic Usage

**Subtitle Matching and Renaming**
```bash
# Process a single folder
subx-cli match /path/to/media/folder

# Process multiple input sources using -i parameter
subx-cli match -i /path/to/videos -i /path/to/more/media

# Preview mode (no actual execution)
subx-cli match --dry-run /path/to/media/folder

# Recursively process subfolders
subx-cli match --recursive /path/to/media/folder

# Combine -i parameter with recursive processing
subx-cli match -i /path/to/videos -i /path/to/movies --recursive

# Copy matched subtitles to video folders
subx-cli match --copy /path/to/media/folder

# Move matched subtitles to video folders  
subx-cli match --move /path/to/media/folder

# Advanced: Mix files and directories with multiple options
subx-cli match -i ./video1.mp4 -i ./subtitles_dir -i ./video2.mkv --recursive --copy --backup

# Combine with recursive and backup options
subx-cli match --recursive --move --backup /path/to/media/folder
```

**Format Conversion**
```bash
# Convert single file
subx-cli convert subtitle.ass --format srt

# Batch conversion using -i parameter for multiple directories
subx-cli convert -i ./srt_files -i ./more_subtitles --format vtt

# Batch conversion with recursive directory scanning
subx-cli convert -i ./srt_files -i ./more_subtitles --format vtt --recursive

# Batch conversion
subx-cli convert --format srt /path/to/subtitles/

# Convert while keeping original file
subx-cli convert --keep-original subtitle.vtt --format srt

# Advanced: Mix files and directories with encoding specification
subx-cli convert -i movie1.srt -i ./batch_dir -i movie2.ass --format srt --recursive --keep-original --encoding utf-8
```

**Timeline Correction**

```bash
# Automatic VAD synchronization (requires audio/video file)
subx-cli sync video.mp4 subtitle.srt

# Manual synchronization (only subtitle file required)  
subx-cli sync --offset 2.5 subtitle.srt

# VAD with custom sensitivity
subx-cli sync --vad-sensitivity 0.8 video.mp4 subtitle.srt

# Batch processing mode (processes entire directories)
subx-cli sync --batch /path/to/media/folder

# Batch processing using -i parameter for multiple directories
subx-cli sync -i ./movies_directory --batch

# Batch processing with recursive directory scanning  
subx-cli sync -i ./movies_directory --batch --recursive

# Advanced: Multiple directories with specific sync method
subx-cli sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive --batch --method vad

# Batch mode with detailed output and dry-run
subx-cli sync -i ./media --batch --recursive --dry-run --verbose
subx-cli sync movie.mkv
subx-cli sync -b media_folder
```

**Character Encoding Detection**
```bash
# Specify files directly
subx-cli detect-encoding *.srt

# Using -i parameter for directory processing (flat)
subx-cli detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose

# Using -i parameter with recursive directory scanning
subx-cli detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose --recursive

# Advanced: Mix specific files with directory scanning
subx-cli detect-encoding -i ./more_subtitles -i specific_file.srt --recursive --verbose
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

# 2. AI match and rename subtitles with file organization
subx-cli match --dry-run --copy .  # Preview first
subx-cli match --copy .            # Execute after confirmation

# 3. Unify conversion to SRT format
subx-cli convert --format srt .

# 4. Fix time synchronization issues
subx-cli sync --batch .
```

### Advanced Workflow with -i Parameter
```bash
# 1. Process multiple directories with different sources
cd ~/Media/

# 2. Match and organize from multiple input sources
subx-cli match -i ./Downloads/Movies -i ./Downloads/TV_Shows -i ./Backup/Subs --recursive --dry-run --copy
subx-cli match -i ./Downloads/Movies -i ./Downloads/TV_Shows -i ./Backup/Subs --recursive --copy

# 3. Batch convert all subtitle formats to SRT with recursive scanning
subx-cli convert -i ./Movies -i ./TV_Shows --format srt --recursive --keep-original

# 4. Batch synchronize all media with multiple directories
subx-cli sync -i ./Movies -i ./TV_Shows --batch --recursive --method vad

# 5. Check encoding of all subtitle files
subx-cli detect-encoding -i ./Movies -i ./TV_Shows --recursive --verbose
```

### File Organization Scenarios
```bash
# Scenario 1: Keep original subtitles in place, copy to video folders
subx-cli match --recursive --copy /media/collection/

# Scenario 1b: Multiple input sources with copy operation
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --copy

# Scenario 2: Move subtitles to video folders, clean up original locations
subx-cli match --recursive --move /media/collection/

# Scenario 2b: Multiple input sources with move operation
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --move

# Scenario 3: Preview file organization operations
subx-cli match --dry-run --copy --recursive /media/collection/

# Scenario 3b: Preview with multiple input sources
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --dry-run --copy

# Scenario 4: Organize files with backup protection
subx-cli match --move --backup --recursive /media/collection/

# Scenario 4b: Multiple sources with backup protection
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --move --backup

# Scenario 5: Advanced - Mix specific files with directories
subx-cli match -i ./video1.mp4 -i ./subtitles_dir -i ./video2.mkv --recursive --copy --backup
```

### Folder Structure Example
```
Before processing (distributed structure):
media/
├── movies/
│   ├── Action/
│   │   └── The.Matrix.1999.1080p.BluRay.mkv
│   └── Drama/
│       └── Forrest.Gump.1994.720p.WEB-DL.mp4
└── subtitles/
    ├── english/
    │   ├── Matrix_EN_Sub.srt
    │   └── ForrestGump_English.srt
    └── chinese/
        ├── 駭客任務_中文字幕.srt
        └── 阿甘正傳.繁中.srt

After processing with --copy option (AI Smart Matching):
media/
├── movies/
│   ├── Action/
│   │   ├── The.Matrix.1999.1080p.BluRay.mkv
│   │   ├── The.Matrix.1999.1080p.BluRay.srt           # AI matched Matrix_EN_Sub.srt and renamed
│   │   └── The.Matrix.1999.1080p.BluRay.zh.srt        # AI matched 駭客任務_中文字幕.srt and renamed
│   └── Drama/
│       ├── Forrest.Gump.1994.720p.WEB-DL.mp4
│       ├── Forrest.Gump.1994.720p.WEB-DL.srt           # AI matched ForrestGump_English.srt and renamed
│       └── Forrest.Gump.1994.720p.WEB-DL.zh.srt        # AI matched 阿甘正傳.繁中.srt and renamed
└── subtitles/                   # Original files preserved
    ├── english/
    │   ├── Matrix_EN_Sub.srt
    │   └── ForrestGump_English.srt
    └── chinese/
        ├── 駭客任務_中文字幕.srt
        └── 阿甘正傳.繁中.srt

After processing with --move option (AI Smart Matching):
media/
├── movies/
│   ├── Action/
│   │   ├── The.Matrix.1999.1080p.BluRay.mkv
│   │   ├── The.Matrix.1999.1080p.BluRay.srt           # AI matched and renamed, then moved
│   │   └── The.Matrix.1999.1080p.BluRay.zh.srt        # AI matched and renamed, then moved
│   └── Drama/
│       ├── Forrest.Gump.1994.720p.WEB-DL.mp4
│       ├── Forrest.Gump.1994.720p.WEB-DL.srt           # AI matched and renamed, then moved
│       └── Forrest.Gump.1994.720p.WEB-DL.zh.srt        # AI matched and renamed, then moved
└── subtitles/                   # Original files removed
    ├── english/                 # Empty directories
    └── chinese/
```

## Configuration

SubX supports comprehensive configuration through environment variables and configuration files.

### Quick Configuration
```bash
# Set OpenAI API Key
export OPENAI_API_KEY="your-api-key-here"

# Optional: Custom OpenAI endpoint
export OPENAI_BASE_URL="https://api.openai.com/v1"

# Or use config commands
subx-cli config set ai.api_key "your-api-key-here"
subx-cli config set ai.model "gpt-4.1-mini"
subx-cli config set ai.base_url "https://api.openai.com/v1"
subx-cli config set ai.temperature 0.3
subx-cli config set ai.retry_attempts 3
```

### Configuration File Location
- Linux/macOS: `~/.config/subx/config.toml`
- Windows: `%APPDATA%\subx\config.toml`

For detailed configuration options, see [Configuration Guide](docs/configuration-guide.md).

## Command Reference

### `subx-cli match` - AI Matching and Renaming
```
Options:
  <PATH>                Target folder path
  --dry-run             Preview mode, no actual execution
  --confidence <NUM>    Minimum confidence threshold (0-100, default: 80)
  --recursive           Recursively process subfolders
  --backup              Backup original files before renaming
  --copy, -c            Copy matched subtitle files to video folders
  --move, -m            Move matched subtitle files to video folders

File Organization:
  The --copy and --move options enable automatic file organization for better
  media player compatibility. When subtitles are matched with videos in different
  directories, these options will copy or move the subtitle files to the same
  folder as their corresponding video files.
  
  - --copy: Preserves original subtitle files in their original locations
  - --move: Removes original subtitle files after moving them
  - These options are mutually exclusive and cannot be used together
  - Only applied when subtitle and video files are in different directories
  - Includes automatic filename conflict resolution with backup support

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
  - Format Settings: Default output format, style preservation, encoding detection confidence, default encoding, etc.
```

### `subx-cli detect-encoding` - File Encoding Detection
```
Options:
  <FILES>...             Target file paths
  -v, --verbose          Show detailed sample text

Configuration Support:
  - Format Settings: Encoding detection confidence threshold, default encoding fallback, etc.
```

### `subx-cli sync` - Timeline Correction
```
Options:
  <VIDEO>               Video file path (supports MP4, MKV/WebM, OGG, WAV audio input)
  <SUBTITLE>            Subtitle file path
  <PATHS>...            Files or directories to process (positional)
  --offset <SECONDS>    Manually specify offset (must not exceed sync.max_offset_seconds config)
  --batch               Batch processing mode
  --method <METHOD>     Sync method (auto|vad, default: from sync.default_method config)
  --vad-sensitivity <SENSITIVITY>    VAD detection sensitivity (0.0-1.0, overrides config)
  --vad-chunk-size <SIZE>           VAD chunk size (overrides config)

Audio Format Support:
  - MP4, MKV/WebM, OGG, WAV containers (automatically transcoded to WAV for analysis)

Configuration Support:
  - Sync Settings: Default sync method, maximum offset range, etc.
  - VAD Processing: Sensitivity, chunk size, sample rate, padding chunks, min speech duration, speech merge gap, etc.
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

### Q: What to do if AI matching accuracy is low?

A: Ensure filenames contain sufficient identifying information (like show name, season, episode numbers). You can also try adjusting the `--confidence` parameter or configure AI model temperature: `subx-cli config set ai.temperature 0.1`

### Q: Timeline sync fails?

A: Ensure the audio/video file is accessible and check if the file format is supported. If VAD sync doesn't work well, try:
- Adjust VAD sensitivity: `subx-cli config set sync.vad.sensitivity 0.8` (higher for quiet audio)
- Use manual offset for difficult cases: `subx-cli sync --offset <seconds> subtitle.srt`
- Check VAD configuration: `subx-cli config set sync.vad.enabled true`
- For very noisy audio: `subx-cli config set sync.vad.min_speech_duration_ms 200`
- For rapid speech: `subx-cli config set sync.vad.speech_merge_gap_ms 100`
- Adjust audio processing parameters:
  - `subx-cli config set sync.vad.chunk_size 512`
  - `subx-cli config set sync.vad.sample_rate 16000`
  - `subx-cli config set sync.vad.padding_chunks 3`

### Q: Poor performance when processing large numbers of files?

A: You can adjust parallel processing configuration:
```bash
subx-cli config set general.max_concurrent_jobs 8     # Increase concurrency
subx-cli config set parallel.task_queue_size 2000    # Increase queue size
subx-cli config set parallel.auto_balance_workers true # Enable load balancing
subx-cli config set parallel.enable_task_priorities true # Enable task priorities
subx-cli config set parallel.max_workers 16          # Increase max workers
```

### Q: Inaccurate encoding detection?

A: Adjust detection confidence threshold and default encoding:
```bash
subx-cli config set formats.encoding_detection_confidence 0.8
subx-cli config set formats.default_encoding "utf-8"
```

### Q: Format conversion issues or styling problems?

A: Configure format conversion settings:
```bash
subx-cli config set formats.default_output "srt"      # Set default output format
subx-cli config set formats.preserve_styling true     # Preserve styling during conversion
```

### Q: Cache files taking up too much space?

A: Use the `subx-cli cache clear` command to clear all cache files.

### Q: How to re-match when new videos and subtitles are added?

A: Clear cache first with `subx-cli cache clear`, then re-run the match command.

### Q: What to do about task execution timeouts?

A: Increase timeout duration: `subx-cli config set general.task_timeout_seconds 7200`  # Set to 2 hours

### Q: File organization (copy/move) operations fail?
A: Check the following common issues:
- Ensure target video directories have write permissions
- Check if there's sufficient disk space for copy operations
- For filename conflicts, the system will automatically rename files with numeric suffixes
- Use `--dry-run` to preview operations before execution: `subx-cli match --dry-run --copy /path`

### Q: Can I use both --copy and --move together?

A: No, these options are mutually exclusive. Choose either `--copy` to preserve original files or `--move` to clean up original locations.

### Q: Why are some subtitles not being copied/moved to video folders?

A: The copy/move operations only apply when:
- Subtitle and video files are in different directories
- AI matching confidence exceeds the threshold (default 80%)
- Files don't already exist in the target location with identical names
Use `--dry-run` to see which operations will be performed.

### Q: How to handle filename conflicts during copy/move operations?

A: The system automatically handles conflicts by:
- Comparing file content when names match
- Auto-renaming with numeric suffixes (e.g., `movie.srt` → `movie.1.srt`)
- Creating backups when `--backup` is enabled
- Skipping conflicting files and continuing with others

## LICENSE

### GPLv3

<img src="https://github.com/user-attachments/assets/8712a047-a117-458d-9c56-cbd3d0e622d8" alt="gplv3" width="300" />

[GNU GENERAL PUBLIC LICENSE Version 3](LICENSE)

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see [https://www.gnu.org/licenses/](https://www.gnu.org/licenses/).

---

> [!NOTE]  
> This project is fully developed using GitHub Copilot and Codex CLI, with an attempt to maintain the maintainability of the software architecture. My goal is to practice controlling and planning professional software engineering work entirely through prompt engineering with AI. This is what professional vibe coding should be.
