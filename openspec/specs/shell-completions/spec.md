# Shell Completions

## Purpose

Generate shell completion scripts so users can get tab-completion for `subx` commands, flags, and enum values in their shell of choice. Implemented in `src/cli/generate_completion_args.rs` and dispatched via the `GenerateCompletion` subcommand using `clap_complete`.

## Requirements

### Requirement: Supported Shells

The system SHALL expose `subx generate-completion <shell>`, where `<shell>` is any variant of `clap_complete::Shell` (at minimum: `bash`, `zsh`, `fish`, `powershell`, `elvish`).

#### Scenario: Generate Bash completion
- **GIVEN** the user runs `subx generate-completion bash`
- **WHEN** the command completes
- **THEN** the command SHALL print a valid Bash completion script to standard output and exit successfully

#### Scenario: Invalid shell rejected
- **GIVEN** the user runs `subx generate-completion unknown`
- **WHEN** the CLI parses the arguments
- **THEN** argument parsing SHALL fail with an error listing the valid shell values

### Requirement: Completion Scope

Generated scripts SHALL provide completion for all defined subcommands and their flags as declared in the `clap` command tree, without requiring network access, configuration, or any AI provider.

#### Scenario: No runtime dependencies
- **GIVEN** an environment with no AI API key, no configuration file, and no network connectivity
- **WHEN** the user runs `subx generate-completion zsh`
- **THEN** the command SHALL still succeed and emit the Zsh completion script
