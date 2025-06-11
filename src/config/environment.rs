//! 環境變數提供者模組
//!
//! 此模組定義了抽象環境變數存取的特徵，以及對應的生產與測試實作。

use std::collections::HashMap;

/// 環境變數提供者特徵
///
/// 此特徵抽象了環境變數的存取，允許在測試中注入模擬實作
pub trait EnvironmentProvider: Send + Sync {
    /// 取得指定環境變數的值
    ///
    /// # 參數
    /// * `key` - 環境變數名稱
    ///
    /// # 回傳值
    /// 如果環境變數存在且有效，回傳 `Some(value)`，否則回傳 `None`
    fn get_var(&self, key: &str) -> Option<String>;

    /// 檢查環境變數是否存在
    ///
    /// # 參數
    /// * `key` - 環境變數名稱
    ///
    /// # 回傳值
    /// 如果環境變數存在，回傳 `true`，否則回傳 `false`
    fn has_var(&self, key: &str) -> bool {
        self.get_var(key).is_some()
    }
}

/// 系統環境變數提供者實作
///
/// 此實作直接讀取系統環境變數，用於生產環境
#[derive(Debug, Default)]
pub struct SystemEnvironmentProvider;

impl SystemEnvironmentProvider {
    /// 建立新的系統環境變數提供者
    pub fn new() -> Self {
        Self
    }
}

impl EnvironmentProvider for SystemEnvironmentProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// 測試環境變數提供者實作
///
/// 此實作使用預設的變數映射，用於測試環境的完全隔離
#[derive(Debug)]
pub struct TestEnvironmentProvider {
    variables: HashMap<String, String>,
}

impl TestEnvironmentProvider {
    /// 建立新的測試環境變數提供者
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// 建立包含指定變數的測試提供者
    ///
    /// # 參數
    /// * `variables` - 環境變數映射
    pub fn with_variables(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }

    /// 設定環境變數
    ///
    /// # 參數
    /// * `key` - 環境變數名稱
    /// * `value` - 環境變數值
    pub fn set_var(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    /// 移除環境變數
    ///
    /// # 參數
    /// * `key` - 環境變數名稱
    pub fn remove_var(&mut self, key: &str) {
        self.variables.remove(key);
    }

    /// 清除所有環境變數
    pub fn clear(&mut self) {
        self.variables.clear();
    }
}

impl EnvironmentProvider for TestEnvironmentProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        self.variables.get(key).cloned()
    }
}

impl Default for TestEnvironmentProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_environment_provider_existing_var() {
        let provider = SystemEnvironmentProvider::new();
        // 使用通常存在的環境變數進行測試
        let path = provider.get_var("PATH");
        assert!(path.is_some());
        assert!(!path.unwrap().is_empty());
    }

    #[test]
    fn test_system_environment_provider_non_existing_var() {
        let provider = SystemEnvironmentProvider::new();
        let result = provider.get_var("NON_EXISTING_VAR_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_test_environment_provider_empty() {
        let provider = TestEnvironmentProvider::new();
        assert!(provider.get_var("ANY_VAR").is_none());
        assert!(!provider.has_var("ANY_VAR"));
    }

    #[test]
    fn test_test_environment_provider_with_variables() {
        let mut vars = HashMap::new();
        vars.insert("TEST_VAR".to_string(), "test_value".to_string());
        let provider = TestEnvironmentProvider::with_variables(vars);
        assert_eq!(provider.get_var("TEST_VAR"), Some("test_value".to_string()));
        assert!(provider.has_var("TEST_VAR"));
        assert!(provider.get_var("OTHER_VAR").is_none());
    }

    #[test]
    fn test_test_environment_provider_set_and_remove() {
        let mut provider = TestEnvironmentProvider::new();
        provider.set_var("DYNAMIC_VAR", "dynamic_value");
        assert_eq!(
            provider.get_var("DYNAMIC_VAR"),
            Some("dynamic_value".to_string())
        );
        provider.remove_var("DYNAMIC_VAR");
        assert!(provider.get_var("DYNAMIC_VAR").is_none());
    }

    #[test]
    fn test_test_environment_provider_clear() {
        let mut provider = TestEnvironmentProvider::new();
        provider.set_var("VAR1", "value1");
        provider.set_var("VAR2", "value2");
        provider.clear();
        assert!(provider.get_var("VAR1").is_none());
        assert!(provider.get_var("VAR2").is_none());
    }
}
