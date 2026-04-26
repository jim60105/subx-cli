# SubX-CLI

<div align="center">
  <img src="assets/logo.svg" alt="SubX CLI Logo" width="800" height="300">

[![Build, Test, Audit & Coverage](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/build-test-audit-coverage.yml) [![Release](https://github.com/jim60105/subx-cli/actions/workflows/release.yml/badge.svg)](https://github.com/jim60105/subx-cli/actions/workflows/release.yml) [![crates.io](https://img.shields.io/crates/v/subx-cli.svg)](https://crates.io/crates/subx-cli) [![docs.rs](https://docs.rs/subx-cli/badge.svg)](https://docs.rs/subx-cli) [![codecov](https://codecov.io/gh/jim60105/subx-cli/graph/badge.svg?token=2C53RSNNAL)](https://codecov.io/gh/jim60105/subx-cli)

[English](./README.md) | 中文

AI 字幕處理命令列工具，自動匹配、重新命名、格式轉換及時間軸校正。

</div>

## SubX 能做什麼

將 SubX 指向一個媒體資料夾，它會透過 AI 分析判斷哪些字幕檔案對應哪些影片，即使檔名使用不同語言且沒有共同命名規則。

```
處理前：
media/
├── movies/
│   └── The.Matrix.1080p.mkv
└── subtitles/
    ├── Matrix_EN_Sub.srt
    └── 駭客任務_中文字幕.srt

執行 subx-cli match --copy media/ 後：
media/
├── movies/
│   ├── The.Matrix.1080p.mkv
│   ├── The.Matrix.1080p.srt         ← AI 匹配 Matrix_EN_Sub.srt
│   └── The.Matrix.1080p.zh.srt      ← AI 匹配 駭客任務_中文字幕.srt
└── subtitles/                         （保留原始檔案）
    ├── Matrix_EN_Sub.srt
    └── 駭客任務_中文字幕.srt
```

除了匹配功能，SubX 也支援字幕格式互轉（SRT、ASS、VTT、SUB），透過本地語音活動偵測（VAD）校正時間軸偏移，偵測檔案字元編碼，並可運用設定的 AI 供應商將字幕內容翻譯為其他語言，同時保留時間軸與字幕順序。

## 快速開始

```bash
# 安裝
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/master/scripts/install.sh | bash

# 設定（使用 OpenRouter 免費 DeepSeek 模型）
export OPENROUTER_API_KEY="<YOUR_API_KEY>"
subx-cli config set ai.provider openrouter
subx-cli config set ai.model "deepseek/deepseek-r1-0528:free"

# 預覽 SubX 將執行的操作
subx-cli match --dry-run --copy /path/to/media/

# 執行
subx-cli match --copy /path/to/media/
```

使用 OpenAI 時改設 `OPENAI_API_KEY`。使用 Azure OpenAI 時需設定 `AZURE_OPENAI_API_KEY` 和 `AZURE_OPENAI_ENDPOINT`。所有供應商選項與環境變數請參閱[配置指南](docs/configuration-guide.md)。

### 使用本地 LLM 執行（Ollama、LM Studio、llama.cpp、vLLM）

想讓字幕完全留在本機？只要選擇 `local` 供應商，就能將 SubX 指向任何相容 OpenAI 介面的本地執行環境——包含 Ollama、LM Studio、llama.cpp 的 `llama-server`、vLLM，或任何支援 `POST /chat/completions` 協定的伺服器。`ollama` 為合法的別名，會在寫入設定時自動正規化為 `local`。一般本地佈署毋須提供 API 金鑰；當選用 `local` 時，SubX 只會連線至設定中的 `base_url`，並忽略所有託管供應商的環境變數（`OPENAI_API_KEY`、`OPENROUTER_API_KEY`、`AZURE_OPENAI_*`）。

```bash
# 以預設連接埠執行 Ollama
subx-cli config set ai.provider local
subx-cli config set ai.base_url "http://localhost:11434/v1"
subx-cli config set ai.model "llama3.1:8b-instruct"
```

端點可以是 loopback、區域網路主機（`http://192.168.x.x:port/v1`）、Tailscale／VPN 主機（`https://host.tailnet.ts.net/v1`），或任何可連線的相容 OpenAI 介面 URL。`local` 同時接受 `http://` 與 `https://`。各執行環境的範例與已知相容性限制請見[本地 / 離線 LLM 供應商](docs/configuration-guide.md#local--offline-llm-provider)章節。

## 命令一覽

`match`、`convert`、`sync`、`detect-encoding` 與 `translate` 命令支援 `-i` 指定多個輸入來源，以及 `--recursive` 遞迴掃描子目錄。完整選項、範例與工作流程請參閱[命令參考](docs/command-reference.md)。

| 命令 | 用途 | 範例 |
|------|------|------|
| `match` | AI 匹配字幕與影片，重新命名／複製／移動 | `subx-cli match --copy ./media` |
| `convert` | 字幕格式互轉 | `subx-cli convert --format srt ./subs/` |
| `sync` | 透過 VAD 或手動偏移校正時間軸 | `subx-cli sync video.mp4 subtitle.srt` |
| `detect-encoding` | 偵測字幕檔案字元編碼 | `subx-cli detect-encoding *.srt` |
| `translate` | 以 AI 將字幕文字翻譯為其他語言 | `subx-cli translate movie.srt --target-language zh-TW` |
| `config` | 檢視及修改設定 | `subx-cli config set ai.provider openai` |
| `cache` | 管理 dry-run 結果快取 | `subx-cli cache clear` |

## 安裝

### Linux / macOS

```bash
# 一鍵安裝腳本
curl -fsSL https://raw.githubusercontent.com/jim60105/subx-cli/master/scripts/install.sh | bash

# 或直接下載執行檔
curl -L "https://github.com/jim60105/subx-cli/releases/latest/download/subx-linux-x86_64" -o subx-cli
chmod +x subx-cli && sudo mv subx-cli /usr/local/bin/
```

### 從原始碼建構（所有平台）

```bash
# 從 crates.io 安裝
cargo install subx-cli

# 或從儲存庫編譯
git clone https://github.com/jim60105/subx-cli.git
cd subx-cli
cargo build --release
```

## 支援格式

| 格式 | 讀取 | 寫入 | 說明 |
|------|------|------|------|
| SRT | ✅ | ✅ | SubRip，最廣泛支援的格式 |
| ASS | ✅ | ✅ | Advanced SubStation Alpha，豐富的樣式功能 |
| VTT | ✅ | ✅ | WebVTT，網頁原生格式 |
| SUB | ✅ | ⚠️ | 多種 SUB 變體，部分寫入支援 |

## 腳本與自動化

於子命令之前傳入 `--output json`，或設定 `SUBX_OUTPUT=json` 環境變數，即可讓所有支援的子命令在標準輸出上產生穩定且版本化的 JSON 封套，適合用於 Shell 腳本、CI 流水線與第三方工具。JSON 模式下會抑制進度條與狀態符號；既有的離開碼（exit code）契約於兩種模式中皆保持不變。

```bash
# 以 jq 取出第一筆比對候選的信心分數
subx-cli --output json match --dry-run ./media \
  | jq -r '.data.candidates[0].confidence'
```

完整的封套結構、錯誤分類、各命令的 payload 結構與腳本範例，請參閱[機器可讀輸出](docs/machine-readable-output.md)。`generate-completion` 是唯一明確拒絕 JSON 模式的子命令。

## 文件

- [命令參考](docs/command-reference.md)——所有子命令的完整選項、範例與工作流程
- [配置指南](docs/configuration-guide.md)——所有設定、環境變數與疑難排解
- [機器可讀輸出](docs/machine-readable-output.md)——`--output json` 契約，提供腳本與自動化使用
- [技術架構](docs/tech-architecture.md)——程式碼結構與設計決策
- [AI 供應商整合指南](docs/ai-provider-integration-guide.md)——如何新增 AI 供應商

## 授權條款

### GPLv3

<img src="https://github.com/user-attachments/assets/8712a047-a117-458d-9c56-cbd3d0e622d8" alt="gplv3" width="300" />

[GNU GENERAL PUBLIC LICENSE Version 3](LICENSE)

Copyright (C) 2025 Jim Chen <Jim@ChenJ.im>.

本程式為自由軟體：您可以依據由自由軟體基金會發布的 GNU 通用公共授權條款（第 3 版，或您選擇的任何後續版本）重新發佈及／或修改本程式。

本程式以期望其有用而發佈，但不提供任何保證；甚至不包含對適銷性或特定用途適用性的默示保證。詳情請參閱 GNU 通用公共授權條款。

您應已隨本程式收到一份 GNU 通用公共授權條款副本。如果沒有，請參見 <https://www.gnu.org/licenses/>。
