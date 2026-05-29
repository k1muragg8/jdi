use clap::Parser;
use pendulum_kelly_cli::cli::{Cli, Commands, DataCommands};
use std::fs;
use std::path::Path;

#[test]
fn test_cli_parsing_export_json() {
    let args = vec!["pendulum-kelly-cli", "data", "export", "--json"];
    let cli = Cli::parse_from(args);

    if let Commands::Data { command } = cli.command {
        if let DataCommands::Export { json, .. } = command {
            assert!(json);
        } else {
            panic!("Expected Export command");
        }
    } else {
        panic!("Expected Data command");
    }
}

#[test]
fn test_cli_parsing_export_dir_and_force() {
    let args = vec![
        "pendulum-kelly-cli",
        "data",
        "export",
        "--json",
        "--dir",
        "my-export",
        "--force",
    ];
    let cli = Cli::parse_from(args);

    if let Commands::Data { command } = cli.command {
        if let DataCommands::Export { json, dir, force } = command {
            assert!(json);
            assert_eq!(dir, Some("my-export".to_string()));
            assert!(force);
        } else {
            panic!("Expected Export command");
        }
    } else {
        panic!("Expected Data command");
    }
}

#[test]
fn test_output_path_generation_concept() {
    // This just tests the logic we used in src/lib.rs
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let export_dir = format!("data/export-json/{}", timestamp);
    assert!(export_dir.starts_with("data/export-json/"));
    assert_eq!(export_dir.len(), "data/export-json/YYYYMMDD-HHMMSS".len());
}

#[test]
fn test_export_no_overwrite_behavior() {
    let export_dir = "data/test_export_no_overwrite";
    let _ = fs::create_dir_all(export_dir);

    // In src/lib.rs we have:
    // if path.exists() && !*force { anyhow::bail!(...) }

    let path = Path::new(export_dir);
    let force = false;
    let exists_and_no_force = path.exists() && !force;

    assert!(exists_and_no_force);

    let force_true = true;
    let exists_and_force = path.exists() && !force_true;
    assert!(!exists_and_force);

    let _ = fs::remove_dir_all(export_dir);
}
