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

    /// Filter target tables in `<database>.<table>` format
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

        /// Output in JSON format
        #[arg(long)]
        json: bool,
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
            Commands::Plan {
                show_unchanged,
                json,
            } => plan::execute(&self.config, &self.target, *show_unchanged, *json).await,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_default_config() {
        let args = vec!["athenadef", "plan"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.config, "athenadef.yaml");
        assert!(!cli.debug);
        assert_eq!(cli.target.len(), 0);
    }

    #[test]
    fn test_cli_custom_config() {
        let args = vec!["athenadef", "--config", "custom.yaml", "plan"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.config, "custom.yaml");
    }

    #[test]
    fn test_cli_debug_flag() {
        let args = vec!["athenadef", "--debug", "plan"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.debug);
    }

    #[test]
    fn test_cli_target_single() {
        let args = vec!["athenadef", "--target", "salesdb.customers", "plan"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.target.len(), 1);
        assert_eq!(cli.target[0], "salesdb.customers");
    }

    #[test]
    fn test_cli_target_multiple() {
        let args = vec![
            "athenadef",
            "--target",
            "salesdb.*",
            "--target",
            "marketingdb.leads",
            "plan",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.target.len(), 2);
        assert_eq!(cli.target[0], "salesdb.*");
        assert_eq!(cli.target[1], "marketingdb.leads");
    }

    #[test]
    fn test_cli_plan_command() {
        let args = vec!["athenadef", "plan"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan {
                show_unchanged,
                json,
            } => {
                assert!(!show_unchanged);
                assert!(!json);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_plan_command_with_flags() {
        let args = vec!["athenadef", "plan", "--show-unchanged", "--json"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan {
                show_unchanged,
                json,
            } => {
                assert!(show_unchanged);
                assert!(json);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_apply_command() {
        let args = vec!["athenadef", "apply"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Apply {
                auto_approve,
                dry_run,
            } => {
                assert!(!auto_approve);
                assert!(!dry_run);
            }
            _ => panic!("Expected Apply command"),
        }
    }

    #[test]
    fn test_cli_apply_command_with_flags() {
        let args = vec!["athenadef", "apply", "--auto-approve", "--dry-run"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Apply {
                auto_approve,
                dry_run,
            } => {
                assert!(auto_approve);
                assert!(dry_run);
            }
            _ => panic!("Expected Apply command"),
        }
    }

    #[test]
    fn test_cli_apply_command_short_flag() {
        let args = vec!["athenadef", "apply", "-a"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Apply {
                auto_approve,
                dry_run,
            } => {
                assert!(auto_approve);
                assert!(!dry_run);
            }
            _ => panic!("Expected Apply command"),
        }
    }

    #[test]
    fn test_cli_export_command() {
        let args = vec!["athenadef", "export"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Export { overwrite, format } => {
                assert!(!overwrite);
                assert_eq!(format, "standard");
            }
            _ => panic!("Expected Export command"),
        }
    }

    #[test]
    fn test_cli_export_command_with_flags() {
        let args = vec!["athenadef", "export", "--overwrite", "--format", "compact"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Export { overwrite, format } => {
                assert!(overwrite);
                assert_eq!(format, "compact");
            }
            _ => panic!("Expected Export command"),
        }
    }

    #[test]
    fn test_cli_combined_flags() {
        let args = vec![
            "athenadef",
            "--config",
            "prod.yaml",
            "--debug",
            "--target",
            "db.table",
            "plan",
            "--json",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.config, "prod.yaml");
        assert!(cli.debug);
        assert_eq!(cli.target.len(), 1);
        match cli.command {
            Commands::Plan {
                show_unchanged,
                json,
            } => {
                assert!(!show_unchanged);
                assert!(json);
            }
            _ => panic!("Expected Plan command"),
        }
    }
}
