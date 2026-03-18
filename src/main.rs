mod api;
mod cmd;
mod config;
mod display;

use clap::Parser;
use cmd::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = cmd::run(cli).await {
        eprintln!("{}: {e:#}", colored::Colorize::red("error"));
        std::process::exit(1);
    }
}
