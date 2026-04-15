mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init(args) => commands::init::run(args).await,
        Commands::Check(args) => commands::check::run(args).await,
        Commands::Wallet(args) => commands::wallet::run(args).await,
        Commands::Build(args) => commands::build::run(args).await,
        Commands::Dashboard(args) => commands::dashboard::run(args).await,
    }
}
