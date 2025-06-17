//! Tests for shell completion generation CLI arguments.
//!
//! This module provides comprehensive tests for the shell completion generation
//! command-line interface, focusing on argument parsing, shell type validation,
//! and debug formatting. All tests follow the testing guidelines.

use clap::Parser;
use clap_complete::Shell;
use subx_cli::cli::{Cli, Commands, GenerateCompletionArgs};

/// Test generate completion args parsing for bash
#[test]
fn test_generate_completion_bash_parsing() {
    let cli = Cli::try_parse_from(["subx", "generate-completion", "bash"]).unwrap();

    if let Commands::GenerateCompletion(args) = cli.command {
        assert_eq!(args.shell, Shell::Bash);
    } else {
        panic!("Expected Commands::GenerateCompletion");
    }
}

/// Test generate completion args parsing for zsh
#[test]
fn test_generate_completion_zsh_parsing() {
    let cli = Cli::try_parse_from(["subx", "generate-completion", "zsh"]).unwrap();

    if let Commands::GenerateCompletion(args) = cli.command {
        assert_eq!(args.shell, Shell::Zsh);
    } else {
        panic!("Expected Commands::GenerateCompletion");
    }
}

/// Test generate completion args parsing for fish
#[test]
fn test_generate_completion_fish_parsing() {
    let cli = Cli::try_parse_from(["subx", "generate-completion", "fish"]).unwrap();

    if let Commands::GenerateCompletion(args) = cli.command {
        assert_eq!(args.shell, Shell::Fish);
    } else {
        panic!("Expected Commands::GenerateCompletion");
    }
}

/// Test generate completion args parsing for powershell
#[test]
fn test_generate_completion_powershell_parsing() {
    let cli = Cli::try_parse_from(["subx", "generate-completion", "powershell"]).unwrap();

    if let Commands::GenerateCompletion(args) = cli.command {
        assert_eq!(args.shell, Shell::PowerShell);
    } else {
        panic!("Expected Commands::GenerateCompletion");
    }
}

/// Test generate completion args parsing for elvish
#[test]
fn test_generate_completion_elvish_parsing() {
    let cli = Cli::try_parse_from(["subx", "generate-completion", "elvish"]).unwrap();

    if let Commands::GenerateCompletion(args) = cli.command {
        assert_eq!(args.shell, Shell::Elvish);
    } else {
        panic!("Expected Commands::GenerateCompletion");
    }
}

/// Test all supported shell types
#[test]
fn test_all_supported_shells() {
    let shells = vec![
        ("bash", Shell::Bash),
        ("zsh", Shell::Zsh),
        ("fish", Shell::Fish),
        ("powershell", Shell::PowerShell),
        ("elvish", Shell::Elvish),
    ];

    for (shell_str, expected_shell) in shells {
        let cli = Cli::try_parse_from(["subx", "generate-completion", shell_str]).unwrap();

        if let Commands::GenerateCompletion(args) = cli.command {
            assert_eq!(args.shell, expected_shell);
        } else {
            panic!(
                "Expected Commands::GenerateCompletion for shell: {}",
                shell_str
            );
        }
    }
}

/// Test generate completion args debug formatting
#[test]
fn test_generate_completion_args_debug_formatting() {
    let args = GenerateCompletionArgs { shell: Shell::Bash };

    let debug_str = format!("{:?}", args);
    assert!(debug_str.contains("GenerateCompletionArgs"));
    assert!(debug_str.contains("shell"));
}

/// Test generate completion args with invalid shell
#[test]
fn test_generate_completion_invalid_shell() {
    let result = Cli::try_parse_from(["subx", "generate-completion", "invalid-shell"]);

    assert!(result.is_err());
}

/// Test generate completion args missing shell argument
#[test]
fn test_generate_completion_missing_shell() {
    let result = Cli::try_parse_from(["subx", "generate-completion"]);

    assert!(result.is_err());
}

/// Test shell enum equality
#[test]
fn test_shell_enum_equality() {
    assert_eq!(Shell::Bash, Shell::Bash);
    assert_eq!(Shell::Zsh, Shell::Zsh);
    assert_eq!(Shell::Fish, Shell::Fish);
    assert_eq!(Shell::PowerShell, Shell::PowerShell);
    assert_eq!(Shell::Elvish, Shell::Elvish);

    assert_ne!(Shell::Bash, Shell::Zsh);
    assert_ne!(Shell::Fish, Shell::PowerShell);
}

/// Test struct construction and field access
#[test]
fn test_generate_completion_args_construction() {
    let args = GenerateCompletionArgs { shell: Shell::Zsh };

    assert_eq!(args.shell, Shell::Zsh);
}

/// Test all shell variants
#[test]
fn test_all_shell_variants() {
    let shells = vec![
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ];

    for shell in shells {
        let args = GenerateCompletionArgs { shell };
        let debug_output = format!("{:?}", args);
        assert!(!debug_output.is_empty());
    }
}

/// Test shell enum debug formatting
#[test]
fn test_shell_enum_debug_formatting() {
    let shells = vec![
        Shell::Bash,
        Shell::Zsh,
        Shell::Fish,
        Shell::PowerShell,
        Shell::Elvish,
    ];

    for shell in shells {
        let debug_str = format!("{:?}", shell);
        assert!(!debug_str.is_empty());
    }
}

/// Test case sensitivity for shell names
#[test]
fn test_shell_name_case_sensitivity() {
    // Test that shell names are case sensitive
    let result = Cli::try_parse_from([
        "subx",
        "generate-completion",
        "BASH", // uppercase should fail
    ]);

    assert!(result.is_err());

    let result = Cli::try_parse_from([
        "subx",
        "generate-completion",
        "Zsh", // mixed case should fail
    ]);

    assert!(result.is_err());
}

/// Test that shell argument is required
#[test]
fn test_shell_argument_required() {
    let result = Cli::try_parse_from(["subx", "generate-completion"]);

    assert!(result.is_err());
}

/// Test args pattern matching
#[test]
fn test_generate_completion_pattern_matching() {
    let args = GenerateCompletionArgs { shell: Shell::Fish };

    match args.shell {
        Shell::Bash => panic!("Expected Fish, got Bash"),
        Shell::Zsh => panic!("Expected Fish, got Zsh"),
        Shell::Fish => {
            // Expected
        }
        Shell::PowerShell => panic!("Expected Fish, got PowerShell"),
        Shell::Elvish => panic!("Expected Fish, got Elvish"),
        _ => panic!("Unexpected shell variant"),
    }
}
