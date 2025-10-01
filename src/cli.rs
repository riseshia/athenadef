use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{apply, export, plan};

#[derive(Parser, Debug)]
#[command(name = "athenadef")]
#[command(version, about = "AWS Athena schema management tool", long_about = None)]
pub struct Cli {
    /// Config file path
    #[arg(short, long, default_value = "athenadef.yaml")]
    pub config: String,

    /// Enable debug logging
    #[arg(long)]
    pub debug: bool,

    /// Filter target tables in <database>.<table> format
    #[arg(short, long)]
    pub target: Vec<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Preview configuration changes
    Plan {
        /// Show tables with no changes
        #[arg(long)]
        show_unchanged: bool,
    },
    /// Apply configuration changes
    Apply {
        /// Skip interactive approval
        #[arg(short, long)]
        auto_approve: bool,

        /// Show what would be done without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Export existing table definitions to local files
    Export {
        /// Overwrite existing files
        #[arg(long)]
        overwrite: bool,

        /// Output format
        #[arg(long, default_value = "standard")]
        format: String,
    },
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Plan { show_unchanged } => {
                plan::execute(&self.config, &self.target, *show_unchanged).await
            }
            Commands::Apply {
                auto_approve,
                dry_run,
            } => apply::execute(&self.config, &self.target, *auto_approve, *dry_run).await,
            Commands::Export { overwrite, format } => {
                export::execute(&self.config, &self.target, *overwrite, format).await
            }
        }
    }
}
