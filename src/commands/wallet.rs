use anyhow::Result;
use clap::{Args, Subcommand};
use duct::cmd;

#[derive(Args)]
pub struct WalletArgs {
    #[command(subcommand)]
    pub action: WalletAction,
}

#[derive(Subcommand)]
pub enum WalletAction {
    /// Show the active wallet address
    Address,
    /// List all addresses in the keystore
    List,
    /// Create a new address
    New,
}

pub async fn run(args: WalletArgs) -> Result<()> {
    match args.action {
        WalletAction::Address => {
            let out = cmd!("sui", "client", "active-address").read()?;
            println!("{}", out.trim());
        }
        WalletAction::List => {
            let out = cmd!("sui", "client", "addresses").read()?;
            println!("{}", out.trim());
        }
        WalletAction::New => {
            let out = cmd!("sui", "client", "new-address", "ed25519").read()?;
            println!("{}", out.trim());
        }
    }
    Ok(())
}
