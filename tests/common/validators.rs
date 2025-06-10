//! 輸出驗證工具，提供靈活的測試輸出驗證功能。

use regex::Regex;
use std::fmt;

/// 驗證結果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    successes: Vec<String>,
    failures: Vec<String>,
}

impl ValidationResult {
    /// 建立新的驗證結果
    pub fn new() -> Self {
        Self {
            successes: Vec::new(),
            failures: Vec::new(),
        }
    }

    /// 新增成功項目
    pub fn add_success(&mut self, message: String) {
        self.successes.push(message);
    }

    /// 新增失敗項目
    pub fn add_failure(&mut self, message: String) {
        self.failures.push(message);
    }

    /// 檢查是否有效
    pub fn is_valid(&self) -> bool {
        self.failures.is_empty()
    }

    /// 取得成功項目
    pub fn successes(&self) -> &[String] {
        &self.successes
    }

    /// 取得失敗項目
    pub fn failures(&self) -> &[String] {
        &self.failures
    }

    /// 取得失敗數量
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }

    /// 取得成功數量
    pub fn success_count(&self) -> usize {
        self.successes.len()
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_valid() {
            write!(
                f,
                "✓ All validations passed ({} checks)",
                self.success_count()
            )
        } else {
            writeln!(f, "✗ Validation failed ({} errors):", self.failure_count())?;
            for failure in &self.failures {
                writeln!(f, "  - {}", failure)?;
            }
            Ok(())
        }
    }
}

/// 輸出驗證器，使用正規表達式模式進行驗證
pub struct OutputValidator {
    patterns: Vec<Regex>,
    anti_patterns: Vec<Regex>,
    pattern_descriptions: Vec<String>,
    anti_pattern_descriptions: Vec<String>,
}

impl OutputValidator {
    /// 建立新的輸出驗證器
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            anti_patterns: Vec::new(),
            pattern_descriptions: Vec::new(),
            anti_pattern_descriptions: Vec::new(),
        }
    }

    /// 新增期望模式
    pub fn expect_pattern(mut self, pattern: &str) -> Self {
        match Regex::new(pattern) {
            Ok(regex) => {
                self.patterns.push(regex);
                self.pattern_descriptions.push(pattern.to_string());
            }
            Err(e) => panic!("Invalid regex pattern '{}': {}", pattern, e),
        }
        self
    }

    /// 新增拒絕模式
    pub fn reject_pattern(mut self, pattern: &str) -> Self {
        match Regex::new(pattern) {
            Ok(regex) => {
                self.anti_patterns.push(regex);
                self.anti_pattern_descriptions.push(pattern.to_string());
            }
            Err(e) => panic!("Invalid regex pattern '{}': {}", pattern, e),
        }
        self
    }

    /// 期望包含特定字串
    pub fn expect_contains(self, text: &str) -> Self {
        self.expect_pattern(&regex::escape(text))
    }

    /// 拒絕包含特定字串
    pub fn reject_contains(self, text: &str) -> Self {
        self.reject_pattern(&regex::escape(text))
    }

    /// 期望行數
    pub fn expect_line_count(self, count: usize) -> Self {
        self.expect_pattern(&format!(r"^(?:[^\n]*\n){{{count}}}[^\n]*$", count = count))
    }

    /// 期望空輸出
    pub fn expect_empty(self) -> Self {
        self.expect_pattern(r"^\s*$")
    }

    /// 期望非空輸出
    pub fn expect_not_empty(self) -> Self {
        self.reject_pattern(r"^\s*$")
    }

    /// 驗證輸出
    pub fn validate(&self, output: &str) -> ValidationResult {
        let mut result = ValidationResult::new();

        // 檢查期望模式
        for (i, pattern) in self.patterns.iter().enumerate() {
            let description = &self.pattern_descriptions[i];
            if pattern.is_match(output) {
                result.add_success(format!("Found expected pattern: {}", description));
            } else {
                result.add_failure(format!("Missing expected pattern: {}", description));
            }
        }

        // 檢查拒絕模式
        for (i, pattern) in self.anti_patterns.iter().enumerate() {
            let description = &self.anti_pattern_descriptions[i];
            if pattern.is_match(output) {
                result.add_failure(format!("Found rejected pattern: {}", description));
            } else {
                result.add_success(format!("Successfully avoided pattern: {}", description));
            }
        }

        result
    }

    /// 驗證並斷言成功
    pub fn validate_and_assert(&self, output: &str) {
        let result = self.validate(output);
        if !result.is_valid() {
            panic!(
                "Output validation failed:\n{}\n\nActual output:\n{}",
                result, output
            );
        }
    }
}

impl Default for OutputValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_success() {
        let mut result = ValidationResult::new();
        result.add_success("Test passed".to_string());
        assert!(result.is_valid());
        assert_eq!(result.success_count(), 1);
        assert_eq!(result.failure_count(), 0);
    }

    #[test]
    fn test_validation_result_failure() {
        let mut result = ValidationResult::new();
        result.add_failure("Test failed".to_string());
        assert!(!result.is_valid());
        assert_eq!(result.success_count(), 0);
        assert_eq!(result.failure_count(), 1);
    }

    #[test]
    fn test_output_validator_expect_pattern() {
        let validator = OutputValidator::new().expect_pattern(r"success");

        let result = validator.validate("Operation completed successfully");
        assert!(result.is_valid());
    }

    #[test]
    fn test_output_validator_reject_pattern() {
        let validator = OutputValidator::new().reject_pattern(r"error");

        let result = validator.validate("Operation completed successfully");
        assert!(result.is_valid());

        let result = validator.validate("Operation failed with error");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_output_validator_expect_contains() {
        let validator = OutputValidator::new().expect_contains("success");

        let result = validator.validate("Operation completed successfully");
        assert!(result.is_valid());
    }

    #[test]
    fn test_output_validator_expect_empty() {
        let validator = OutputValidator::new().expect_empty();

        let result = validator.validate("");
        assert!(result.is_valid());

        let result = validator.validate("   \n  ");
        assert!(result.is_valid());

        let result = validator.validate("not empty");
        assert!(!result.is_valid());
    }

    #[test]
    fn test_output_validator_expect_not_empty() {
        let validator = OutputValidator::new().expect_not_empty();

        let result = validator.validate("some content");
        assert!(result.is_valid());

        let result = validator.validate("");
        assert!(!result.is_valid());
    }

    #[test]
    #[should_panic(expected = "Output validation failed")]
    fn test_output_validator_assert_fails() {
        let validator = OutputValidator::new().expect_contains("success");

        validator.validate_and_assert("Operation failed");
    }
}
