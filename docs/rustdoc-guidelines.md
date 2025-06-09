# SubX Rustdoc Guidelines

This document establishes comprehensive standards for documenting Rust code in the SubX project, ensuring consistency, clarity, and maintainability.

## General Principles

- **Language**: All documentation must be written in English
- **Clarity**: Use clear, concise language that avoids unnecessary jargon
- **Practicality**: Provide practical examples that demonstrate real-world usage
- **Completeness**: Document error conditions, panics, and edge cases
- **Connectivity**: Include links to related functions, types, and modules using `[`backticks`]`

## Module Documentation

Module-level documentation should use `//!` syntax and include:

```rust
//! Brief one-line summary of the module's purpose.
//!
//! Detailed explanation of the module's functionality, architecture,
//! and key concepts. Include examples of typical usage patterns.
//!
//! # Key Components
//!
//! - [`MainStruct`] - Primary structure for handling operations
//! - [`helper_function`] - Utility function for common tasks
//!
//! # Examples
//!
//! ```rust
//! use subx_cli::module_name::MainStruct;
//!
//! let instance = MainStruct::new();
//! instance.do_something()?;
//! ```
//!
//! # Architecture
//!
//! Describe the module's design patterns, relationships with other modules,
//! and any important architectural decisions.
```

## Struct and Enum Documentation

### Structs

```rust
/// Brief description of the struct's purpose.
///
/// Detailed explanation of the struct's role, typical usage,
/// and relationship to other types in the system.
///
/// # Fields
///
/// - `field_name`: Description of the field's purpose and constraints
/// - `another_field`: Description with validation rules or format
///
/// # Examples
///
/// ```rust
/// use subx_cli::SomeStruct;
///
/// let config = SomeStruct::new();
/// config.validate()?;
/// ```
///
/// # Thread Safety
///
/// Document thread safety characteristics if relevant.
pub struct MyStruct {
    /// Brief description of the field.
    /// 
    /// More detailed explanation if the field has complex behavior,
    /// validation rules, or specific format requirements.
    pub field_name: String,
}
```

### Enums

```rust
/// Brief description of the enum's purpose.
///
/// Detailed explanation of what the enum represents and when
/// each variant should be used.
///
/// # Variants
///
/// Each variant should be documented with its specific use case
/// and any associated data meanings.
///
/// # Examples
///
/// ```rust
/// use subx_cli::MyEnum;
///
/// let variant = MyEnum::VariantA("value".to_string());
/// match variant {
///     MyEnum::VariantA(data) => println!("Found: {}", data),
///     MyEnum::VariantB => println!("No data"),
/// }
/// ```
#[derive(Debug, Clone)]
pub enum MyEnum {
    /// Description of when this variant is used.
    ///
    /// Additional details about the associated data if present.
    VariantA(String),
    
    /// Description of when this variant is used.
    VariantB,
}
```

## Function Documentation

### Public Functions

```rust
/// Brief description of what the function does.
///
/// Detailed explanation of the function's behavior, including
/// any important algorithm details or implementation notes.
///
/// # Arguments
///
/// - `param1`: Description of the parameter and its constraints
/// - `param2`: Description with expected range or format
///
/// # Returns
///
/// Description of the return value and its meaning.
/// For `Result` types, describe both success and error cases.
///
/// # Errors
///
/// This function returns an error if:
/// - Specific condition 1 occurs
/// - Specific condition 2 occurs
/// - Input validation fails
///
/// # Panics
///
/// This function panics if:
/// - Specific panic condition (avoid panics in library code)
///
/// # Examples
///
/// ```rust
/// use subx_cli::my_function;
///
/// let result = my_function("input", 42)?;
/// assert_eq!(result, "expected_output");
/// ```
///
/// # Performance
///
/// Document performance characteristics for computationally expensive functions.
///
/// # Safety
///
/// Document safety requirements for unsafe functions.
pub fn my_function(param1: &str, param2: i32) -> Result<String, Error> {
    // Implementation
}
```

### Associated Functions and Methods

```rust
impl MyStruct {
    /// Creates a new instance with default settings.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let instance = MyStruct::new();
    /// ```
    pub fn new() -> Self {
        // Implementation
    }

    /// Performs an operation on the instance.
    ///
    /// # Arguments
    ///
    /// - `input`: The data to process
    ///
    /// # Returns
    ///
    /// The processed result or an error if processing fails.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut instance = MyStruct::new();
    /// let result = instance.process("data")?;
    /// ```
    pub fn process(&mut self, input: &str) -> Result<String, Error> {
        // Implementation
    }
}
```

## Error Documentation

Error types require special attention:

```rust
/// Comprehensive error handling for SubX operations.
///
/// This enum covers all possible error conditions with specific
/// context to facilitate debugging and user-friendly reporting.
///
/// # Error Categories
///
/// - I/O errors: File system operations
/// - Configuration errors: Invalid settings or missing values
/// - Processing errors: Format or content-related failures
///
/// # Examples
///
/// ```rust
/// use subx_cli::error::{SubXError, SubXResult};
///
/// fn example() -> SubXResult<()> {
///     Err(SubXError::Config {
///         message: "Missing required field".to_string(),
///     })
/// }
/// ```
#[derive(Error, Debug)]
pub enum SubXError {
    /// I/O operation failed during file system access.
    ///
    /// # Common Causes
    /// - Permission issues
    /// - Disk space shortage
    /// - Network file system problems
    ///
    /// # Resolution
    /// Check file permissions and available disk space.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Example Code Standards

### Quality Requirements

- **Compilation**: All examples must compile successfully
- **Relevance**: Examples should demonstrate practical usage
- **Completeness**: Include necessary imports and setup
- **Testing**: Use `cargo test --doc` to validate examples

### Example Patterns

```rust
/// # Examples
///
/// Basic usage:
///
/// ```rust
/// use subx_cli::SomeType;
///
/// let instance = SomeType::new();
/// let result = instance.process()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Advanced usage with configuration:
///
/// ```rust
/// # use subx_cli::{SomeType, Config};
/// let config = Config::builder()
///     .option("value")
///     .build();
/// let instance = SomeType::with_config(config);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
```

## Documentation Validation and CI Integration

### Local Development

```bash
# Generate documentation and check for warnings
cargo doc --all-features --no-deps --document-private-items

# Test documentation examples
cargo test --doc --verbose

# Check for missing documentation
cargo clippy -- -W missing_docs -D warnings
```

### CI/CD Integration

The following checks should be integrated into the CI pipeline:

```yaml
# Documentation quality check
- name: Check documentation
  run: |
    # Check for missing documentation
    cargo clippy -- -W missing_docs -D warnings
    
    # Build docs and check for warnings
    cargo doc --all-features --no-deps 2>&1 | \
      tee doc_output.log && ! grep -i "warning\|error" doc_output.log

# Test documentation examples
- name: Test documentation examples
  run: cargo test --doc --verbose
```

## Special Considerations

### Async Functions

```rust
/// Asynchronously processes the input data.
///
/// # Arguments
///
/// - `data`: Input data to process
///
/// # Returns
///
/// A future that resolves to the processed result.
///
/// # Examples
///
/// ```rust
/// # use tokio_test;
/// # use subx_cli::async_function;
/// #[tokio::test]
/// async fn test_async() {
///     let result = async_function("input").await?;
///     assert_eq!(result, "expected");
/// #   Ok::<(), Box<dyn std::error::Error>>(())
/// }
/// ```
pub async fn async_function(data: &str) -> Result<String, Error> {
    // Implementation
}
```

### Generic Functions

```rust
/// Processes items of any type that implements `Display`.
///
/// # Type Parameters
///
/// - `T`: Must implement `Display` for string conversion
///
/// # Examples
///
/// ```rust
/// use subx_cli::process_displayable;
///
/// let result = process_displayable(&42);
/// let result2 = process_displayable(&"hello");
/// ```
pub fn process_displayable<T: std::fmt::Display>(item: &T) -> String {
    // Implementation
}
```

## Maintenance and Quality Assurance

### Code Review Checklist

- [ ] All public APIs have documentation
- [ ] Examples compile and are tested
- [ ] Error conditions are documented
- [ ] Links to related types are included
- [ ] Documentation follows the established style

### Documentation Debt Management

- Track undocumented APIs in issues
- Prioritize documentation for frequently used functions
- Update documentation when APIs change
- Regular documentation audits

This guide ensures that SubX maintains high-quality, consistent documentation that serves both contributors and users effectively.
