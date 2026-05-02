use anyhow::{Context, Result};
use clap::Args;
use duct::cmd;
use notify::{Config, PollWatcher, Watcher};
use std::{path::PathBuf, sync::mpsc, time::Duration};

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

    /// Optional watch mode
    #[arg(short, long)]
    pub watch: bool,
}

pub async fn run(args: BuildArgs, verbose: bool) -> Result<()> {
    let path = args
        .path
        .canonicalize()
        .context("Package path does not exist")?;

    if verbose {
        println!(
            "sui CLI args: move {} --path {}",
            if args.test { "test" } else { "build" },
            path.display()
        );
    }

    if args.watch {
        run_watch(&path, &args)
    } else {
        if args.test {
            run_tests(&path, args.skip_fetch, args.filter.as_deref())
        } else {
            run_build(&path, args.skip_fetch, args.doc)
        }
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

fn run_watch(path: &PathBuf, args: &BuildArgs) -> Result<()> {
    println!("Watching for file changes in {}......", path.display());
    trigger_action(path, args)?;
    let (tx, rx) = mpsc::channel();
    let config = Config::default().with_poll_interval(Duration::from_millis(500));
    let mut watcher = PollWatcher::new(tx, config)?;
    let n_path = path.join(PathBuf::from("sources"));
    watcher.watch(&n_path, notify::RecursiveMode::Recursive)?;
    let toml_path = path.join("Move.toml");
    if toml_path.exists() {
        watcher.watch(&toml_path, notify::RecursiveMode::NonRecursive)?;
    }
    for res in rx {
        match res {
            Ok(_) => {
                println!("\nChange detected! Re-running...");
                let _ = trigger_action(path, args);
            }
            Err(e) => return Err(anyhow::anyhow!("Watch error: {:?}", e)),
        }
    }
    Ok(())
}
// Helper to decide whether to run build or test inside the loop
fn trigger_action(path: &PathBuf, args: &BuildArgs) -> Result<()> {
    if args.test {
        run_tests(path, args.skip_fetch, args.filter.as_deref())
    } else {
        run_build(path, args.skip_fetch, args.doc)
    }
}
