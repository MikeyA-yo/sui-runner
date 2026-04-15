use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Args)]
pub struct InitArgs {
    /// Project name (defaults to current directory name)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Target network: mainnet | testnet | devnet | localnet
    #[arg(short = 'N', long, default_value = "testnet")]
    pub network: String,
}

#[derive(Serialize, Deserialize)]
struct ProjectConfig {
    name: String,
    network: String,
    version: String,
}

pub async fn run(args: InitArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project_name = args
        .name
        .unwrap_or_else(|| cwd.file_name().unwrap_or_default().to_string_lossy().into_owned());

    println!("Initialising project '{}' on {}", project_name, args.network);

    let config = ProjectConfig {
        name: project_name.clone(),
        network: args.network.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let config_path = PathBuf::from("sui-runner.json");
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, json).context("Failed to write sui-runner.json")?;

    println!("Created {}", config_path.display());
    println!("Done. Run `sui-runner check` to verify your toolchain.");

    Ok(())
}
