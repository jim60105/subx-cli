# SubX-CLI

<div align="center">
  <img src="assets/logo.svg" alt="SubX CLI Logo" width="800" height="300">

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli) [![codecov](https://codecov.io/gh/jim60105/subx-cli/graph/badge.svg?token=2C53RSNNAL)](https://codecov.io/gh/jim60105/subx-cli)

[English](./README.md) | 中文

AI 智慧字幕處理工具，自動匹配、重命名及轉換字幕檔案。

</div>

## 功能特色

- 🤖 **AI 智慧匹配** - 使用 AI 技術自動識別影片與字幕的對應關係並重命名
- 📁 **檔案整理** - 自動複製或移動匹配的字幕檔案到影片資料夾，提升播放器相容性
- 🔄 **格式轉換** - 支援 SRT、ASS、VTT、SUB 等主流字幕格式互轉
- 🔊 **音訊同步** - 直接解碼多種音訊容器格式（MP4、MKV、WebM、OGG、WAV），以 VAD 為基礎進行同步，無需中間轉檔
- ⏰ **時間軸校正** - 自動偵測並修正字幕時間偏移問題
- 🏃 **批次處理** - 一次處理整個資料夾的媒體檔案
- 🔍 **Dry-run 模式** - 預覽操作結果，安全可靠
- 📦 **快取管理** - 重複 dry-run 可重用先前分析結果，提升效率

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

#### 方式 3：使用 Cargo 安裝
```bash
# 從 crates.io 安裝
cargo install subx-cli

# 或從原始碼編譯
git clone https://github.com/jim60105/subx-cli.git
cd subx-cli
cargo build --release
sudo cp target/release/subx-cli/usr/local/bin/
```

## 快速開始

### 1. 配置設定
```bash
# 設定 OpenAI API Key (僅用於 AI 匹配功能)
export OPENAI_API_KEY="your-api-key-here"

# 配置 VAD 設定
subx-cli config set sync.vad.sensitivity 0.8
subx-cli config set sync.vad.enabled true

# 啟用一般備份功能
subx-cli config set general.backup_enabled true

# 配置平行處理
subx-cli config set parallel.max_workers 8
subx-cli config set parallel.task_queue_size 1000
```

### 2. 基本使用

**字幕匹配與重命名**
```bash
# 處理單個資料夾
subx-cli match /path/to/media/folder

# 使用 -i 參數處理多個輸入來源
subx-cli match -i /path/to/videos -i /path/to/more/media

# 預覽模式（不實際執行）
subx-cli match --dry-run /path/to/media/folder

# 遞迴處理子資料夾
subx-cli match --recursive /path/to/media/folder

# 結合 -i 參數與遞迴處理
subx-cli match -i /path/to/videos -i /path/to/movies --recursive

# 複製匹配的字幕到影片資料夾
subx-cli match --copy /path/to/media/folder

# 移動匹配的字幕到影片資料夾
subx-cli match --move /path/to/media/folder

# 進階：混合檔案和目錄與多個選項
subx-cli match -i ./video1.mp4 -i ./subtitles_dir -i ./video2.mkv --recursive --copy --backup

# 結合遞迴和備份選項使用
subx-cli match --recursive --move --backup /path/to/media/folder
```

**格式轉換**
```bash
# 單檔案轉換
subx-cli convert subtitle.ass --format srt

# 使用 -i 參數批次轉換多個目錄
subx-cli convert -i ./srt_files -i ./more_subtitles --format vtt

# 批次轉換並遞迴掃描目錄
subx-cli convert -i ./srt_files -i ./more_subtitles --format vtt --recursive

# 批次轉換
subx-cli convert --format srt /path/to/subtitles/

# 轉換並保留原檔案
subx-cli convert --keep-original subtitle.vtt --format srt

# 進階：混合檔案和目錄，指定編碼
subx-cli convert -i movie1.srt -i ./batch_dir -i movie2.ass --format srt --recursive --keep-original --encoding utf-8
```

**時間軸校正**

```bash
# 自動 VAD 同步（需要音訊 / 影片檔案）
subx-cli sync video.mp4 subtitle.srt

# 手動同步（僅需字幕檔案）
subx-cli sync --offset 2.5 subtitle.srt

# 明確指定 VAD 方法並自訂靈敏度
subx-cli sync --vad-sensitivity 0.8 video.mp4 subtitle.srt

# 批次處理模式（處理整個目錄）
subx-cli sync --batch /path/to/media/folder

# 使用 -i 參數批次處理多個目錄
subx-cli sync -i ./movies_directory --batch

# 批次處理並遞迴掃描目錄
subx-cli sync -i ./movies_directory --batch --recursive

# 進階：多個目錄並指定同步方法
subx-cli sync -i ./movies1 -i ./movies2 -i ./tv_shows --recursive --batch --method vad

# 批次模式並顯示詳細輸出和 dry-run
subx-cli sync -i ./media --batch --recursive --dry-run --verbose
subx-cli sync movie.mkv
subx-cli sync subtitles.ass
subx-cli sync -b media_folder
```

**字元編碼檢測**
```bash
# 直接指定檔案
subx-cli detect-encoding *.srt

# 使用 -i 參數處理目錄（平面掃描）
subx-cli detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose

# 使用 -i 參數遞迴掃描目錄
subx-cli detect-encoding -i ./subtitles1 -i ./subtitles2 --verbose --recursive

# 進階：混合特定檔案與目錄掃描
subx-cli detect-encoding -i ./more_subtitles -i specific_file.srt --recursive --verbose
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

### 使用 -i 參數的進階工作流程
```bash
# 1. 處理多個目錄的不同來源
cd ~/Media/

# 2. 從多個輸入來源匹配並整理
subx-cli match -i ./Downloads/Movies -i ./Downloads/TV_Shows -i ./Backup/Subs --recursive --dry-run --copy
subx-cli match -i ./Downloads/Movies -i ./Downloads/TV_Shows -i ./Backup/Subs --recursive --copy

# 3. 批次轉換所有字幕格式為 SRT 並遞迴掃描
subx-cli convert -i ./Movies -i ./TV_Shows --format srt --recursive --keep-original

# 4. 批次同步所有媒體檔案
subx-cli sync -i ./Movies -i ./TV_Shows --batch --recursive --method vad

# 5. 檢查所有字幕檔案編碼
subx-cli detect-encoding -i ./Movies -i ./TV_Shows --recursive --verbose
```

### 檔案整理應用場景
```bash
# 場景 1：保留原始字幕位置，複製到影片資料夾
subx-cli match --recursive --copy /media/collection/

# 場景 1b：使用多個輸入來源進行複製操作
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --copy

# 場景 2：移動字幕到影片資料夾，清理原始位置
subx-cli match --recursive --move /media/collection/

# 場景 2b：使用多個輸入來源進行移動操作
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --move

# 場景 3：預覽檔案整理操作
subx-cli match --dry-run --copy --recursive /media/collection/

# 場景 3b：使用多個輸入來源預覽
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --dry-run --copy

# 場景 4：使用備份保護進行檔案整理
subx-cli match --move --backup --recursive /media/collection/

# 場景 4b：多個來源使用備份保護
subx-cli match -i /media/movies -i /media/tv_shows -i /backup/subtitles --recursive --move --backup

# 場景 5：進階 - 混合特定檔案與目錄
subx-cli match -i ./video1.mp4 -i ./subtitles_dir -i ./video2.mkv --recursive --copy --backup
```

### 資料夾結構範例
```
處理前（分散式結構）：
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
        └── 阿甘正傳. 繁中.srt

使用 --copy 選項處理後（AI 智慧匹配）：
media/
├── movies/
│   ├── Action/
│   │   ├── The.Matrix.1999.1080p.BluRay.mkv
│   │   ├── The.Matrix.1999.1080p.BluRay.srt           # AI 匹配 Matrix_EN_Sub.srt 並重新命名
│   │   └── The.Matrix.1999.1080p.BluRay.zh.srt        # AI 匹配 駭客任務_中文字幕.srt 並重新命名
│   └── Drama/
│       ├── Forrest.Gump.1994.720p.WEB-DL.mp4
│       ├── Forrest.Gump.1994.720p.WEB-DL.srt           # AI 匹配 ForrestGump_English.srt 並重新命名
│       └── Forrest.Gump.1994.720p.WEB-DL.zh.srt        # AI 匹配 阿甘正傳.繁中.srt 並重新命名
└── subtitles/                   # 原始檔案保留
    ├── english/
    │   ├── Matrix_EN_Sub.srt
    │   └── ForrestGump_English.srt
    └── chinese/
        ├── 駭客任務_中文字幕.srt
        └── 阿甘正傳.繁中.srt

使用 --move 選項處理後（AI 智慧匹配）：
media/
├── movies/
│   ├── Action/
│   │   ├── The.Matrix.1999.1080p.BluRay.mkv
│   │   ├── The.Matrix.1999.1080p.BluRay.srt           # AI 匹配並重新命名後移動
│   │   └── The.Matrix.1999.1080p.BluRay.zh.srt        # AI 匹配並重新命名後移動
│   └── Drama/
│       ├── Forrest.Gump.1994.720p.WEB-DL.mp4
│       ├── Forrest.Gump.1994.720p.WEB-DL.srt           # AI 匹配並重新命名後移動
│       └── Forrest.Gump.1994.720p.WEB-DL.zh.srt        # AI 匹配並重新命名後移動
└── subtitles/                   # 原始檔案已移除
    ├── english/                 # 空目錄
    └── chinese/
```

## 配置選項

SubX 支援透過環境變數和配置檔案進行全面配置。

### 快速配置
```bash
# 設定 OpenAI API Key
export OPENAI_API_KEY="your-api-key-here"

# 可選：自訂 OpenAI 端點
export OPENAI_BASE_URL="https://api.openai.com/v1"

# 或使用配置指令
subx-cli config set ai.api_key "your-api-key-here"
subx-cli config set ai.model "gpt-4.1-mini"
subx-cli config set ai.base_url "https://api.openai.com/v1"
subx-cli config set ai.temperature 0.3
subx-cli config set ai.retry_attempts 3
```

### 配置檔案位置
- Linux/macOS: `~/.config/subx/config.toml`
- Windows: `%APPDATA%\subx\config.toml`

詳細配置選項請參考 [配置指南](docs/configuration-guide.md)。

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
  - 格式設定: 預設輸出格式、樣式保留、編碼檢測信心度、預設編碼等
```

### `subx-cli detect-encoding` - 檔案編碼檢測
```
選項:
  <FILES>...             目標檔案路徑
  -v, --verbose          顯示詳細樣本文字

配置支援:
  - 格式設定: 編碼檢測信心度閾值、預設編碼回退等
```

### `subx-cli sync` - 時間軸校正
```
選項:
  <VIDEO>               影片檔案路徑 (支援 MP4、MKV/WebM、OGG、WAV 音訊輸入)
  <SUBTITLE>            字幕檔案路徑
  <PATHS>...            檔案或資料夾路徑 (位置參數)
  --offset <SECONDS>    手動指定偏移量 (不可超過 sync.max_offset_seconds 配置)
  --batch               批次處理模式
  --method <METHOD>     同步方法 (auto|vad，預設值: 來自 sync.default_method 配置)
  --vad-sensitivity <SENSITIVITY>    VAD 檢測靈敏度 (0.0-1.0，覆蓋配置)
  --vad-chunk-size <SIZE>           VAD 區塊大小 (覆蓋配置)

音訊格式支援:
  - MP4、MKV/WebM、OGG、WAV 容器 (自動轉碼為 WAV 進行分析)

配置支援:
  - 同步設定: 預設同步方法、最大偏移範圍等
  - VAD 處理: 靈敏度、區塊大小、採樣率、填充區塊、最小語音持續時間、語音合併間隔等
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

### Q: AI 匹配準確度不高怎麼辦？

A: 確保檔案名包含足夠的識別資訊（如劇名、季數、集數）。同時可以嘗試調整 `--confidence` 參數或配置 AI 模型溫度：`subx-cli config set ai.temperature 0.1`

### Q: 時間軸同步失敗？

A: 確保音訊 / 影片檔案可存取，並檢查檔案格式是否支援。如果 VAD 同步不理想，可以嘗試：
- 調整 VAD 靈敏度：`subx-cli config set sync.vad.sensitivity 0.8`（較高值適用於安靜音訊）
- 針對困難案例使用手動偏移：`subx-cli sync --offset <seconds> subtitle.srt`
- 檢查 VAD 配置：`subx-cli config set sync.vad.enabled true`
- 針對非常嘈雜的音訊：`subx-cli config set sync.vad.min_speech_duration_ms 200`
- 針對快速語音：`subx-cli config set sync.vad.speech_merge_gap_ms 100`
- 調整音訊處理參數：
  - `subx-cli config set sync.vad.chunk_size 512`
  - `subx-cli config set sync.vad.sample_rate 16000`
  - `subx-cli config set sync.vad.padding_chunks 3`

### Q: 處理大量檔案時性能不佳？

A: 可以調整並行處理配置：
```bash
subx-cli config set general.max_concurrent_jobs 8     # 增加並發數
subx-cli config set parallel.task_queue_size 2000    # 增加佇列大小
subx-cli config set parallel.auto_balance_workers true # 啟用負載平衡
subx-cli config set parallel.enable_task_priorities true # 啟用任務優先級
subx-cli config set parallel.max_workers 16          # 增加最大工作執行緒數
```

### Q: 編碼檢測不準確？

A: 調整檢測信心度閾值和預設編碼：
```bash
subx-cli config set formats.encoding_detection_confidence 0.8
subx-cli config set formats.default_encoding "utf-8"
```

### Q: 格式轉換問題或樣式問題？

A: 配置格式轉換設定：
```bash
subx-cli config set formats.default_output "srt"      # 設定預設輸出格式
subx-cli config set formats.preserve_styling true     # 轉換時保留樣式
```

### Q: 快取檔案佔用太多空間？

A: 使用 `subx-cli cache clear` 指令可以清除所有快取檔案。

### Q: 如何在新的影片與字幕加入後重新匹配？

A: 先清除快取 `subx-cli cache clear`，再重新執行 match 命令。

### Q: 任務執行逾時怎麼辦？

A: 增加逾時時間：`subx-cli config set general.task_timeout_seconds 7200`  # 設定為 2 小時

### Q: 檔案整理（複製 / 移動）操作失敗？

A: 檢查以下常見問題：
- 確保目標影片目錄具有寫入權限
- 檢查複製操作是否有足夠的磁碟空間
- 檔名衝突時系統會自動重新命名並加上數字後綴
- 使用 `--dry-run` 在執行前預覽操作：`subx-cli match --dry-run --copy /path`

### Q: 可以同時使用 --copy 和 --move 嗎？

A: 不可以，這兩個選項互斥。請選擇 `--copy` 保留原始檔案或 `--move` 移動檔案。

### Q: 為什麼有些字幕沒有被複製 / 移動到影片資料夾？

A: 複製 / 移動操作只在以下條件下執行：
- 字幕和影片檔案位於不同目錄
- AI 匹配信心度超過閾值（預設 80%）
- 目標位置不存在相同名稱的檔案
使用 `--dry-run` 查看將要執行的操作。

### Q: 如何處理複製 / 移動操作中的檔名衝突？

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
