mod auth;
mod commands;
mod gravity;
mod search;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dv",
    version,
    about = "Dataverse CLI - Query social data from X/Twitter and Reddit via Macrocosmos SN13 (Bittensor Data Universe)",
    long_about = None,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output format: table, json, csv
    #[arg(short, long, global = true, default_value = "table")]
    pub output: String,

    /// API key (overrides MC_API env var and config file)
    #[arg(long, global = true)]
    pub api_key: Option<String>,

    /// Preview the API request without executing it
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Request timeout in seconds
    #[arg(long, global = true, default_value = "120")]
    pub timeout: u64,

    /// Base URL override
    #[arg(long, global = true, hide = true)]
    pub base_url: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Search social media posts from X or Reddit
    Search(search::SearchArgs),

    /// Manage Gravity data collection tasks
    #[command(subcommand)]
    Gravity(gravity::GravityCommands),

    /// Configure API key
    Auth,

    /// Check API key and connection status
    Status,

    /// Emit machine-readable JSON catalog of all commands, flags, and API mappings (for LLM/agent consumption)
    #[command(hide = true)]
    Commands,
}

/// Shared options extracted from Cli so the command enum can be moved independently.
pub struct GlobalOpts {
    pub output: String,
    pub api_key: Option<String>,
    pub dry_run: bool,
    pub timeout: u64,
    pub base_url: Option<String>,
}

pub async fn run(cli: Cli) -> Result<()> {
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            crate::display::banner::print_banner();
            return Ok(());
        }
    };

    let opts = GlobalOpts {
        output: cli.output,
        api_key: cli.api_key,
        dry_run: cli.dry_run,
        timeout: cli.timeout,
        base_url: cli.base_url,
    };

    match command {
        Commands::Search(args) => search::run(&opts, args).await,
        Commands::Gravity(cmd) => gravity::run(&opts, cmd).await,
        Commands::Auth => auth::run_auth().await,
        Commands::Status => auth::run_status(&opts).await,
        Commands::Commands => {
            commands::run_commands();
            Ok(())
        }
    }
}
