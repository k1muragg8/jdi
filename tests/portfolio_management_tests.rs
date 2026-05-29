use clap::Parser;
use pendulum_kelly_cli::cli::{Cli, Commands, PortfolioCommands};
use pendulum_kelly_cli::repository::RepositoryContext;

#[test]
fn test_cli_parsing_portfolio_list() {
    let args = vec!["pendulum-kelly-cli", "portfolio", "list"];
    let cli = Cli::parse_from(args);

    if let Commands::Portfolio { command } = cli.command {
        match command {
            PortfolioCommands::List => {}
            _ => panic!("Expected List command"),
        }
    } else {
        panic!("Expected Portfolio command");
    }
}

#[test]
fn test_cli_parsing_portfolio_create() {
    let args = vec!["pendulum-kelly-cli", "portfolio", "create", "test_p"];
    let cli = Cli::parse_from(args);

    if let Commands::Portfolio { command } = cli.command {
        match command {
            PortfolioCommands::Create { name } => {
                assert_eq!(name, "test_p");
            }
            _ => panic!("Expected Create command"),
        }
    } else {
        panic!("Expected Portfolio command");
    }
}

#[test]
fn test_cli_parsing_global_portfolio_flag() {
    let args = vec!["pendulum-kelly-cli", "--portfolio", "my_p", "tx", "list"];
    let cli = Cli::parse_from(args);

    assert_eq!(cli.portfolio, Some("my_p".to_string()));

    if let Commands::Tx { .. } = cli.command {
        // ok
    } else {
        panic!("Expected Tx command");
    }
}

#[test]
fn test_repository_context_default_portfolio() {
    let ctx = RepositoryContext::default();
    assert_eq!(ctx.portfolio_id, "default");
}
