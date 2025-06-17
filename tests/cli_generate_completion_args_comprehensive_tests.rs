//! Comprehensive tests for shell completion generation command-line arguments.
//!
//! This module provides complete test coverage for the GenerateCompletionArgs
//! type, testing all supported shells and functionality according to the testing
//! guidelines in `docs/testing-guidelines.md`.

use clap_complete::Shell;
use subx_cli::cli::GenerateCompletionArgs;

mod generate_completion_args_tests {
    use super::*;

    #[test]
    fn test_generate_completion_args_creation() {
        // Test Bash completion args
        let bash_args = GenerateCompletionArgs { shell: Shell::Bash };
        assert_eq!(bash_args.shell, Shell::Bash);

        // Test Zsh completion args
        let zsh_args = GenerateCompletionArgs { shell: Shell::Zsh };
        assert_eq!(zsh_args.shell, Shell::Zsh);

        // Test Fish completion args
        let fish_args = GenerateCompletionArgs { shell: Shell::Fish };
        assert_eq!(fish_args.shell, Shell::Fish);

        // Test PowerShell completion args
        let powershell_args = GenerateCompletionArgs {
            shell: Shell::PowerShell,
        };
        assert_eq!(powershell_args.shell, Shell::PowerShell);

        // Test Elvish completion args
        let elvish_args = GenerateCompletionArgs {
            shell: Shell::Elvish,
        };
        assert_eq!(elvish_args.shell, Shell::Elvish);
    }

    #[test]
    fn test_generate_completion_args_debug_implementation() {
        let args = GenerateCompletionArgs { shell: Shell::Bash };

        let debug_output = format!("{:?}", args);
        assert!(debug_output.contains("GenerateCompletionArgs"));
        assert!(debug_output.contains("Bash"));
    }

    #[test]
    fn test_all_supported_shells() {
        let shells = vec![
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ];

        for shell in shells {
            let args = GenerateCompletionArgs { shell };

            // Verify each shell can be created and accessed
            match args.shell {
                Shell::Bash => assert_eq!(args.shell, Shell::Bash),
                Shell::Zsh => assert_eq!(args.shell, Shell::Zsh),
                Shell::Fish => assert_eq!(args.shell, Shell::Fish),
                Shell::PowerShell => assert_eq!(args.shell, Shell::PowerShell),
                Shell::Elvish => assert_eq!(args.shell, Shell::Elvish),
                _ => panic!("Unexpected shell variant"),
            }
        }
    }

    #[test]
    fn test_shell_equality() {
        let bash_args1 = GenerateCompletionArgs { shell: Shell::Bash };
        let bash_args2 = GenerateCompletionArgs { shell: Shell::Bash };
        let zsh_args = GenerateCompletionArgs { shell: Shell::Zsh };

        // Test equality
        assert_eq!(bash_args1.shell, bash_args2.shell);
        assert_ne!(bash_args1.shell, zsh_args.shell);
    }

    #[test]
    fn test_shell_debug_output() {
        let test_cases = vec![
            (Shell::Bash, "Bash"),
            (Shell::Zsh, "Zsh"),
            (Shell::Fish, "Fish"),
            (Shell::PowerShell, "PowerShell"),
            (Shell::Elvish, "Elvish"),
        ];

        for (shell, expected_string) in test_cases {
            let args = GenerateCompletionArgs { shell };
            let debug_output = format!("{:?}", args);
            assert!(debug_output.contains(expected_string));
        }
    }

    #[test]
    fn test_shell_copy_and_clone() {
        let original = GenerateCompletionArgs { shell: Shell::Bash };

        // Test that Shell implements Copy
        let copied_shell = original.shell;
        assert_eq!(copied_shell, Shell::Bash);

        // Test that the original is still usable after copy
        assert_eq!(original.shell, Shell::Bash);
    }

    #[test]
    fn test_generate_completion_args_with_different_shells() {
        let test_shells = vec![
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ];

        let mut args_vec = Vec::new();

        // Create args for each shell
        for shell in test_shells {
            args_vec.push(GenerateCompletionArgs { shell });
        }

        // Verify each args was created correctly
        assert_eq!(args_vec.len(), 5);
        assert_eq!(args_vec[0].shell, Shell::Bash);
        assert_eq!(args_vec[1].shell, Shell::Zsh);
        assert_eq!(args_vec[2].shell, Shell::Fish);
        assert_eq!(args_vec[3].shell, Shell::PowerShell);
        assert_eq!(args_vec[4].shell, Shell::Elvish);
    }

    #[test]
    fn test_shell_match_patterns() {
        let shells_and_expected = vec![
            (Shell::Bash, "unix-like shell"),
            (Shell::Zsh, "unix-like shell"),
            (Shell::Fish, "modern shell"),
            (Shell::PowerShell, "microsoft shell"),
            (Shell::Elvish, "modern shell"),
        ];

        for (shell, shell_type) in shells_and_expected {
            let args = GenerateCompletionArgs { shell };

            let shell_category = match args.shell {
                Shell::Bash | Shell::Zsh => "unix-like shell",
                Shell::Fish | Shell::Elvish => "modern shell",
                Shell::PowerShell => "microsoft shell",
                _ => "unknown shell",
            };

            assert_eq!(shell_category, shell_type);
        }
    }

    #[test]
    fn test_multiple_args_instances() {
        // Test creating multiple instances with different shells
        let bash = GenerateCompletionArgs { shell: Shell::Bash };
        let zsh = GenerateCompletionArgs { shell: Shell::Zsh };
        let fish = GenerateCompletionArgs { shell: Shell::Fish };

        // Verify they are independent and correctly set
        assert_eq!(bash.shell, Shell::Bash);
        assert_eq!(zsh.shell, Shell::Zsh);
        assert_eq!(fish.shell, Shell::Fish);

        // Verify they are different from each other
        assert_ne!(bash.shell, zsh.shell);
        assert_ne!(bash.shell, fish.shell);
        assert_ne!(zsh.shell, fish.shell);
    }
}
