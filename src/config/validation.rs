//! Low-level validation functions for individual configuration values.
//!
//! This module provides primitive validation functions that are used by
//! higher-level validation systems. These functions focus on validating
//! individual values and types without knowledge of the overall configuration
//! structure.
//!
//! # Architecture
//!
//! This module is the foundation of the validation system:
//! - [`crate::config::validation`] (this module) - Low-level validation functions
//! - [`crate::config::validator`] - High-level configuration section validators
//! - [`crate::config::field_validator`] - Key-value validation for configuration service

use crate::error::{SubXError, SubXResult};
use std::path::Path;
use url::Url;

/// Validate a string value against a list of allowed values.
pub fn validate_enum(value: &str, allowed: &[&str]) -> SubXResult<()> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(SubXError::config(format!(
            "Invalid value '{}'. Allowed values: {}",
            value,
            allowed.join(", ")
        )))
    }
}

/// Validate a float value within a specified range.
pub fn validate_float_range(value: &str, min: f32, max: f32) -> SubXResult<f32> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| SubXError::config(format!("Invalid float value: {value}")))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {parsed} is out of range [{min}, {max}]"
        )));
    }
    Ok(parsed)
}

/// Validate an unsigned integer within a specified range.
pub fn validate_uint_range(value: &str, min: u32, max: u32) -> SubXResult<u32> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| SubXError::config(format!("Invalid integer value: {value}")))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {parsed} is out of range [{min}, {max}]"
        )));
    }
    Ok(parsed)
}

/// Validate a u64 value within a specified range.
pub fn validate_u64_range(value: &str, min: u64, max: u64) -> SubXResult<u64> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| SubXError::config(format!("Invalid u64 value: {value}")))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {parsed} is out of range [{min}, {max}]"
        )));
    }
    Ok(parsed)
}

/// Validate a usize value within a specified range.
pub fn validate_usize_range(value: &str, min: usize, max: usize) -> SubXResult<usize> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| SubXError::config(format!("Invalid usize value: {value}")))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {parsed} is out of range [{min}, {max}]"
        )));
    }
    Ok(parsed)
}

/// Validate API key format.
pub fn validate_api_key(value: &str) -> SubXResult<()> {
    if value.is_empty() {
        return Err(SubXError::config("API key cannot be empty".to_string()));
    }
    if value.len() < 10 {
        return Err(SubXError::config("API key is too short".to_string()));
    }
    Ok(())
}

/// Validate URL format.
pub fn validate_url(value: &str) -> SubXResult<()> {
    if !value.starts_with("http://") && !value.starts_with("https://") {
        return Err(SubXError::config(format!(
            "Invalid URL format: {value}. Must start with http:// or https://"
        )));
    }
    Ok(())
}

/// Parse boolean value from string.
pub fn parse_bool(value: &str) -> SubXResult<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
        "false" | "0" | "no" | "off" | "disabled" => Ok(false),
        _ => Err(SubXError::config(format!("Invalid boolean value: {value}"))),
    }
}

/// Validate that a string is a valid URL.
///
/// # Arguments
/// * `value` - The string to validate as URL
///
/// # Errors
/// Returns error if the string is not a valid URL format.
pub fn validate_url_format(value: &str) -> SubXResult<()> {
    if value.trim().is_empty() {
        return Ok(()); // Empty URLs are often optional
    }

    Url::parse(value).map_err(|_| SubXError::config(format!("Invalid URL format: {value}")))?;
    Ok(())
}

/// Validate that a number is positive.
///
/// # Arguments
/// * `value` - The number to validate
///
/// # Errors
/// Returns error if the number is not positive.
pub fn validate_positive_number<T>(value: T) -> SubXResult<()>
where
    T: PartialOrd + Default + std::fmt::Display + Copy,
{
    if value <= T::default() {
        return Err(SubXError::config(format!(
            "Value must be positive, got: {value}"
        )));
    }
    Ok(())
}

/// Validate that a number is within a specified range.
///
/// # Arguments
/// * `value` - The value to validate
/// * `min` - Minimum allowed value (inclusive)
/// * `max` - Maximum allowed value (inclusive)
///
/// # Errors
/// Returns error if the value is outside the specified range.
pub fn validate_range<T>(value: T, min: T, max: T) -> SubXResult<()>
where
    T: PartialOrd + std::fmt::Display + Copy,
{
    if value < min || value > max {
        return Err(SubXError::config(format!(
            "Value {value} is outside allowed range [{min}, {max}]"
        )));
    }
    Ok(())
}

/// Validate that a string is not empty after trimming.
///
/// # Arguments
/// * `value` - The string to validate
/// * `field_name` - Name of the field for error messages
///
/// # Errors
/// Returns error if the string is empty or contains only whitespace.
pub fn validate_non_empty_string(value: &str, field_name: &str) -> SubXResult<()> {
    if value.trim().is_empty() {
        return Err(SubXError::config(format!("{field_name} cannot be empty")));
    }
    Ok(())
}

/// Validate that a path exists and is accessible.
///
/// # Arguments
/// * `value` - The path string to validate
/// * `must_exist` - Whether the path must already exist
///
/// # Errors
/// Returns error if path is invalid or doesn't exist when required.
pub fn validate_file_path(value: &str, must_exist: bool) -> SubXResult<()> {
    if value.trim().is_empty() {
        return Err(SubXError::config("File path cannot be empty"));
    }

    let path = Path::new(value);
    if must_exist && !path.exists() {
        return Err(SubXError::config(format!("Path does not exist: {value}")));
    }

    Ok(())
}

/// Validate temperature value for AI models.
///
/// # Arguments
/// * `temperature` - The temperature value to validate
///
/// # Errors
/// Returns error if temperature is outside the valid range (0.0-2.0).
pub fn validate_temperature(temperature: f32) -> SubXResult<()> {
    validate_range(temperature, 0.0, 2.0)
        .map_err(|_| SubXError::config("AI temperature must be between 0.0 and 2.0"))
}

/// Validate AI model name format.
///
/// # Arguments
/// * `model` - The model name to validate
///
/// # Errors
/// Returns error if model name is invalid.
pub fn validate_ai_model(model: &str) -> SubXResult<()> {
    validate_non_empty_string(model, "AI model")?;

    // Basic format validation - could be extended
    if model.len() > 100 {
        return Err(SubXError::config(
            "AI model name is too long (max 100 characters)",
        ));
    }

    Ok(())
}

/// Validate that a value is a power of two.
///
/// # Arguments
/// * `value` - The value to check
///
/// # Errors
/// Returns error if the value is not a power of two.
pub fn validate_power_of_two(value: usize) -> SubXResult<()> {
    if value == 0 || !value.is_power_of_two() {
        return Err(SubXError::config(format!(
            "Value {value} must be a power of two"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_format() {
        assert!(validate_url_format("https://api.openai.com").is_ok());
        assert!(validate_url_format("").is_ok()); // Empty is OK for optional fields
        assert!(validate_url_format("invalid-url").is_err());
        assert!(validate_url_format("ftp://example.com").is_ok()); // Different protocol is OK
    }

    #[test]
    fn test_validate_positive_number() {
        assert!(validate_positive_number(1.0).is_ok());
        assert!(validate_positive_number(0.0).is_err());
        assert!(validate_positive_number(-1.0).is_err());
        assert!(validate_positive_number(0.1).is_ok());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(1.5, 0.0, 2.0).is_ok());
        assert!(validate_range(0.0, 0.0, 2.0).is_ok()); // Boundary values OK
        assert!(validate_range(2.0, 0.0, 2.0).is_ok());
        assert!(validate_range(-0.1, 0.0, 2.0).is_err());
        assert!(validate_range(2.1, 0.0, 2.0).is_err());
    }

    #[test]
    fn test_validate_non_empty_string() {
        assert!(validate_non_empty_string("test", "field").is_ok());
        assert!(validate_non_empty_string("", "field").is_err());
        assert!(validate_non_empty_string("   ", "field").is_err()); // Whitespace only
        assert!(validate_non_empty_string(" test ", "field").is_ok());
    }

    #[test]
    fn test_validate_temperature() {
        assert!(validate_temperature(0.8).is_ok());
        assert!(validate_temperature(0.0).is_ok());
        assert!(validate_temperature(2.0).is_ok());
        assert!(validate_temperature(-0.1).is_err());
        assert!(validate_temperature(2.1).is_err());
    }

    #[test]
    fn test_validate_ai_model() {
        assert!(validate_ai_model("gpt-4").is_ok());
        assert!(validate_ai_model("").is_err());
        assert!(validate_ai_model(&"a".repeat(101)).is_err()); // Too long
        assert!(validate_ai_model(&"a".repeat(100)).is_ok()); // Max length OK
    }

    #[test]
    fn test_validate_power_of_two() {
        assert!(validate_power_of_two(1).is_ok());
        assert!(validate_power_of_two(2).is_ok());
        assert!(validate_power_of_two(4).is_ok());
        assert!(validate_power_of_two(256).is_ok());
        assert!(validate_power_of_two(1024).is_ok());
        assert!(validate_power_of_two(0).is_err());
        assert!(validate_power_of_two(3).is_err());
        assert!(validate_power_of_two(5).is_err());
    }

    #[test]
    fn test_validate_enum() {
        let allowed = &["openai", "anthropic"];
        assert!(validate_enum("openai", allowed).is_ok());
        assert!(validate_enum("anthropic", allowed).is_ok());
        assert!(validate_enum("invalid", allowed).is_err());
    }

    #[test]
    fn test_validate_float_range() {
        assert!(validate_float_range("1.5", 0.0, 2.0).is_ok());
        assert!(validate_float_range("0.0", 0.0, 2.0).is_ok());
        assert!(validate_float_range("2.0", 0.0, 2.0).is_ok());
        assert!(validate_float_range("-0.1", 0.0, 2.0).is_err());
        assert!(validate_float_range("2.1", 0.0, 2.0).is_err());
        assert!(validate_float_range("invalid", 0.0, 2.0).is_err());
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true").unwrap(), true);
        assert_eq!(parse_bool("false").unwrap(), false);
        assert_eq!(parse_bool("1").unwrap(), true);
        assert_eq!(parse_bool("0").unwrap(), false);
        assert_eq!(parse_bool("yes").unwrap(), true);
        assert_eq!(parse_bool("no").unwrap(), false);
        assert_eq!(parse_bool("enabled").unwrap(), true);
        assert_eq!(parse_bool("disabled").unwrap(), false);
        assert!(parse_bool("invalid").is_err());
    }
}
