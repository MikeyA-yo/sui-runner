use clap::{Parser, Subcommand};
use crate::commands::{build::BuildArgs, check::CheckArgs, dashboard::DashboardArgs, init::InitArgs, wallet::WalletArgs};

#[derive(Parser)]
#[command(
    name = "sui-runner",
    about = "A setup and utility CLI for the Sui ecosystem",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialise a new Sui project in the current directory
    Init(InitArgs),
    /// Check that required tools (sui, git, etc.) are installed
    Check(CheckArgs),
    /// Manage Sui wallets
    Wallet(WalletArgs),
    /// Compile (or test) a Move package
    Build(BuildArgs),
    /// Live TUI dashboard — active address, network, project info
    Dashboard(DashboardArgs),
}
