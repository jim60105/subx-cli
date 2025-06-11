//! Environment variable provider module.
//!
//! This module defines traits for abstracting environment variable access,
//! along with corresponding production and test implementations.

use std::collections::HashMap;

/// Environment variable provider trait.
///
/// This trait abstracts environment variable access, allowing for mock implementations
/// to be injected during testing.
pub trait EnvironmentProvider: Send + Sync {
    /// Get the value of the specified environment variable.
    ///
    /// # Arguments
    /// * `key` - Environment variable name
    ///
    /// # Returns
    /// Returns `Some(value)` if the environment variable exists and is valid,
    /// otherwise returns `None`.
    fn get_var(&self, key: &str) -> Option<String>;

    /// Check if an environment variable exists.
    ///
    /// # Arguments
    /// * `key` - Environment variable name
    ///
    /// # Returns
    /// Returns `true` if the environment variable exists, otherwise `false`.
    fn has_var(&self, key: &str) -> bool {
        self.get_var(key).is_some()
    }
}

/// System environment variable provider implementation.
///
/// This implementation directly reads system environment variables,
/// intended for use in production environments.
#[derive(Debug, Default)]
pub struct SystemEnvironmentProvider;

impl SystemEnvironmentProvider {
    /// Create a new system environment variable provider.
    pub fn new() -> Self {
        Self
    }
}

impl EnvironmentProvider for SystemEnvironmentProvider {
    fn get_var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

/// Test environment variable provider implementation.
///
/// This implementation uses a predefined variable mapping,
/// intended for complete isolation in test environments.
#[derive(Debug)]
pub struct TestEnvironmentProvider {
    variables: HashMap<String, String>,
}

impl TestEnvironmentProvider {
    /// Create a new test environment variable provider.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Create a test provider containing specified variables.
    ///
    /// # Arguments
    /// * `variables` - Environment variable mapping
    pub fn with_variables(variables: HashMap<String, String>) -> Self {
        Self { variables }
    }

    /// Set an environment variable.
    ///
    /// # Arguments
    /// * `key` - Environment variable name
    /// * `value` - Environment variable value
    pub fn set_var(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    /// Remove an environment variable.
    ///
    /// # Arguments
    /// * `key` - Environment variable name
    pub fn remove_var(&mut self, key: &str) {
        self.variables.remove(key);
    }

    /// Clear all environment variables.
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
        // Test using a commonly existing environment variable
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
