//! Shell completion script generation command-line arguments.
//!
//! This module defines the command-line interface for generating shell completion
//! scripts that enable tab completion for SubX commands in various shell environments.
//! This significantly improves the user experience by providing intelligent command
//! and argument completion.
//!
//! # Supported Shells
//!
//! SubX can generate completion scripts for all major shells:
//! - **Bash**: Most common on Linux systems
//! - **Zsh**: Default on macOS and advanced Linux setups
//! - **Fish**: Modern shell with powerful completion features
//! - **PowerShell**: Windows and cross-platform PowerShell environments
//! - **Elvish**: Modern shell with structured data support
//!
//! # Completion Features
//!
//! Generated completion scripts provide:
//! - **Command Completion**: Tab completion for all subcommands
//! - **Argument Completion**: Intelligent completion for command arguments
//! - **Path Completion**: File and directory path suggestions
//! - **Enum Completion**: Valid values for enum-based arguments
//! - **Help Integration**: Context-aware help text display
//!
//! # Installation Examples
//!
//! ```bash
//! # Bash (add to ~/.bashrc)
//! eval "$(subx generate-completion bash)"
//!
//! # Zsh (add to ~/.zshrc)
//! eval "$(subx generate-completion zsh)"
//!
//! # Fish (save to completions directory)
//! subx generate-completion fish > ~/.config/fish/completions/subx.fish
//!
//! # PowerShell (add to profile)
//! subx generate-completion powershell | Out-String | Invoke-Expression
//! ```

// src/cli/generate_completion_args.rs
use clap::Args;
use clap_complete::Shell;

/// Command-line arguments for generating shell completion scripts.
///
/// The generate-completion command creates shell-specific completion scripts
/// that enable intelligent tab completion for SubX commands and arguments.
/// This greatly enhances the command-line user experience by providing
/// context-aware suggestions and reducing typing requirements.
///
/// # Completion Capabilities
///
/// Generated scripts provide completion for:
/// - **Subcommands**: `match`, `convert`, `sync`, `detect-encoding`, etc.
/// - **Flags and Options**: `--format`, `--output`, `--confidence`, etc.
/// - **Enum Values**: Available formats, AI providers, sync methods
/// - **File Paths**: Intelligent file and directory completion
/// - **Configuration Keys**: Valid configuration setting names
///
/// # Shell Integration
///
/// Each shell has different integration methods:
/// - **Immediate**: Load completion in current session
/// - **Persistent**: Add to shell configuration for permanent availability
/// - **System-wide**: Install for all users on the system
/// - **Per-project**: Enable completion in specific project directories
///
/// # Performance Considerations
///
/// Completion scripts are optimized for performance:
/// - **Lazy Loading**: Completions are generated on-demand
/// - **Caching**: Results are cached where appropriate
/// - **Minimal Overhead**: Scripts add minimal startup time to shell
/// - **Incremental Updates**: Only regenerate when command structure changes
///
/// # Examples
///
/// ```rust
/// use subx_cli::cli::GenerateCompletionArgs;
/// use clap_complete::Shell;
///
/// // Generate Bash completion
/// let bash_args = GenerateCompletionArgs {
///     shell: Shell::Bash,
/// };
///
/// // Generate Zsh completion
/// let zsh_args = GenerateCompletionArgs {
///     shell: Shell::Zsh,
/// };
///
/// // Generate Fish completion
/// let fish_args = GenerateCompletionArgs {
///     shell: Shell::Fish,
/// };
/// ```
#[derive(Args, Debug)]
pub struct GenerateCompletionArgs {
    /// Target shell for completion script generation
    #[arg(value_enum)]
    pub shell: Shell,
}
