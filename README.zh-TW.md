# SubX-CLI

<div align="center">
  <img src="assets/logo.svg" alt="SubX CLI Logo" width="800" height="300">

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli) [![codecov](https://codecov.io/gh/jim60105/subx-cli/graph/badge.svg?token=2C53RSNNAL)](https://codecov.io/gh/jim60105/subx-cli)

[English](./README.md) | 中文

一個字幕處理 CLI 工具，使用 AI 智慧自動匹配、重命名和處理字幕檔案。

</div>

> [!NOTE]  
> 這個專案目前處於早期開發階段，基本功能已經實作完成但仍有改進空間。若是發現任何問題歡迎提交 Issue。

## 功能特色

- 🤖 **AI 智慧匹配** - 使用 AI 技術自動識別影片與字幕的對應關係並重命名
- � **檔案整理** - 自動複製或移動匹配的字幕檔案到影片資料夾，實現無縫播放
- �🔄 **格式轉換** - 支援 SRT、ASS、VTT、SUB 等主流字幕格式互轉
- ⏰ **時間軸校正** - 自動檢測並修正字幕時間偏移問題
- 🏃 **批量處理** - 一次處理整個資料夾的媒體檔案
- 🔍 **Dry-run 模式** - 預覽操作結果，安全可靠
- 📦 **快取管理** - 重複 Dry-run 可直接重用先前的分析結果，提高效率

## 安裝

### Linux

#### 方式 1：下載並執行安裝腳本
```bash
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/master/scripts/install.sh | bash
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

# 可選：設定自訂 OpenAI Base URL (用於 OpenAI API 或私有部署)
export OPENAI_BASE_URL="https://api.openai.com/v1"

# 或通過配置檔案指令設定
subx-cli config set ai.api_key "your-api-key-here"
subx-cli config set ai.base_url "https://api.openai.com/v1"
subx-cli config set ai.model "gpt-4.1-mini"
subx-cli config set general.backup_enabled true
```

### 2. 基本使用

**字幕匹配與重命名**
```bash
# 處理單個資料夾
subx-cli match /path/to/media/folder

# 預覽模式（不實際執行）
subx-cli match --dry-run /path/to/media/folder

# 遞迴處理子資料夾
subx-cli match --recursive /path/to/media/folder

# 複製匹配的字幕到影片資料夾
subx-cli match --copy /path/to/media/folder

# 移動匹配的字幕到影片資料夾
subx-cli match --move /path/to/media/folder

# 結合遞迴和備份選項使用
subx-cli match --recursive --copy --backup /path/to/media/folder
```

**格式轉換**
```bash
# 單檔案轉換
subx-cli convert subtitle.ass --format srt

# 批量轉換
subx-cli convert --format srt /path/to/subtitles/

# 轉換並保留原檔案
subx-cli convert --keep-original subtitle.vtt --format srt
```

**時間軸校正**
```bash
# 自動檢測偏移
subx-cli sync video.mp4 subtitle.srt

# 手動指定偏移
subx-cli sync --offset -2.5 video.mp4 subtitle.srt

# 批量同步整個資料夾
subx-cli sync --batch /path/to/media/folder
```

**快取管理**
```bash
# 清除 Dry-run 快取
subx-cli cache clear
```

## 使用範例

### 典型工作流程
```bash
# 1. 處理下載的影片和字幕
cd ~/Downloads/TV_Show_S01/

# 2. AI 匹配並重命名字幕，同時整理檔案
subx-cli match --dry-run --copy .  # 先預覽
subx-cli match --copy .            # 確認後執行

# 3. 統一轉換為 SRT 格式
subx-cli convert --format srt .

# 4. 修正時間同步問題
subx-cli sync --batch .
```

### 檔案整理應用場景
```bash
# 場景 1：保留原始字幕位置，複製到影片資料夾
subx-cli match --recursive --copy /media/collection/

# 場景 2：移動字幕到影片資料夾，清理原始位置
subx-cli match --recursive --move /media/collection/

# 場景 3：預覽檔案整理操作
subx-cli match --dry-run --copy --recursive /media/collection/

# 場景 4：使用備份保護進行檔案整理
subx-cli match --move --backup --recursive /media/collection/
```

### 資料夾結構範例
```
處理前（分散式結構）：
media/
├── movies/
│   ├── Action/
│   │   └── Movie1.mkv
│   └── Drama/
│       └── Movie2.mp4
└── subtitles/
    ├── english/
    │   ├── Movie1.srt
    │   └── Movie2.srt
    └── chinese/
        ├── Movie1.zh.srt
        └── Movie2.zh.srt

使用 --copy 選項處理後：
media/
├── movies/
│   ├── Action/
│   │   ├── Movie1.mkv
│   │   ├── Movie1.srt           # 從 subtitles/english/ 複製
│   │   └── Movie1.zh.srt        # 從 subtitles/chinese/ 複製
│   └── Drama/
│       ├── Movie2.mp4
│       ├── Movie2.srt           # 從 subtitles/english/ 複製
│       └── Movie2.zh.srt        # 從 subtitles/chinese/ 複製
└── subtitles/                   # 原始檔案保留
    ├── english/
    │   ├── Movie1.srt
    │   └── Movie2.srt
    └── chinese/
        ├── Movie1.zh.srt
        └── Movie2.zh.srt

使用 --move 選項處理後：
media/
├── movies/
│   ├── Action/
│   │   ├── Movie1.mkv
│   │   ├── Movie1.srt           # 從 subtitles/english/ 移動
│   │   └── Movie1.zh.srt        # 從 subtitles/chinese/ 移動
│   └── Drama/
│       ├── Movie2.mp4
│       ├── Movie2.srt           # 從 subtitles/english/ 移動
│       └── Movie2.zh.srt        # 從 subtitles/chinese/ 移動
└── subtitles/                   # 原始檔案已移除
    ├── english/                 # 空目錄
    └── chinese/
```

## 配置選項

### 配置檔案位置
- Linux/macOS: `~/.config/subx/config.toml`
- Windows: `%APPDATA%\subx\config.toml`

### 配置範例
```toml
[ai]
# AI 服務提供商，目前支援 "openai"
provider = "openai"
# 使用的 AI 模型
model = "gpt-4.1-mini"
# API 端點，可由 OPENAI_BASE_URL 環境變數覆蓋
base_url = "https://api.openai.com/v1"
# API 金鑰，可由 OPENAI_API_KEY 環境變數覆蓋
api_key = "your-api-key-here"
# AI 回應隨機性控制 (0.0-2.0)
temperature = 0.3
# 傳送給 AI 的內容長度上限
max_sample_length = 2000
# API 請求失敗重試次數
retry_attempts = 3
# 重試間隔 (毫秒)
retry_delay_ms = 1000

[formats]
# 預設輸出格式
default_output = "srt"
# 轉換時是否保留樣式
preserve_styling = true
# 預設文字編碼
default_encoding = "utf-8"
# 編碼檢測信心度閾值 (0.0-1.0)
encoding_detection_confidence = 0.7

[sync]
# 音訊同步的最大偏移範圍 (秒)
max_offset_seconds = 30.0
# 音訊相關性分析閾值 (0.0-1.0)
correlation_threshold = 0.7
# 對話檢測的音訊能量閾值
dialogue_detection_threshold = 0.01
# 最小對話片段持續時間 (毫秒)
min_dialogue_duration_ms = 500
# 對話片段合併間隔 (毫秒)
dialogue_merge_gap_ms = 500
# 是否啟用對話檢測功能
enable_dialogue_detection = true
# 音訊處理採樣率 (Hz)
audio_sample_rate = 16000
# 是否自動檢測音訊採樣率
auto_detect_sample_rate = true

[general]
# 是否啟用檔案備份，可由 SUBX_BACKUP_ENABLED 環境變數覆蓋
backup_enabled = false
# 最大並發任務數
max_concurrent_jobs = 4
# 任務執行逾時時間 (秒)
task_timeout_seconds = 3600
# 是否顯示進度條
enable_progress_bar = true
# 工作執行緒閒置逾時 (秒)
worker_idle_timeout_seconds = 300

[parallel]
# 任務佇列大小限制
task_queue_size = 100
# 是否啟用任務優先級排程
enable_task_priorities = true
# 是否啟用自動負載平衡
auto_balance_workers = true
# 佇列溢出策略 ("block" | "drop_oldest" | "reject")
queue_overflow_strategy = "block"
```

## 命令參考

### `subx-cli match` - AI 匹配重命名
```
選項:
  <PATH>                目標資料夾路徑
  --dry-run             預覽模式，不實際執行
  --confidence <NUM>    最低信心度閾值 (0-100, 預設值: 80)
  --recursive           遞歸處理子資料夾
  --backup              重命名前備份原檔案
  --copy, -c            複製匹配的字幕檔案到影片資料夾
  --move, -m            移動匹配的字幕檔案到影片資料夾

檔案整理功能:
  --copy 和 --move 選項啟用自動檔案整理功能，提升媒體播放器相容性。
  當字幕與影片位於不同目錄時，這些選項會將字幕檔案複製或移動到
  對應影片檔案所在的資料夾。
  
  - --copy: 保留原始字幕檔案在原位置
  - --move: 移動後移除原始字幕檔案
  - 這兩個選項互斥，不能同時使用
  - 僅在字幕和影片檔案位於不同目錄時生效
  - 包含自動檔名衝突解決和備份支援功能

配置支援:
  - AI 設定: 支援自訂 API 端點、模型、溫度等參數
  - 並行處理: 支援最大並發數、任務佇列大小、優先級排程等
  - 一般設定: 支援備份、進度條、逾時控制等
```

### `subx-cli convert` - 格式轉換
```
選項:
  <INPUT>               輸入檔案或資料夾路徑
  --format <FORMAT>     目標格式 (srt|ass|vtt|sub)
  --output, -o <FILE>   輸出檔案名
  --keep-original       保留原始檔案
  --encoding <ENC>      指定文字編碼 (預設值: utf-8)

配置支援:
  - 格式設定: 預設輸出格式、樣式保留、編碼檢測等
```

### `subx-cli detect-encoding` - 檔案編碼檢測
```
選項:
  <FILES>...             目標檔案路徑
  -v, --verbose          顯示詳細樣本文字

配置支援:
  - 格式設定: 編碼檢測信心度閾值、預設編碼等
```

### `subx-cli sync` - 時間軸校正
```
選項:
  <VIDEO>               影片檔案路徑
  <SUBTITLE>            字幕檔案路徑
  --offset <SECONDS>    手動指定偏移量
  --batch               批量處理模式
  --range <SECONDS>     偏移檢測範圍 (預設值: 配置檔案中的 max_offset_seconds)
  --threshold <THRESHOLD>  相關性閾值 (0-1，預設值: 配置檔案中的 correlation_threshold)

配置支援:
  - 同步設定: 最大偏移範圍、相關性閾值、對話檢測等
  - 音訊處理: 採樣率、對話檢測閾值、片段合併等
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
  clear                 清除所有 Dry-run 快取檔案
```

### `subx-cli generate-completion` - 產生 shell 補全腳本
```
使用:
  subx-cli generate-completion <SHELL>  支援的 shell: bash, zsh, fish, powershell, elvish
```

## 支援格式

| 格式 | 讀取 | 寫入 | 說明 |
|------|------|------|------|
| SRT  | ✅   | ✅   | SubRip 字幕格式 |
| ASS  | ✅   | ✅   | Advanced SubStation Alpha 格式 |
| VTT  | ✅   | ✅   | WebVTT 格式 |
| SUB  | ✅   | ⚠️   | 多種 SUB 變體格式 |

## 疑難排解

### 常見問題

**Q: AI 匹配準確度不高怎麼辦？**
A: 確保檔案名包含足夠的識別資訊（如劇名、季數、集數）。同時可以嘗試調整 `--confidence` 參數或配置 AI 模型溫度：`subx-cli config set ai.temperature 0.1`

**Q: 時間軸同步失敗？**
A: 確保影片檔案可存取，並檢查檔案格式是否支援。如果自動同步不理想，可以嘗試：
- 手動指定偏移量：`subx-cli sync --offset <seconds> video.mp4 subtitle.srt`
- 調整同步配置：`subx-cli config set sync.correlation_threshold 0.6`
- 啟用對話檢測：`subx-cli config set sync.enable_dialogue_detection true`

**Q: 處理大量檔案時性能不佳？**
A: 可以調整並行處理配置：
```bash
subx-cli config set general.max_concurrent_jobs 8     # 增加並發數
subx-cli config set parallel.task_queue_size 200     # 增加佇列大小
subx-cli config set parallel.auto_balance_workers true # 啟用負載平衡
```

**Q: 編碼檢測不準確？**
A: 調整檢測信心度閾值：`subx-cli config set formats.encoding_detection_confidence 0.8`

**Q: 快取檔案佔用太多空間？**
A: 使用 `subx-cli cache clear` 指令可以清除所有快取檔案。

**Q: 如何在新的影片與字幕加入後重新匹配？**
A: 先清除快取 `subx-cli cache clear`，再重新執行 match 命令。

**Q: 任務執行逾時怎麼辦？**
A: 增加逾時時間：`subx-cli config set general.task_timeout_seconds 7200`  # 設定為 2 小時

**Q: 檔案整理（複製/移動）操作失敗？**
A: 檢查以下常見問題：
- 確保目標影片目錄具有寫入權限
- 檢查複製操作是否有足夠的磁碟空間
- 檔名衝突時系統會自動重新命名並加上數字後綴
- 使用 `--dry-run` 在執行前預覽操作：`subx-cli match --dry-run --copy /path`

**Q: 可以同時使用 --copy 和 --move 嗎？**
A: 不可以，這兩個選項互斥。請選擇 `--copy` 保留原始檔案或 `--move` 清理原始位置。

**Q: 為什麼有些字幕沒有被複製/移動到影片資料夾？**
A: 複製/移動操作只在以下條件下執行：
- 字幕和影片檔案位於不同目錄
- AI 匹配信心度超過閾值（預設 80%）
- 目標位置不存在相同名稱的檔案
使用 `--dry-run` 查看將要執行的操作。

**Q: 如何處理複製/移動操作中的檔名衝突？**
A: 系統會自動處理衝突：
- 比較檔案內容當名稱相同時
- 自動重新命名並加上數字後綴（如 `movie.srt` → `movie.1.srt`）
- 啟用 `--backup` 時建立備份檔案
- 跳過衝突檔案並繼續處理其他檔案

## LICENSE

### GPLv3

<img src="https://github.com/user-attachments/assets/8712a047-a117-458d-9c56-cbd3d0e622d8" alt="gplv3" width="300" />

[GNU GENERAL PUBLIC LICENSE Version 3](LICENSE)

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see [https://www.gnu.org/licenses/](https://www.gnu.org/licenses/).

---

> [!NOTE]  
> 這個專案完全使用 GitHub Copilot 和 Codex CLI 開發，並嘗試維持軟體架構的可維護性。我的目標是完全透過提示詞工程與 AI 協作，進行專業水準的軟體規劃和實作。我認為這才是專業人士的 Vibe Coding 該有的樣子。
