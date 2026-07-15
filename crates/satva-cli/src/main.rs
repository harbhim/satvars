mod config;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};

use config::PipelineConfig;
use satva_core::PipelineOptions;

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
        // Passed as a reference (`&config`) so the `PathBuf` correctly coerces to `&Path`
        Command::Run { config } => run(&config),
    }
}

fn run(config_path: &Path) -> Result<()> {
    // `config_path` is already a `&Path`, so taking another reference (`&config_path`)
    // creates a `&&Path`, which triggers a Clippy warning.
    let config = PipelineConfig::load(config_path)?;
    let (mut pipeline, schema) = config.build()?;

    if let Some(schema) = schema {
        println!("Inferred schema:");
        println!("{schema:#?}\n");
    }

    let result = pipeline.run(PipelineOptions::new())?;

    println!("Pipeline summary:");
    println!("{:#?}", result.summary);

    if !result.logs.is_empty() {
        println!("\nLogs:");
        for log in &result.logs {
            println!("{log:?}");
        }
    }

    Ok(())
}
