# SubX-CLI

[![CI](https://github.com/jim60105/subx-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/ci.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli)

一個智慧字幕處理 CLI 工具，使用 AI 技術自動匹配、重命名和處理字幕文件。

> [!WARNING]  
> This project is currently in a very early stage of development and isn't functional at the moment. I would appreciate it if you could star it, so you'll be notified when I release updates in the future.

## 功能特色

- 🤖 **AI 智慧匹配** - 自動識別影片與字幕的對應關係並重命名
- 🔄 **格式轉換** - 支援 SRT、ASS、VTT、SUB 等主流字幕格式互轉
- ⏰ **時間軸校正** - 自動檢測並修正字幕時間偏移問題
- 🏃 **批量處理** - 一次處理整個資料夾的媒體文件
- 🔍 **Dry-run 模式** - 預覽操作結果，安全可靠
+ 📦 **快取管理** - 管理 Dry-run 結果快取檔案
## 安裝

### Linux

#### 方式 1：下載並執行安裝腳本
```bash
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/main/scripts/install.sh | bash
```

#### 方式 2：直接下載預編譯檔案
```bash
# 下載最新版本
curl -L "https://github.com/jim60105/subx-cli/releases/latest/download/subx-linux-x86_64" -o subx-cli
chmod +x subx-cli
sudo mv subx-cli /usr/local/bin/
```

#### 方式 3：使用 Cargo 編譯安裝
```bash
# 從 crates.io 安裝
cargo install subx-cli

# 或從原始碼編譯
git clone https://github.com/jim60105/subx-cli.git
cd subx-cli
cargo build --release
sudo cp target/release/subx-cli /usr/local/bin/
```

## 快速開始

### 1. 配置 API 金鑰
```bash
# 設定 OpenAI API Key (用於 AI 匹配功能)
export OPENAI_API_KEY="your-api-key-here"

# 或建立配置文件
subx-cli config set openai-key "your-api-key-here"
```

### 2. 基本使用

**字幕匹配與重命名**
```bash
# 處理單個資料夾
subx-cli match /path/to/media/folder

# 預覽模式（不實際執行）
subx-cli match --dry-run /path/to/media/folder
```

**格式轉換**
```bash
# 單文件轉換
subx-cli convert subtitle.ass -o subtitle.srt

# 批量轉換
subx-cli convert --format srt /path/to/subtitles/

# 轉換並保留原文件
subx-cli convert --keep-original subtitle.vtt -o subtitle.srt
```

**時間軸校正**
```bash
# 自動檢測偏移
subx-cli sync video.mp4 subtitle.srt

# 手動指定偏移
subx-cli sync --offset -2.5 subtitle.srt

# 批量同步整個資料夾
subx-cli sync --batch /path/to/media/folder
```

## 使用範例

### 典型工作流程
```bash
# 1. 處理下載的影片和字幕
cd ~/Downloads/TV_Show_S01/

# 2. AI 匹配並重命名字幕
subx-cli match --dry-run .  # 先預覽
subx-cli match .            # 確認後執行

# 3. 統一轉換為 SRT 格式
subx-cli convert --format srt .

# 4. 修正時間同步問題
subx-cli sync --batch .
```

### 資料夾結構範例
```
處理前:
TV_Show_S01/
├── S01E01.mkv
├── S01E02.mkv
├── subtitle_from_internet_1.ass
└── subtitle_from_internet_2.ass

處理後:
TV_Show_S01/
├── S01E01.mkv
├── S01E01.ass          # 匹配並重命名
├── S01E02.mkv
└── S01E02.ass          # 匹配並重命名
```

## 配置選項

### 配置文件位置
- Linux/macOS: `~/.config/subx/config.toml`
- Windows: `%APPDATA%\subx\config.toml`

### 配置範例
```toml
[ai]
provider = "openai"
model = "gpt-4o-mini"
max_sample_length = 2000

[formats]
default_output = "srt"
preserve_styling = true

[sync]
max_offset_seconds = 30
audio_sample_rate = 16000
```

## 命令參考

### `subx-cli match` - AI 匹配重命名
```
選項:
  --dry-run              預覽模式，不實際執行
  --confidence <NUM>     最低信心度閾值 (0-100)
  --recursive           遞歸處理子資料夾
  --backup              重命名前備份原文件
```

### `subx-cli convert` - 格式轉換
```
選項:
  --format <FORMAT>     目標格式 (srt|ass|vtt|sub)
  --output, -o <FILE>   輸出文件名
  --keep-original       保留原始文件
  --encoding <ENC>      指定文字編碼
```

### `subx-cli sync` - 時間軸校正
```
選項:
  --offset <SECONDS>    手動指定偏移量
  --batch               批量處理模式
  --range <SECONDS>     偏移檢測範圍
  --method <METHOD>     同步方法 (audio|manual)
```

### `subx-cli config` - 配置管理
```
使用:
  subx-cli config set <KEY> <VALUE>   設定配置值
  subx-cli config get <KEY>           獲取配置值
  subx-cli config list                列出所有配置
  subx-cli config reset               重置配置
```

### `subx-cli cache` - Dry-run 快取管理
```
選項:
  clear           清除所有 Dry-run 快取檔案

### `subx-cli generate-completion` - 產生 shell 補全腳本
```
使用:
  subx-cli generate-completion <SHELL>  支援的 shell: bash, zsh, fish, powershell, elvish
```

## 支援格式

| 格式 | 讀取 | 寫入 | 說明 |
|------|------|------|------|
| SRT  | ✅   | ✅   | SubRip 字幕 |
| ASS  | ✅   | ✅   | Advanced SSA |
| VTT  | ✅   | ✅   | WebVTT |
| SUB  | ✅   | ⚠️   | 多種 SUB 變體 |

## 疑難排解

### 常見問題

**Q: AI 匹配準確度不高怎麼辦？**
A: 確保文件名包含足夠的識別信息（如劇名、季數、集數）。

**Q: 時間軸同步失敗？**
A: 確保影片文件可訪問，並嘗試手動指定偏移量：`subx-cli sync --offset <seconds>`

---

> [!NOTE]  
> This project is fully developed using GitHub Copilot and Codex CLI, with an attempt to maintain the maintainability of the software architecture. My goal is to practice controlling and planning professional software engineering work entirely through prompt engineering with AI. This is what professional vibe coding should be.
