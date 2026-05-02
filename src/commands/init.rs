use anyhow::{Context, Result, anyhow};
use clap::Args;
use duct::cmd;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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

pub async fn run(args: InitArgs, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project_name = args.name.unwrap_or_else(|| {
        cwd.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });

    if !is_safe_name(&*project_name) {
        return Err(anyhow!(
            "project name '{}' contains invalid characters for a folder",
            project_name
        ));
    }
    println!(
        "Initialising project '{}' on {}",
        project_name, args.network
    );

    let config = ProjectConfig {
        name: project_name.clone(),
        network: args.network.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let cmd_args = vec!["move", "new", project_name.as_str()];
    cmd("sui", cmd_args).run().expect("Command Failed");
    // out.stdout
    let mut pth_file = PathBuf::from(&*project_name);
    pth_file.push("sources");
    let file_name = format!("{project_name}.move");
    pth_file.push(file_name);
    write_hello_sui(pth_file, &*project_name)?;
    let mut config_path = PathBuf::from(project_name.clone());
    config_path.push("sui-runner.json");
    let json = serde_json::to_string_pretty(&config)?;
    if verbose {
        println!("Writing config:\n{}", json);
    }
    fs::write(&config_path, json).context("Failed to write sui-runner.json")?;

    println!("Created {}", config_path.display());
    println!(
        "Done. Run cd {} \nRun `sui-runner check` to verify your toolchain.",
        &*project_name
    );

    Ok(())
}

fn write_hello_sui(path: PathBuf, name: &str) -> Result<()> {
    let mut file_op = OpenOptions::new().append(true).open(path)?;
    let hello_sui = format!(
        r#"
module {name}::hello_sui;

use std::string::String;

public fun hello_world(): String {{
    b"Hello, Sui Runner!".to_string()
}}
"#
    );
    writeln!(file_op, "{}", hello_sui)?;
    // file_op.write_all(hello_sui.as_bytes()).await?;
    Ok(())
}

fn is_safe_name(name: &str) -> bool {
    let path = Path::new(name);
    // Ensure it's just a single component and not a full path or ".."
    path.components().count() == 1 && !name.contains('/') && !name.contains('\\')
}
