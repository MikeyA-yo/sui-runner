use anyhow::Context;
use clap::Args;
use duct::cmd;
use std::path::PathBuf;

#[derive(Args)]
pub struct DeployArgs {
    /// Path to the Move package (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,

    /// Maximum gas budget in MIST (defaults to 150_000_000)
    #[arg(short, long, default_value = "150000000")]
    pub gas: u64,
}

pub async fn run(args: DeployArgs) -> anyhow::Result<()> {
    let path = &args.path;
    deploy(path, args.gas)?;
    Ok(())
}

pub fn deploy(path: &PathBuf, gas: u64) -> anyhow::Result<()> {
    let gas_str = gas.to_string();
    let path_str = path.to_str().unwrap_or(".");

    cmd("sui", ["client", "publish", "--gas-budget", &gas_str, path_str])
        .run()
        .context("Failed to deploy sui package")?;
    Ok(())
}
