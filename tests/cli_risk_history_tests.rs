use clap::Parser;
use pendulum_kelly_cli::cli::{Cli, Commands, RiskCommands};

#[test]
fn test_risk_history_cli_parsing() {
    // 1. Positional symbol
    let cli = Cli::try_parse_from(&[
        "pendulum-kelly-cli",
        "risk",
        "history",
        "^VIX",
        "--days",
        "250",
    ])
    .unwrap();
    if let Commands::Risk {
        command:
            RiskCommands::History {
                symbol,
                symbol_opt,
                days,
                ..
            },
    } = cli.command
    {
        assert_eq!(symbol, Some("^VIX".to_string()));
        assert_eq!(symbol_opt, None);
        assert_eq!(days, 250);
    } else {
        panic!("Incorrect command parsed");
    }

    // 2. Named --symbol
    let cli = Cli::try_parse_from(&[
        "pendulum-kelly-cli",
        "risk",
        "history",
        "--symbol",
        "^VIX",
        "--days",
        "250",
    ])
    .unwrap();
    if let Commands::Risk {
        command:
            RiskCommands::History {
                symbol,
                symbol_opt,
                days,
                ..
            },
    } = cli.command
    {
        assert_eq!(symbol, None);
        assert_eq!(symbol_opt, Some("^VIX".to_string()));
        assert_eq!(days, 250);
    } else {
        panic!("Incorrect command parsed");
    }

    // 3. Both (same)
    let cli = Cli::try_parse_from(&[
        "pendulum-kelly-cli",
        "risk",
        "history",
        "^VIX",
        "--symbol",
        "^VIX",
    ])
    .unwrap();
    if let Commands::Risk {
        command: RiskCommands::History {
            symbol, symbol_opt, ..
        },
    } = cli.command
    {
        assert_eq!(symbol, Some("^VIX".to_string()));
        assert_eq!(symbol_opt, Some("^VIX".to_string()));
    } else {
        panic!("Incorrect command parsed");
    }

    // 4. Both (different)
    let cli = Cli::try_parse_from(&[
        "pendulum-kelly-cli",
        "risk",
        "history",
        "^VIX",
        "--symbol",
        "^TYX",
    ])
    .unwrap();
    if let Commands::Risk {
        command: RiskCommands::History {
            symbol, symbol_opt, ..
        },
    } = cli.command
    {
        assert_eq!(symbol, Some("^VIX".to_string()));
        assert_eq!(symbol_opt, Some("^TYX".to_string()));
    } else {
        panic!("Incorrect command parsed");
    }
}
