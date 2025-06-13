//! Tests for configuration value validation functions.

use subx_cli::config::validation::*;
// use subx_cli::error::SubXError;  // removed unused import

#[test]
fn test_validate_enum_success() {
    let result = validate_enum("openai", &["openai", "anthropic", "local"]);
    assert!(result.is_ok());
}

#[test]
fn test_validate_enum_failure() {
    let result = validate_enum("invalid", &["openai", "anthropic", "local"]);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid value 'invalid'")
    );
}

#[test]
fn test_validate_float_range_success() {
    let result = validate_float_range("0.5", 0.0, 1.0);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 0.5);
}

#[test]
fn test_validate_float_range_out_of_bounds() {
    let result = validate_float_range("1.5", 0.0, 1.0);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of range"));
}

#[test]
fn test_validate_float_range_invalid_format() {
    let result = validate_float_range("not_a_number", 0.0, 1.0);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid float value")
    );
}

#[test]
fn test_validate_uint_range_success() {
    let result = validate_uint_range("32", 1, 64);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 32);
}

#[test]
fn test_validate_uint_range_out_of_bounds() {
    let result = validate_uint_range("128", 1, 64);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of range"));
}

#[test]
fn test_validate_api_key_success() {
    let result = validate_api_key("sk-1234567890abcdef");
    assert!(result.is_ok());
}

#[test]
fn test_validate_api_key_too_short() {
    let result = validate_api_key("short");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("too short"));
}

#[test]
fn test_validate_api_key_empty() {
    let result = validate_api_key("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("cannot be empty"));
}

#[test]
fn test_validate_url_success() {
    let result = validate_url("https://api.openai.com/v1");
    assert!(result.is_ok());
    let result = validate_url("http://localhost:8080");
    assert!(result.is_ok());
}

#[test]
fn test_validate_url_invalid_format() {
    let result = validate_url("ftp://example.com");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid URL format")
    );
}

#[test]
fn test_parse_bool_true_values() {
    let true_values = ["true", "1", "yes", "on", "enabled", "TRUE", "Yes", "ON"];
    for value in &true_values {
        let result = parse_bool(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
    }
}

#[test]
fn test_parse_bool_false_values() {
    let false_values = ["false", "0", "no", "off", "disabled", "FALSE", "No", "OFF"];
    for value in &false_values {
        let result = parse_bool(value);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
}

#[test]
fn test_parse_bool_invalid_value() {
    let result = parse_bool("maybe");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid boolean value")
    );
}
