# Bug #14: 配置整合測試並行執行競爭條件問題

## 問題描述

`test_full_config_integration` 測試在 CI 環境中間歇性失敗，主要問題是測試期望的 `max_sample_length` 值為 3000，但實際載入的值為 2000。錯誤訊息顯示測試建立了一個臨時配置檔案，但在載入配置時卻讀取了另一個不同路徑的檔案。

## 問題現象

### 失敗的測試案例
```
test test_full_config_integration ... FAILED

assertion `left == right` failed
  left: 2000
 right: 3000
```

### 根本原因分析

從 debug 日誌可以看出：

1. **建立的配置檔案路徑**：
   ```
   /var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpBMfNtQ/config.toml
   ```
   內容包含：`max_sample_length = 3000`

2. **實際載入的配置檔案路徑**：
   ```
   /var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpHBwKBu/config.toml
   ```
   內容包含不同的配置值

3. **問題核心**：環境變數 `SUBX_CONFIG_PATH` 的設定與實際載入的檔案路徑不一致

## 影響範圍

- **測試穩定性**：測試在 CI 環境中間歇性失敗
- **並行執行**：同時執行的其他測試可能影響全域環境變數
- **開發流程**：測試失敗影響 CI/CD 流水線

## 技術分析

### 問題根源

1. **全域狀態污染**：多個測試共享全域配置管理器和環境變數
2. **臨時檔案路徑衝突**：`tempfile::TempDir` 可能產生相似的路徑
3. **環境變數競爭**：`SUBX_CONFIG_PATH` 環境變數被多個測試同時修改
4. **配置管理器單例模式**：全域配置管理器在多執行緒環境下可能產生競爭條件

### 失敗場景重現

當以下條件同時發生時會觸發此問題：
- 多個配置相關測試並行執行
- 測試之間的清理邏輯不完整
- 全域配置管理器狀態未正確隔離

## 解決方案

### 測試隔離增強

#### 1.1 實作測試專用的配置管理器隔離
```rust
// 在測試中使用執行緒本地儲存或測試專用的配置管理器實例
thread_local! {
    static TEST_CONFIG_MANAGER: RefCell<Option<ConfigManager>> = RefCell::new(None);
}
```

#### 1.2 為測試檔案名稱添加隨機流水號
```rust
use uuid::Uuid;

fn create_test_config_file() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let unique_id = Uuid::new_v4().to_string()[..8].to_string();
    let config_path = temp_dir.path().join(format!("config_{}.toml", unique_id));
    (temp_dir, config_path)
}
```

#### 1.3 增強環境變數清理機制
```rust
fn setup_isolated_test_env() -> TestEnvironment {
    let backup_vars = backup_environment_vars();
    let (temp_dir, config_path) = create_test_config_file();
    
    TestEnvironment {
        temp_dir,
        config_path,
        backup_vars,
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        restore_environment_vars(&self.backup_vars);
        reset_global_config_manager();
    }
}
```

#### 1.4 添加相依套件
```toml
[dev-dependencies]
uuid = { version = "1.0", features = ["v4"] }
```

## 實作步驟

### 階段 1：基礎隔離機制 (2-3 小時)
1. **添加 `uuid` 相依**：用於產生唯一檔案名稱
2. **實作唯一檔案名稱**：使用 UUID 確保臨時檔案路徑唯一性
3. **增強環境變數清理**：在每個測試前後確實清理環境變數

### 階段 2：測試環境隔離 (4-6 小時)
1. **實作 `TestEnvironment` 結構**：提供完整的測試環境隔離
2. **改善測試輔助函數**：重構測試設定和清理邏輯
3. **添加配置管理器重置機制**：確保測試間的狀態隔離

### 階段 3：進階隔離優化 (1-2 天)
1. **實作執行緒本地配置管理器**：完全隔離測試環境
2. **添加更詳細的 debug 日誌**：協助追蹤和診斷問題
3. **添加綜合測試套件**：驗證並行執行的穩定性

## 驗證標準

### 測試穩定性驗證
```bash
# 重複執行測試 50 次確認穩定性
for i in {1..50}; do
    echo "Test run $i"
    cargo test config_integration_tests || exit 1
done
```

### 並行執行驗證
```bash
# 並行執行所有測試確認無競爭條件
cargo test --workspace --all-features -- --test-threads=8
```

### CI 環境驗證
- 在 macOS、Linux、Windows 三個平台上驗證修復效果
- 確認測試在 GitHub Actions 中連續執行 10 次無失敗

## 風險評估

### 低風險
- 添加 `uuid` 測試相依套件
- 改善測試檔案建立邏輯
- 增強環境變數清理機制

### 中風險
- 實作測試環境隔離機制
- 修改現有測試結構
- 添加執行緒本地儲存機制

### 高風險
- 修改全域配置管理器的核心邏輯
- 大幅重構現有測試架構

## 優先級

**高優先級** - 此問題影響 CI 穩定性，需要儘快解決以確保開發流程順暢。

## 相關資源

- [Rust 測試並行性文件](https://doc.rust-lang.org/book/ch11-02-running-tests.html#running-tests-in-parallel-or-consecutively)
- [tempfile crate 最佳實踐](https://docs.rs/tempfile/)
- [uuid crate 文件](https://docs.rs/uuid/)
- [Rust 執行緒本地儲存](https://doc.rust-lang.org/std/thread/struct.LocalKey.html)

## 後續追蹤

- 監控測試執行穩定性指標
- 建立測試並行執行的最佳實踐文件
- 考慮在其他整合測試中應用相同的隔離機制

## Raw log
```
     Running `/Users/runner/work/subx-cli/subx-cli/target/debug/deps/config_integration_tests-f3de106a2d21f8d7`

running 2 tests
test test_base_url_unified_config_integration ... ok
test test_full_config_integration ... FAILED

failures:

---- test_full_config_integration stdout ----
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Original config file path: "/var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpBMfNtQ/config.toml"
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Config file exists before canonicalize: true
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Config file content:
    
    [ai]
    provider = "openai"
    model = "gpt-4"
    max_sample_length = 3000
    
    [general]
    backup_enabled = true
    max_concurrent_jobs = 8
    
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Config file size: 124 bytes
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Final config path string: /private/var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpBMfNtQ/config.toml
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Setting SUBX_CONFIG_PATH environment variable
[2025-06-09T14:01:59Z DEBUG config_integration_tests] SUBX_CONFIG_PATH = /private/var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpBMfNtQ/config.toml
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Calling init_config_manager()
[2025-06-09T14:01:59Z DEBUG subx_cli::config] config_file_path: Checking SUBX_CONFIG_PATH environment variable
[2025-06-09T14:01:59Z DEBUG subx_cli::config] config_file_path: Using custom path from env: /private/var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpHBwKBu/config.toml
[2025-06-09T14:01:59Z DEBUG subx_cli::config] config_file_path: Custom path exists: true
[2025-06-09T14:01:59Z DEBUG subx_cli::config] init_config_manager: Using config path: "/private/var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpHBwKBu/config.toml"
[2025-06-09T14:01:59Z DEBUG subx_cli::config] init_config_manager: Config path exists: true
[2025-06-09T14:01:59Z DEBUG subx_cli::config] init_config_manager: Created manager with 3 sources
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Starting to load configuration
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Loading 3 sources in order
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Loading source 1 - 'file' (priority 10)
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: Attempting to load from path: "/private/var/folders/y6/nj790rtn62lfktb1sh__79hc0000gn/T/.tmpHBwKBu/config.toml"
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: Path exists: true
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: Read 81 bytes from file
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: File content:
    
    [ai]
    provider = "openai"
    model = "gpt-4"
    base_url = "https://api.custom.com/v1"
    
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: Parsed successfully
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: cfg.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: cfg.ai.model = Some("gpt-4")
[2025-06-09T14:01:59Z DEBUG subx_cli::config::source] FileSource: cfg.ai.provider = Some("openai")
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Source 'file' returned cfg.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: After merging 'file', merged.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Loading source 2 - 'environment' (priority 5)
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Source 'environment' returned cfg.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: After merging 'environment', merged.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Loading source 3 - 'cli' (priority 1)
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Source 'cli' returned cfg.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: After merging 'cli', merged.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config::manager] ConfigManager: Final stored config.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config] init_config_manager: Manager loaded successfully
[2025-06-09T14:01:59Z DEBUG subx_cli::config] init_config_manager: Updated global manager
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Calling load_config()
[2025-06-09T14:01:59Z DEBUG subx_cli::config] load_config: Getting global config manager
[2025-06-09T14:01:59Z DEBUG subx_cli::config] load_config: Locking manager
[2025-06-09T14:01:59Z DEBUG subx_cli::config] load_config: Getting partial config
[2025-06-09T14:01:59Z DEBUG subx_cli::config] load_config: partial_config.ai.max_sample_length = None
[2025-06-09T14:01:59Z DEBUG subx_cli::config] load_config: Converting to complete config
[2025-06-09T14:01:59Z DEBUG subx_cli::config] load_config: Final config.ai.max_sample_length = 2000
[2025-06-09T14:01:59Z DEBUG config_integration_tests] Loaded config values:
[2025-06-09T14:01:59Z DEBUG config_integration_tests]   config.ai.model = gpt-4
[2025-06-09T14:01:59Z DEBUG config_integration_tests]   config.ai.max_sample_length = 2000
[2025-06-09T14:01:59Z DEBUG config_integration_tests]   config.ai.provider = openai
[2025-06-09T14:01:59Z DEBUG config_integration_tests]   config.general.max_concurrent_jobs = 4
[2025-06-09T14:01:59Z DEBUG config_integration_tests]   config.ai.api_key = Some("env-api-key")

thread 'test_full_config_integration' panicked at tests/config_integration_tests.rs:109:5:
assertion `left == right` failed
  left: 2000
 right: 3000
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    test_full_config_integration

test result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test config_integration_tests`
Error: Process completed with exit code 101.
```
