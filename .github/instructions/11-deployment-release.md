# Product Backlog #11: 部署與發佈

## 領域範圍
編譯優化、跨平台支援、CI/CD、發佈流程

## 完成項目

### 1. 編譯配置優化
- [ ] Release 模式最佳化設定
- [ ] 二進位檔案大小優化
- [ ] 靜態連結配置
- [ ] 交叉編譯設定

### 2. 跨平台支援
- [ ] Linux (x86_64) 支援
- [ ] macOS (x86_64, ARM64) 支援
- [ ] Windows (x86_64) 支援
- [ ] 平台特定相依套件處理

### 3. CI/CD Pipeline
- [ ] GitHub Actions 工作流程
- [ ] 自動化測試執行
- [ ] 自動化編譯和打包
- [ ] 發佈流程自動化

### 4. 套件和發佈
- [ ] GitHub Releases 配置
- [ ] 預編譯二進位檔案
- [ ] Homebrew Formula
- [ ] Cargo 發佈準備

### 5. 文件和範例
- [ ] README 完善
- [ ] 使用指南撰寫
- [ ] API 文件生成
- [ ] 範例和教學

### 6. 品質保證
- [ ] 效能基準測試
- [ ] 記憶體使用分析
- [ ] 安全性檢查
- [ ] 相容性測試

## 技術設計

### Cargo.toml 最佳化配置

> 套件以實際使用到的為主，以下只是範例，實際情況可能會有所不同。

```toml
# Cargo.toml (完整版)
[package]
name = "subx-cli"
version = "0.1.0"
edition = "2021"
authors = ["CHEN, CHUN <jim60105@gmail.com>"]
description = "智慧字幕處理 CLI 工具，使用 AI 技術自動匹配、重命名和處理字幕檔案"
license = "GPLv3"
repository = "https://github.com/jim60105/subx-cli"
homepage = "https://github.com/jim60105/subx-cli"
documentation = "https://docs.rs/subx-cli"
readme = "README.md"
keywords = ["subtitle", "ai", "cli", "media", "sync"]
categories = ["command-line-utilities", "multimedia", "text-processing"]

[[bin]]
name = "subx-cli"
path = "src/main.rs"

[dependencies]
# CLI 框架
clap = { version = "4.0", features = ["derive", "env"] }
clap_complete = "4.0"

# 異步運行時
tokio = { version = "1.0", features = ["full"] }

# HTTP 客戶端
reqwest = { version = "0.11", features = ["json", "rustls-tls"], default-features = false }

# 序列化
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# 錯誤處理
anyhow = "1.0"
thiserror = "1.0"

# 檔案處理
walkdir = "2.0"
regex = "1.0"
encoding_rs = "0.8"

# 音訊處理
symphonia = { version = "0.5", features = ["aac", "mp3", "vorbis", "flac"] }
rustfft = "6.0"

# 用戶介面
indicatif = "0.17"
colored = "2.0"
dialoguer = "0.10"

# 配置管理
dirs = "5.0"

# 日誌
log = "0.4"
env_logger = "0.10"

# 實用工具
futures = "0.3"
async-trait = "0.1"

[dev-dependencies]
tokio-test = "0.4"
tempfile = "3.0"
mockall = "0.11"
criterion = { version = "0.5", features = ["html_reports"] }

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true

[profile.dev]
opt-level = 0
debug = true
split-debuginfo = "unpacked"

# 編譯目標配置
[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser"] }

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[[bench]]
name = "audio_processing"
harness = false

[[bench]]
name = "subtitle_parsing"
harness = false
```

### GitHub Actions CI/CD
```yaml
# .github/workflows/ci.yml
name: CI

on:
  push:
    branches: [ master ]
  pull_request:
    branches: [ master ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [stable]

    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: ${{ matrix.rust }}
        profile: minimal
        override: true
        components: rustfmt, clippy
    
    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}
    
    - name: Check formatting
      run: cargo fmt -- --check
    
    - name: Check clippy
      run: cargo clippy -- -D warnings
    
    - name: Run tests
      run: cargo test --verbose
    
    - name: Run integration tests
      run: cargo test --test '*' --verbose

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/audit@v1

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
        components: llvm-tools-preview
    
    - name: Install cargo-llvm-cov
      uses: taiki-e/install-action@cargo-llvm-cov
    
    - name: Generate code coverage
      run: cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
    
    - name: Upload to codecov.io
      uses: codecov/codecov-action@v3
      with:
        files: lcov.info
        fail_ci_if_error: false
```

### 發佈工作流程
```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  create-release:
    name: Create Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
    - name: Create Release
      id: create_release
      uses: actions/create-release@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        tag_name: ${{ github.ref }}
        release_name: Release ${{ github.ref }}
        draft: false
        prerelease: false

  build:
    name: Build and Upload
    needs: create-release
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
        - os: ubuntu-latest
          target: x86_64-unknown-linux-gnu
          binary_name: subx
          asset_name: subx-linux-x86_64
        - os: windows-latest
          target: x86_64-pc-windows-msvc
          binary_name: subx.exe
          asset_name: subx-windows-x86_64.exe
        - os: macos-latest
          target: x86_64-apple-darwin
          binary_name: subx
          asset_name: subx-macos-x86_64
        - os: macos-latest
          target: aarch64-apple-darwin
          binary_name: subx
          asset_name: subx-macos-aarch64

    steps:
    - uses: actions/checkout@v3
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        target: ${{ matrix.target }}
        override: true
    
    - name: Build
      run: cargo build --release --target ${{ matrix.target }}
    
    - name: Package
      shell: bash
      run: |
        if [[ "${{ matrix.os }}" == "windows-latest" ]]; then
          cp target/${{ matrix.target }}/release/${{ matrix.binary_name }} ${{ matrix.asset_name }}
        else
          cp target/${{ matrix.target }}/release/${{ matrix.binary_name }} ${{ matrix.asset_name }}
          chmod +x ${{ matrix.asset_name }}
        fi
    
    - name: Upload Release Asset
      uses: actions/upload-release-asset@v1
      env:
        GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
      with:
        upload_url: ${{ needs.create-release.outputs.upload_url }}
        asset_path: ./${{ matrix.asset_name }}
        asset_name: ${{ matrix.asset_name }}
        asset_content_type: application/octet-stream

  publish-crates:
    name: Publish to crates.io
    needs: build
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        override: true
    - name: Publish to crates.io
      run: cargo publish --token ${{ secrets.CRATES_TOKEN }}
```

### 效能基準測試
```rust
// benches/audio_processing.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use subx::services::audio::AudioAnalyzer;
use std::path::Path;

fn bench_audio_envelope_extraction(c: &mut Criterion) {
    let analyzer = AudioAnalyzer::new(16000);
    let test_audio_path = Path::new("test_data/sample.mp4");
    
    if test_audio_path.exists() {
        c.bench_function("audio_envelope_extraction", |b| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    analyzer.extract_envelope(black_box(test_audio_path)).await
                })
            })
        });
    }
}

fn bench_correlation_calculation(c: &mut Criterion) {
    let audio_data = vec![0.5f32; 16000]; // 1秒 16kHz
    let subtitle_data = vec![1.0f32; 16000];
    
    c.bench_function("correlation_calculation", |b| {
        b.iter(|| {
            // 模擬相關係數計算
            let mut sum = 0.0;
            for (a, s) in audio_data.iter().zip(subtitle_data.iter()) {
                sum += a * s;
            }
            black_box(sum)
        })
    });
}

criterion_group!(benches, bench_audio_envelope_extraction, bench_correlation_calculation);
criterion_main!(benches);
```

### 安裝指令
```bash
# scripts/install.sh
#!/bin/bash

set -e

# 檢測作業系統和架構
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
    x86_64) ARCH="x86_64" ;;
    arm64|aarch64) ARCH="aarch64" ;;
    *) echo "不支援的架構: $ARCH"; exit 1 ;;
esac

case $OS in
    linux) PLATFORM="linux" ;;
    darwin) PLATFORM="macos" ;;
    *) echo "不支援的作業系統: $OS"; exit 1 ;;
esac

# 下載最新版本
RELEASE_URL="https://api.github.com/repos/jim60105/subx-cli/releases/latest"
BINARY_NAME="subx-${PLATFORM}-${ARCH}"

echo "正在下載 SubX 最新版本..."
curl -L "$(curl -s $RELEASE_URL | grep "browser_download_url.*$BINARY_NAME" | cut -d '"' -f 4)" -o subx

chmod +x subx

# 安裝到系統路徑
if [[ "$EUID" -eq 0 ]]; then
    mv subx /usr/local/bin/
    echo "SubX 已安裝到 /usr/local/bin/subx"
else
    sudo mv subx /usr/local/bin/
    echo "SubX 已安裝到 /usr/local/bin/subx"
fi

echo "安裝完成! 執行 'subx --help' 開始使用"
```

### Homebrew Formula
```ruby
# Formula/subx.rb
class Subx < Formula
  desc "智慧字幕處理 CLI 工具"
  homepage "https://github.com/jim60105/subx-cli"
  url "https://github.com/jim60105/subx-cli/archive/v0.1.0.tar.gz"
  sha256 "YOUR_SHA256_HERE"
  license "GPLv3"

  depends_on "rust" => :build
  depends_on "ffmpeg" => :optional

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/subx", "--version"
  end
end
```

## 驗收標準
1. 所有平台編譯成功
2. CI/CD 流程正常運作
3. 發佈流程自動化
4. 效能基準測試通過
5. 安裝指令正常運作

## 估計工時
3-4 天

## 相依性
- 依賴 Backlog #09 (命令整合與測試)

## 風險評估
- 低風險：部署和發佈是標準流程
- 注意事項：跨平台相容性、自動化流程穩定性
