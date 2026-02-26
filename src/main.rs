mod cli;
mod commands;
mod config;
mod error;
mod frontmatter;
mod links;
mod note;
mod output;
mod srs;

use clap::Parser;

use cli::{Cli, Commands};
use config::{load_config, resolve_vault, resolve_vault_with_file};
use error::{format_error, SproutError};

fn main() {
    let cli = Cli::parse();
    let format = &cli.format;

    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            let err = SproutError::ParseError(format!("config: {e}"));
            format_error(&err, format);
            std::process::exit(1);
        }
    };

    let result = run_command(&cli, &config);

    if let Err(e) = result {
        format_error(&e, format);
        std::process::exit(1);
    }
}

fn run_command(cli: &Cli, config: &config::Config) -> Result<(), SproutError> {
    let format = &cli.format;

    match &cli.command {
        Commands::Init { file } => {
            let vault = resolve_vault_for_file(file, cli, config)?;
            commands::init::run(file, &vault, config, format)
        }
        Commands::Show { file } => {
            let vault = resolve_vault_for_file(file, cli, config)?;
            commands::show::run(file, &vault, format)
        }
        Commands::Done { file, rating } => {
            let vault = resolve_vault_for_file(file, cli, config)?;
            commands::done::run(file, rating, &vault, config, format)
        }
        Commands::Promote { file, maturity } => {
            let vault = resolve_vault_for_file(file, cli, config)?;
            commands::promote::run(file, maturity, &vault, format)
        }
        Commands::Review => {
            let vault = resolve_vault_safe(cli, config)?;
            commands::review::run(&vault, &config.exclude_dirs(), format)
        }
        Commands::List { maturity } => {
            let vault = resolve_vault_safe(cli, config)?;
            commands::list::run(&vault, maturity.as_ref(), &config.exclude_dirs(), format)
        }
        Commands::Stats => {
            let vault = resolve_vault_safe(cli, config)?;
            commands::stats::run(&vault, &config.exclude_dirs(), format)
        }
    }
}

/// Resolve vault for file-based commands.
/// For commands that take a file argument, derive the vault from the file's parent
/// if no explicit vault is given.
fn resolve_vault_for_file(
    file: &std::path::Path,
    cli: &Cli,
    config: &config::Config,
) -> Result<std::path::PathBuf, SproutError> {
    resolve_vault_with_file(cli.vault.as_ref(), config, Some(file))
        .map_err(|e: anyhow::Error| SproutError::VaultNotFound(e.to_string()))
}

/// Resolve vault for vault-wide commands (review, list, stats).
fn resolve_vault_safe(
    cli: &Cli,
    config: &config::Config,
) -> Result<std::path::PathBuf, SproutError> {
    resolve_vault(cli.vault.as_ref(), config)
        .map_err(|e: anyhow::Error| SproutError::VaultNotFound(e.to_string()))
}
