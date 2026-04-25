# Command Reference

SubX-CLI provides seven subcommands. The `match`, `convert`, `sync`, and
`detect-encoding` commands accept `-i <PATH>` for specifying multiple input
sources and `--recursive` for subdirectory scanning. Positional path
arguments and `-i` inputs are combined (except in `detect-encoding`, where
they are mutually exclusive). The `config`, `cache`, and
`generate-completion` commands have their own argument structures.

### Archive Input Support

The `match`, `convert`, `sync`, and `detect-encoding` commands accept
`.zip` archives as direct inputs (positional path or via `-i`). Archives
are transparently extracted to a temporary directory for the duration of
the command, and the extracted files are processed as if they had been
supplied directly. Temporary directories are cleaned up automatically
when the command finishes.

`.rar` archives are supported only when SubX is built with the
`archive-rar` feature (which links against the native `libunrar`
library). Standard release builds do not include RAR support.

Extraction is governed by safety limits to prevent decompression bombs
and path-traversal attacks: a maximum total expanded size of 1 GiB and a
maximum of 10,000 entries per archive. Symlink, hardlink, and
path-traversal entries are skipped with a warning.

Only archives passed *directly* as inputs are extracted. Archives
discovered during recursive directory traversal are **not** extracted —
they are treated as ordinary files and filtered by the command's
extension list. To opt out of automatic extraction entirely, pass
`--no-extract`; the archive is then treated as a regular file and is
subject to the same extension filter as any other input.

## `match` — AI Subtitle Matching

Scans input paths for video and subtitle files, uses AI to determine which
subtitles belong to which videos, and renames (or copies/moves) the
subtitles to match the video filenames.

```
subx-cli match [OPTIONS] [PATH]
```

### Options

| Flag | Description |
|------|-------------|
| `[PATH]` | Target folder or file |
| `-i`, `--input <PATH>` | Additional input path (repeatable) |
| `--dry-run` | Preview matches without executing |
| `-r`, `--recursive` | Recurse into subdirectories |
| `--confidence <N>` | Minimum confidence threshold, 0–100 (default: 80) |
| `--backup` | Back up original files before renaming |
| `--copy`, `-c` | Copy matched subtitles into the video's directory |
| `--move`, `-m` | Move matched subtitles into the video's directory |
| `--no-extract` | Skip automatic extraction of archive files (`.zip`, `.rar`). When set, archive files are treated as regular files and subject to the normal extension filter. |

`--copy` and `--move` are mutually exclusive. They only take effect when
the subtitle and video reside in different directories. When a filename
collision occurs at the target location, SubX compares file content and
either skips the duplicate or appends a numeric suffix (e.g.,
`movie.1.srt`).

### Examples

```bash
# Process a single folder
subx-cli match /path/to/media/

# Preview first, then execute
subx-cli match --dry-run --copy /path/to/media/
subx-cli match --copy /path/to/media/

# Multiple inputs with recursive scanning
subx-cli match -i /media/movies -i /media/tv_shows --recursive --copy

# Mix files and directories with backup
subx-cli match -i ./video1.mp4 -i ./subtitles_dir --recursive --copy --backup

# Move subtitles instead of copying (removes originals)
subx-cli match --recursive --move --backup /media/collection/
```

### File Organization Scenarios

**Scenario: Videos and subtitles in separate trees**

```
Before:
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
```

After `subx-cli match --copy --recursive media/`:

```
media/
├── movies/
│   ├── Action/
│   │   ├── The.Matrix.1999.1080p.BluRay.mkv
│   │   ├── The.Matrix.1999.1080p.BluRay.srt        ← AI matched Matrix_EN_Sub.srt
│   │   └── The.Matrix.1999.1080p.BluRay.zh.srt     ← AI matched 駭客任務_中文字幕.srt
│   └── Drama/
│       ├── Forrest.Gump.1994.720p.WEB-DL.mp4
│       ├── Forrest.Gump.1994.720p.WEB-DL.srt        ← AI matched ForrestGump_English.srt
│       └── Forrest.Gump.1994.720p.WEB-DL.zh.srt     ← AI matched 阿甘正傳.繁中.srt
└── subtitles/                                         (originals preserved with --copy)
    ├── english/
    │   ├── Matrix_EN_Sub.srt
    │   └── ForrestGump_English.srt
    └── chinese/
        ├── 駭客任務_中文字幕.srt
        └── 阿甘正傳.繁中.srt
```

With `--move` instead of `--copy`, the original subtitle files are removed
after relocation.

## `convert` — Format Conversion

Converts subtitle files between SRT, ASS, VTT, and SUB formats. Supports
single-file and batch operations.

```
subx-cli convert [OPTIONS] [INPUT]
```

### Options

| Flag | Description |
|------|-------------|
| `[INPUT]` | Input file or directory |
| `-i`, `--input <PATH>` | Additional input path (repeatable) |
| `--format <FMT>` | Target format: `srt`, `ass`, `vtt`, `sub` (optional; defaults to `formats.default_output` config) |
| `-o`, `--output <FILE>` | Output filename (single-file mode) |
| `--keep-original` | Keep the source file after conversion |
| `--encoding <ENC>` | Character encoding (default: `utf-8`) |
| `-r`, `--recursive` | Recurse into subdirectories |
| `--no-extract` | Skip automatic extraction of archive files (`.zip`, `.rar`). When set, archive files are treated as regular files and subject to the normal extension filter. |

### Examples

```bash
# Single file
subx-cli convert subtitle.ass --format srt

# Batch conversion
subx-cli convert --format srt /path/to/subtitles/

# Multiple directories with recursive scanning, preserving originals
subx-cli convert -i ./srt_files -i ./more_subtitles --format vtt --recursive --keep-original

# Specify encoding explicitly
subx-cli convert -i movie.srt --format srt --encoding utf-8
```

## `sync` — Timeline Correction

Corrects subtitle timing by computing the offset between audio speech
segments and subtitle timestamps. The primary method uses local Voice
Activity Detection (VAD); a manual mode is available for direct offset
specification.

```
subx-cli sync [OPTIONS] [PATHS]...
```

### Options

| Flag | Description |
|------|-------------|
| `[PATH]...` | Positional video, subtitle, or directory paths |
| `-v`, `--video <VIDEO>` | Video file path |
| `-s`, `--subtitle <SUBTITLE>` | Subtitle file path |
| `-i`, `--input <PATH>` | Additional input path (repeatable) |
| `--offset <SECONDS>` | Manual offset in seconds (bypasses VAD) |
| `-b`, `--batch [DIRECTORY]` | Batch mode; optionally specify a directory path |
| `--method <M>` | Sync method: `vad` or `manual` (omit to auto-select) |
| `-w`, `--window <SECONDS>` | Analysis time window in seconds (default: 30) |
| `--vad-sensitivity <SENSITIVITY>` | VAD sensitivity 0.0–1.0 (overrides config) |
| `-o`, `--output <PATH>` | Output file path |
| `-r`, `--recursive` | Recurse into subdirectories |
| `--dry-run` | Preview sync results without writing |
| `--verbose` | Show detailed processing output |
| `--force` | Overwrite existing output file without confirmation |
| `--no-extract` | Skip automatic extraction of archive files (`.zip`, `.rar`). When set, archive files are treated as regular files and subject to the normal extension filter. |

Supported audio containers: MP4, MKV, WebM, OGG, WAV. SubX decodes audio
natively via Symphonia — FFmpeg is not required.

### Examples

```bash
# Automatic VAD sync (requires audio/video file + subtitle)
subx-cli sync video.mp4 subtitle.srt

# Manual offset (subtitle file only)
subx-cli sync --offset 2.5 subtitle.srt

# Custom VAD sensitivity for quiet audio
subx-cli sync --vad-sensitivity 0.8 video.mp4 subtitle.srt

# Batch processing with recursive scanning
subx-cli sync -i ./movies -i ./tv_shows --batch --recursive --method vad

# Preview batch results
subx-cli sync -i ./media --batch --recursive --dry-run --verbose
```

## `detect-encoding` — Character Encoding Detection

Identifies the character encoding of subtitle files. Useful for diagnosing
garbled text before conversion.

```
subx-cli detect-encoding [OPTIONS] [FILES]...
```

### Options

| Flag | Description |
|------|-------------|
| `<FILES>...` | Target file(s) (required; mutually exclusive with `-i`) |
| `-i`, `--input <PATH>` | Input directory path (repeatable; mutually exclusive with positional files) |
| `-v`, `--verbose` | Show sample text from each file |
| `-r`, `--recursive` | Recurse into subdirectories |
| `--no-extract` | Skip automatic extraction of archive files (`.zip`, `.rar`). When set, archive files are treated as regular files and subject to the normal extension filter. |

Positional file arguments and `-i` cannot be used together. Use `-i` for
directory-based scanning, or positional arguments for specific files.

### Examples

```bash
# Check specific files
subx-cli detect-encoding *.srt

# Scan directories recursively with verbose output
subx-cli detect-encoding -i ./subtitles1 -i ./subtitles2 --recursive --verbose
```

## `config` — Configuration Management

Reads and writes SubX configuration values. Settings persist to the config
file at `~/.config/subx/config.toml` (Linux/macOS) or
`%APPDATA%\subx\config.toml` (Windows).

```
subx-cli config <SUBCOMMAND>
```

| Subcommand | Description |
|------------|-------------|
| `set <KEY> <VALUE>` | Set a configuration value |
| `get <KEY>` | Get a configuration value |
| `list` | List all configuration values |
| `reset` | Reset configuration to defaults |

### Examples

```bash
subx-cli config set ai.provider openrouter
subx-cli config set ai.model "deepseek/deepseek-r1-0528:free"
subx-cli config get ai.provider
subx-cli config list
subx-cli config reset
```

For all configuration keys and environment variables, see the
[Configuration Guide](configuration-guide.md).

## `cache` — Cache Management

Manages the dry-run result cache and operation journal. SubX caches AI
analysis results so repeated `--dry-run` invocations reuse previous
matches. The journal records every file operation for rollback.

```
subx-cli cache <SUBCOMMAND>
```

| Subcommand | Description |
|------------|-------------|
| `status` | Display cache metadata (size, age, validity) |
| `apply` | Replay cached dry-run results without calling the AI |
| `rollback` | Undo the most recent batch of file operations |
| `clear` | Remove cached data (cache, journal, or both) |

### `cache status`

```
subx-cli cache status [--json]
```

| Flag | Description |
|------|-------------|
| `--json` | Output machine-readable JSON |

### `cache apply`

```
subx-cli cache apply [--yes] [--force] [--confidence <0-100>]
```

| Flag | Description |
|------|-------------|
| `--yes` | Skip interactive confirmation |
| `--force` | Bypass staleness and config hash validation |
| `--confidence <N>` | Minimum confidence threshold (0–100) |

### `cache rollback`

```
subx-cli cache rollback [--force]
```

| Flag | Description |
|------|-------------|
| `--force` | Bypass destination integrity checks |

### `cache clear`

```
subx-cli cache clear [--type <cache|journal|all>]
```

| Flag | Default | Description |
|------|---------|-------------|
| `--type` | `all` | Type of data to clear |

## `generate-completion` — Shell Completions

Generates shell completion scripts for tab-completion support.

```
subx-cli generate-completion <SHELL>
```

Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`.

```bash
# Bash
subx-cli generate-completion bash > ~/.local/share/bash-completion/completions/subx-cli

# Zsh
subx-cli generate-completion zsh > ~/.zfunc/_subx-cli

# Fish
subx-cli generate-completion fish > ~/.config/fish/completions/subx-cli.fish
```

## Workflows

### Typical Workflow

```bash
# 1. Navigate to media folder
cd ~/Downloads/TV_Show_S01/

# 2. Preview AI matching results
subx-cli match --dry-run --copy .

# 3. Execute matching and file organization
subx-cli match --copy .

# 4. Convert all subtitles to SRT
subx-cli convert --format srt .

# 5. Fix timing drift
subx-cli sync --batch .
```

### Multi-Source Workflow

```bash
# Match across multiple directories
subx-cli match -i ./Downloads/Movies -i ./Downloads/TV_Shows -i ./Backup/Subs \
    --recursive --dry-run --copy

# After reviewing dry-run output, execute
subx-cli match -i ./Downloads/Movies -i ./Downloads/TV_Shows -i ./Backup/Subs \
    --recursive --copy

# Batch convert everything to SRT
subx-cli convert -i ./Movies -i ./TV_Shows --format srt --recursive --keep-original

# Batch sync with VAD
subx-cli sync -i ./Movies -i ./TV_Shows --batch --recursive

# Check encodings
subx-cli detect-encoding -i ./Movies -i ./TV_Shows --recursive --verbose
```

## Troubleshooting

**AI matching accuracy is low.** Filenames with identifying information
(show name, season, episode) produce better results. Lower the AI
temperature for more deterministic output:
`subx-cli config set ai.temperature 0.1`

**Timeline sync produces incorrect offsets.** Verify the audio file is
accessible and in a supported container format. For quiet audio, increase
VAD sensitivity: `subx-cli config set sync.vad.sensitivity 0.8`. For noisy
audio, raise the minimum speech duration:
`subx-cli config set sync.vad.min_speech_duration_ms 200`. When automatic
detection fails, fall back to manual offset:
`subx-cli sync --offset <seconds> subtitle.srt`

**Batch processing is slow.** Increase worker count and queue size:

```bash
subx-cli config set parallel.max_workers 16
subx-cli config set parallel.task_queue_size 2000
```

**Encoding detection is wrong.** Raise the detection confidence threshold:
`subx-cli config set formats.encoding_detection_confidence 0.8`. If the
file uses a rare encoding, specify it explicitly:
`subx-cli convert --encoding big5 subtitle.srt --format srt`

**Subtitles are not copied/moved.** The `--copy` and `--move` flags only
take effect when the subtitle and video are in different directories, the AI
confidence exceeds the threshold (default 80%), and no identically named
file already exists at the target. Use `--dry-run` to preview which files
will be affected.

**`--copy` and `--move` together?** These flags are mutually exclusive. Use
`--copy` to preserve originals or `--move` to clean up after relocation.

**Cache taking too much space.** Run `subx-cli cache clear` to remove all
cached dry-run results. If new files have been added and you want fresh
matches, clear the cache before re-running `match`.

**Task execution timeouts.** Increase the timeout:
`subx-cli config set general.task_timeout_seconds 7200`
