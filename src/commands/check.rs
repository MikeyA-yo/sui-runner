use anyhow::Result;
use clap::Args;
use duct::cmd;

#[derive(Args)]
pub struct CheckArgs {
    /// Show versions of found tools
    #[arg(short, long)]
    pub verbose: bool,
}

struct Tool {
    name: &'static str,
    version_flag: &'static str,
}

const REQUIRED_TOOLS: &[Tool] = &[
    Tool { name: "sui",  version_flag: "--version" },
    Tool { name: "git",  version_flag: "--version" },
    Tool { name: "cargo", version_flag: "--version" },
];

pub async fn run(args: CheckArgs) -> Result<()> {
    println!("Checking required tools...\n");

    let mut all_ok = true;

    for tool in REQUIRED_TOOLS {
        match cmd(tool.name, &[tool.version_flag]).read() {
            Ok(output) => {
                let version = output.lines().next().unwrap_or("").trim().to_string();
                if args.verbose {
                    println!("  [ok]  {} — {}", tool.name, version);
                } else {
                    println!("  [ok]  {}", tool.name);
                }
            }
            Err(_) => {
                eprintln!("  [missing]  {}", tool.name);
                all_ok = false;
            }
        }
    }

    println!();
    if all_ok {
        println!("All tools found.");
    } else {
        anyhow::bail!("One or more required tools are missing. Install them and re-run.");
    }

    Ok(())
}
