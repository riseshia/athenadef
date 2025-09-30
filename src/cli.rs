use anyhow::Result;
use clap::{Parser, Subcommand};

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
                println!("Plan command (show_unchanged: {})", show_unchanged);
                Ok(())
            }
            Commands::Apply {
                auto_approve,
                dry_run,
            } => {
                println!(
                    "Apply command (auto_approve: {}, dry_run: {})",
                    auto_approve, dry_run
                );
                Ok(())
            }
            Commands::Export { overwrite, format } => {
                println!(
                    "Export command (overwrite: {}, format: {})",
                    overwrite, format
                );
                Ok(())
            }
        }
    }
}
