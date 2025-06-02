# SubX

一個智慧字幕處理 CLI 工具，使用 AI 技術自動匹配、重命名和處理字幕文件。

> [!WARNING]  
> This project is currently in a very early stage of development and isn't functional at the moment. I would appreciate it if you could star it, so you'll be notified when I release updates in the future.

## 功能特色

- 🤖 **AI 智慧匹配** - 自動識別影片與字幕的對應關係並重命名
- 🔄 **格式轉換** - 支援 SRT、ASS、VTT、SUB 等主流字幕格式互轉
- ⏰ **時間軸校正** - 自動檢測並修正字幕時間偏移問題
- 🏃 **批量處理** - 一次處理整個資料夾的媒體文件
- 🔍 **Dry-run 模式** - 預覽操作結果，安全可靠
<!-- 
## 安裝

### 從 Releases 下載
```bash
# macOS / Linux
curl -L https://github.com/yourusername/subx/releases/latest/download/subx-{platform} -o subx
chmod +x subx
sudo mv subx /usr/local/bin/

# Windows
# 下載 subx.exe 並添加到 PATH
```

### 從源碼編譯
```bash
git clone https://github.com/yourusername/subx.git
cd subx
cargo build --release
sudo cp target/release/subx /usr/local/bin/ -->
```

## 快速開始

### 1. 配置 API 金鑰
```bash
# 設定 OpenAI API Key (用於 AI 匹配功能)
export OPENAI_API_KEY="your-api-key-here"

# 或建立配置文件
subx config set openai-key "your-api-key-here"
```

### 2. 基本使用

**字幕匹配與重命名**
```bash
# 處理單個資料夾
subx match /path/to/media/folder

# 預覽模式（不實際執行）
subx match --dry-run /path/to/media/folder
```

**格式轉換**
```bash
# 單文件轉換
subx convert subtitle.ass -o subtitle.srt

# 批量轉換
subx convert --format srt /path/to/subtitles/

# 轉換並保留原文件
subx convert --keep-original subtitle.vtt -o subtitle.srt
```

**時間軸校正**
```bash
# 自動檢測偏移
subx sync video.mp4 subtitle.srt

# 手動指定偏移
subx sync --offset -2.5 subtitle.srt

# 批量同步整個資料夾
subx sync --batch /path/to/media/folder
```

## 使用範例

### 典型工作流程
```bash
# 1. 處理下載的影片和字幕
cd ~/Downloads/TV_Show_S01/

# 2. AI 匹配並重命名字幕
subx match --dry-run .  # 先預覽
subx match .            # 確認後執行

# 3. 統一轉換為 SRT 格式
subx convert --format srt .

# 4. 修正時間同步問題
subx sync --batch .
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

### `subx match` - AI 匹配重命名
```
選項:
  --dry-run              預覽模式，不實際執行
  --confidence <NUM>     最低信心度閾值 (0-100)
  --recursive           遞歸處理子資料夾
  --backup              重命名前備份原文件
```

### `subx convert` - 格式轉換
```
選項:
  --format <FORMAT>     目標格式 (srt|ass|vtt|sub)
  --output, -o <FILE>   輸出文件名
  --keep-original       保留原始文件
  --encoding <ENC>      指定文字編碼
```

### `subx sync` - 時間軸校正
```
選項:
  --offset <SECONDS>    手動指定偏移量
  --batch               批量處理模式
  --range <SECONDS>     偏移檢測範圍
  --method <METHOD>     同步方法 (audio|manual)
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
A: 確保影片文件可訪問，並嘗試手動指定偏移量：`subx sync --offset <seconds>`

---

> [!NOTE]  
> This project is fully developed using GitHub Copilot and Codex CLI, with an attempt to maintain the maintainability of the software architecture. My goal is to practice controlling and planning professional software engineering work entirely through prompt engineering with AI. This is what professional video coding should be.
