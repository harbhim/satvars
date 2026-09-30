use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use satva_core::PipelineOptions;
use satva_runner::run_yaml;

#[derive(Parser)]
#[command(
    name = "satva",
    version,
    about = "Run Satva data pipelines from a config file"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a pipeline defined in a YAML config file.
    Run {
        /// Path to the pipeline config (YAML).
        #[arg(short, long)]
        config: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { config } => run(&config),
    }
}

fn run(config_path: &Path) -> Result<()> {
    let report = run_yaml(config_path, PipelineOptions::new())?;

    if let Some(schema) = report.schema {
        println!("Inferred schema:");
        println!("{schema:#?}\n");
    }

    println!("Pipeline summary:");
    println!("{:#?}", report.summary);

    if !report.logs.is_empty() {
        println!("\nLogs:");
        for log in &report.logs {
            println!("{log:?}");
        }
    }

    Ok(())
}
