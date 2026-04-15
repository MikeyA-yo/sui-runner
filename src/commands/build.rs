use anyhow::{Context, Result};
use clap::Args;
use duct::cmd;
use std::path::PathBuf;

#[derive(Args)]
pub struct BuildArgs {
    /// Path to the Move package (defaults to current directory)
    #[arg(short, long, default_value = ".")]
    pub path: PathBuf,

    /// Skip fetching latest git dependencies
    #[arg(long)]
    pub skip_fetch: bool,

    /// Generate Move documentation
    #[arg(long)]
    pub doc: bool,

    /// Run tests instead of building
    #[arg(short, long)]
    pub test: bool,

    /// (test only) filter tests by name substring
    #[arg(short, long, requires = "test")]
    pub filter: Option<String>,
}

pub async fn run(args: BuildArgs, verbose: bool) -> Result<()> {
    let path = args
        .path
        .canonicalize()
        .context("Package path does not exist")?;

    if verbose {
        println!("sui CLI args: move {} --path {}", if args.test { "test" } else { "build" }, path.display());
    }

    if args.test {
        run_tests(&path, args.skip_fetch, args.filter.as_deref())
    } else {
        run_build(&path, args.skip_fetch, args.doc)
    }
}

fn run_build(path: &PathBuf, skip_fetch: bool, doc: bool) -> Result<()> {
    println!("Building Move package at {}\n", path.display());

    let mut sui_args = vec!["move", "build", "--path"];
    let path_str = path.to_string_lossy();
    sui_args.push(&path_str);

    if skip_fetch {
        sui_args.push("--skip-fetch-latest-git-deps");
    }
    if doc {
        sui_args.push("--generate-docs");
    }

    cmd("sui", &sui_args)
        .run()
        .context("Failed to run `sui move build`. Is the Sui CLI installed?")?;

    Ok(())
}

fn run_tests(path: &PathBuf, skip_fetch: bool, filter: Option<&str>) -> Result<()> {
    println!("Running Move tests at {}\n", path.display());

    let path_str = path.to_string_lossy();
    let mut sui_args = vec!["move", "test", "--path", &path_str];

    if skip_fetch {
        sui_args.push("--skip-fetch-latest-git-deps");
    }
    if let Some(f) = filter {
        sui_args.push("--filter");
        sui_args.push(f);
    }

    cmd("sui", &sui_args)
        .run()
        .context("Failed to run `sui move test`. Is the Sui CLI installed?")?;

    Ok(())
}
