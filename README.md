# SubX-CLI

<div align="center">
  <img src="assets/logo.svg" alt="SubX CLI Logo" width="800" height="300">

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli) [![codecov](https://codecov.io/gh/jim60105/subx-cli/graph/badge.svg?token=2C53RSNNAL)](https://codecov.io/gh/jim60105/subx-cli)

English | [中文](./README.zh-TW.md)

AI-powered CLI for automated subtitle matching, renaming, format conversion, and timeline correction.

</div>

## What SubX Does

Point SubX at a media folder and it uses AI to figure out which subtitle
files belong to which videos, even when the filenames are in different
languages and share no common naming pattern.

```
Before:
media/
├── movies/
│   └── The.Matrix.1080p.mkv
└── subtitles/
    ├── Matrix_EN_Sub.srt
    └── 駭客任務_中文字幕.srt

After running: subx-cli match --copy media/
media/
├── movies/
│   ├── The.Matrix.1080p.mkv
│   ├── The.Matrix.1080p.srt         ← AI matched Matrix_EN_Sub.srt
│   └── The.Matrix.1080p.zh.srt      ← AI matched 駭客任務_中文字幕.srt
└── subtitles/                         (originals preserved)
    ├── Matrix_EN_Sub.srt
    └── 駭客任務_中文字幕.srt
```

Beyond matching, SubX converts between subtitle formats (SRT, ASS, VTT,
SUB), corrects timing drift using local Voice Activity Detection, and
detects file character encodings.

## Quick Start

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/master/scripts/install.sh | bash

# Configure (OpenRouter with free DeepSeek model)
export OPENROUTER_API_KEY="your-key"
subx-cli config set ai.provider openrouter
subx-cli config set ai.model "deepseek/deepseek-r1-0528:free"

# Preview what SubX will do
subx-cli match --dry-run --copy /path/to/media/

# Execute
subx-cli match --copy /path/to/media/
```

For OpenAI, set `OPENAI_API_KEY` instead. For Azure OpenAI, set
`AZURE_OPENAI_API_KEY` and `AZURE_OPENAI_ENDPOINT`. See the
[Configuration Guide](docs/configuration-guide.md) for all provider options
and environment variables.

## Commands

The `match`, `convert`, `sync`, and `detect-encoding` commands support `-i`
for multiple inputs and `--recursive` for subdirectory scanning. See the
[Command Reference](docs/command-reference.md) for full options, examples,
and workflows.

| Command | Purpose | Example |
|---------|---------|---------|
| `match` | AI-match subtitles to videos, rename/copy/move | `subx-cli match --copy ./media` |
| `convert` | Convert between subtitle formats | `subx-cli convert --format srt ./subs/` |
| `sync` | Correct timing via VAD or manual offset | `subx-cli sync video.mp4 subtitle.srt` |
| `detect-encoding` | Identify subtitle file encodings | `subx-cli detect-encoding *.srt` |
| `config` | View and modify settings | `subx-cli config set ai.provider openai` |
| `cache` | Manage dry-run result cache | `subx-cli cache clear` |

## Installation

### Linux / macOS

```bash
# One-line installer
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/master/scripts/install.sh | bash

# Or download the binary directly
curl -L "https://github.com/jim60105/subx-cli/releases/latest/download/subx-linux-x86_64" -o subx-cli
chmod +x subx-cli && sudo mv subx-cli /usr/local/bin/
```

### From Source (any platform)

```bash
# From crates.io
cargo install subx-cli

# Or compile from the repository
git clone https://github.com/jim60105/subx-cli.git
cd subx-cli
cargo build --release
```

## Supported Formats

| Format | Read | Write | Notes |
|--------|------|-------|-------|
| SRT | ✅ | ✅ | SubRip — most widely supported |
| ASS | ✅ | ✅ | Advanced SubStation Alpha — rich styling |
| VTT | ✅ | ✅ | WebVTT — web-native format |
| SUB | ✅ | ⚠️ | Multiple SUB variants, partial write support |

## Documentation

- [Command Reference](docs/command-reference.md) — full options, examples,
  and workflows for every subcommand
- [Configuration Guide](docs/configuration-guide.md) — all settings,
  environment variables, and troubleshooting
- [Technical Architecture](docs/tech-architecture.md) — codebase structure
  and design decisions
- [AI Provider Integration](docs/ai-provider-integration-guide.md) — how to
  add a new AI provider

## License

### GPLv3

<img src="https://github.com/user-attachments/assets/8712a047-a117-458d-9c56-cbd3d0e622d8" alt="gplv3" width="300" />

[GNU GENERAL PUBLIC LICENSE Version 3](LICENSE)

Copyright (C) 2025 Jim Chen <Jim@ChenJ.im>.

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see [https://www.gnu.org/licenses/](https://www.gnu.org/licenses/).
