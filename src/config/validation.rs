//! Configuration value validation utilities.
//!
//! This module provides comprehensive validation for configuration values,
//! ensuring type safety and constraint compliance.

use crate::error::{SubXError, SubXResult};

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
        .map_err(|_| SubXError::config(format!("Invalid float value: {}", value)))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    Ok(parsed)
}

/// Validate an unsigned integer within a specified range.
pub fn validate_uint_range(value: &str, min: u32, max: u32) -> SubXResult<u32> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| SubXError::config(format!("Invalid integer value: {}", value)))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    Ok(parsed)
}

/// Validate a u64 value within a specified range.
pub fn validate_u64_range(value: &str, min: u64, max: u64) -> SubXResult<u64> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| SubXError::config(format!("Invalid u64 value: {}", value)))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
        )));
    }
    Ok(parsed)
}

/// Validate a usize value within a specified range.
pub fn validate_usize_range(value: &str, min: usize, max: usize) -> SubXResult<usize> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| SubXError::config(format!("Invalid usize value: {}", value)))?;
    if parsed < min || parsed > max {
        return Err(SubXError::config(format!(
            "Value {} is out of range [{}, {}]",
            parsed, min, max
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
            "Invalid URL format: {}. Must start with http:// or https://",
            value
        )));
    }
    Ok(())
}

/// Parse boolean value from string.
pub fn parse_bool(value: &str) -> SubXResult<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => Ok(true),
        "false" | "0" | "no" | "off" | "disabled" => Ok(false),
        _ => Err(SubXError::config(format!(
            "Invalid boolean value: {}",
            value
        ))),
    }
}
