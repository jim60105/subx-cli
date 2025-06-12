use clap::Parser;
use subx_cli::cli::{Cli, Commands, ConfigAction};

#[test]
fn test_config_list_parse() {
    let cli = Cli::try_parse_from(&["subx", "config", "list"]).expect("parse 'config list'");
    match cli.command {
        Commands::Config(args) => {
            assert!(matches!(args.action, ConfigAction::List));
        }
        _ => panic!("Expected Config subcommand"),
    }
}

#[test]
fn test_config_set_parse() {
    let cli = Cli::try_parse_from(&["subx", "config", "set", "ai.provider", "gpt-4.1"]).
        expect("parse 'config set'");
    match cli.command {
        Commands::Config(args) => {
            if let ConfigAction::Set { key, value } = args.action {
                assert_eq!(key, "ai.provider");
                assert_eq!(value, "gpt-4.1");
            } else {
                panic!("Expected Set action");
            }
        }
        _ => panic!("Expected Config subcommand"),
    }
}

#[test]
fn test_config_get_parse() {
    let cli = Cli::try_parse_from(&["subx", "config", "get", "general.timeout"]).
        expect("parse 'config get'");
    match cli.command {
        Commands::Config(args) => {
            if let ConfigAction::Get { key } = args.action {
                assert_eq!(key, "general.timeout");
            } else {
                panic!("Expected Get action");
            }
        }
        _ => panic!("Expected Config subcommand"),
    }
}

#[test]
fn test_config_reset_parse() {
    let cli = Cli::try_parse_from(&["subx", "config", "reset"]).
        expect("parse 'config reset'");
    match cli.command {
        Commands::Config(args) => {
            assert!(matches!(args.action, ConfigAction::Reset));
        }
        _ => panic!("Expected Config subcommand"),
    }
}

#[test]
fn test_unknown_config_subcommand_fails() {
    let err = Cli::try_parse_from(&["subx", "config", "foo"]).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("error"), "unexpected error output: {}", msg);
}
