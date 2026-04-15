use clap::{Parser, Subcommand};
use crate::commands::{build::BuildArgs, check::CheckArgs, dashboard::DashboardArgs, init::InitArgs, wallet::WalletArgs};

#[derive(Parser)]
#[command(
    name = "sui-runner",
    about = "A setup and utility CLI for the Sui ecosystem",
    long_about = "sui-runner helps you set up, build, and manage projects on the Sui blockchain.\nRun any subcommand with --help for detailed usage.",
    version
)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

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
