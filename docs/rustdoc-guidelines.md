 # SubX Rustdoc Guidelines

 ## General Principles
 - All documentation must be written in English
 - Use clear, concise language
 - Provide practical examples where applicable
 - Document error conditions and panics
 - Include links to related functions/types

 ## Module Documentation
 - Start with a brief one-line summary
 - Explain the module's purpose and scope
 - List key types and functions
 - Provide usage examples for complex modules

 ## Function Documentation
 - Brief description of what the function does
 - Document all parameters with their constraints
 - Document return values and error conditions
 - Include examples for non-trivial functions
 - Use `# Errors`, `# Panics`, `# Examples` sections

 ## Struct Documentation
 - Brief description of the struct's purpose
 - Detailed explanation of the struct's role and relationships
 - Use `# Fields` section to document fields

 ## Example Code
 - Ensure all examples compile successfully
 - Use `cargo test --doc` to validate examples

 ## Linting and Validation
 - Run `cargo doc --all-features --no-deps` and fix warnings
 - Run `cargo test --doc` to ensure examples pass
