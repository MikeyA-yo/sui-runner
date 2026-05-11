mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let verbose = cli.verbose;

    match cli.command {
        Commands::Init(args) => commands::init::run(args, verbose).await,
        Commands::Check(args) => commands::check::run(args).await,
        Commands::Wallet(args) => commands::wallet::run(args).await,
        Commands::Build(args) => commands::build::run(args, verbose).await,
        Commands::Dashboard(args) => commands::dashboard::run(args).await,
        Commands::Deploy(args) => commands::deploy::run(args).await,
    }
}
