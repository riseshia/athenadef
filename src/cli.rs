use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{apply, export, init, plan};

#[derive(Parser, Debug)]
#[command(name = "athenadef")]
#[command(version, about = "AWS Athena schema management tool", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new configuration file
    ///
    /// Creates a default athenadef.yaml configuration file with helpful comments.
    /// This is typically the first command you run when setting up athenadef.
    ///
    /// Examples:
    ///   athenadef init
    ///   athenadef init --force
    Init {
        /// Config file path
        #[arg(short, long, default_value = "athenadef.yaml")]
        config: String,

        /// Enable debug logging
        #[arg(long)]
        debug: bool,

        /// Overwrite existing configuration file
        ///
        /// By default, init will fail if athenadef.yaml already exists to prevent
        /// accidental overwrites. Use this flag to replace an existing file.
        #[arg(long)]
        force: bool,
    },
    /// Preview configuration changes
    ///
    /// Calculates the differences between your local schema definitions and the current state
    /// in AWS Athena, displaying what changes would be made without executing them.
    ///
    /// Examples:
    ///   athenadef plan
    ///   athenadef plan --target salesdb.customers
    ///   athenadef plan --json > changes.json
    Plan {
        /// Config file path
        #[arg(short, long, default_value = "athenadef.yaml")]
        config: String,

        /// Enable debug logging
        #[arg(long)]
        debug: bool,

        /// Filter target tables in `<database>.<table>` format
        ///
        /// Can be used multiple times to specify multiple targets.
        /// Supports wildcards: `salesdb.*` (all tables in database) or `*.customers` (table across databases).
        #[arg(short, long)]
        target: Vec<String>,

        /// Show tables with no changes
        ///
        /// By default, only tables with changes are displayed. Use this flag to also show
        /// tables that match the remote state.
        #[arg(long)]
        show_unchanged: bool,

        /// Output in JSON format
        ///
        /// Outputs the diff result as structured JSON instead of human-readable text.
        /// Useful for programmatic processing or integration with other tools.
        #[arg(long)]
        json: bool,
    },
    /// Apply configuration changes
    ///
    /// Executes the changes needed to make your AWS Athena schema match your local definitions.
    /// This will create, update, or delete tables as needed. By default, prompts for confirmation
    /// before making changes.
    ///
    /// Examples:
    ///   athenadef apply
    ///   athenadef apply --auto-approve
    ///   athenadef apply --dry-run --target salesdb.*
    Apply {
        /// Config file path
        #[arg(short, long, default_value = "athenadef.yaml")]
        config: String,

        /// Enable debug logging
        #[arg(long)]
        debug: bool,

        /// Filter target tables in `<database>.<table>` format
        ///
        /// Can be used multiple times to specify multiple targets.
        /// Supports wildcards: `salesdb.*` (all tables in database) or `*.customers` (table across databases).
        #[arg(short, long)]
        target: Vec<String>,

        /// Skip interactive approval
        ///
        /// Automatically approves and applies all changes without prompting for confirmation.
        /// Use with caution in production environments.
        #[arg(short, long)]
        auto_approve: bool,

        /// Show what would be done without executing
        ///
        /// Performs all the planning and validation but skips the actual execution.
        /// Similar to 'plan' but follows the apply workflow.
        #[arg(long)]
        dry_run: bool,
    },
    /// Export existing table definitions to local files
    ///
    /// Retrieves table definitions from AWS Athena and saves them as SQL files in your
    /// local directory structure (database_name/table_name.sql).
    ///
    /// Examples:
    ///   athenadef export
    ///   athenadef export --overwrite
    ///   athenadef export --target salesdb.*
    Export {
        /// Config file path
        #[arg(short, long, default_value = "athenadef.yaml")]
        config: String,

        /// Enable debug logging
        #[arg(long)]
        debug: bool,

        /// Filter target tables in `<database>.<table>` format
        ///
        /// Can be used multiple times to specify multiple targets.
        /// Supports wildcards: `salesdb.*` (all tables in database) or `*.customers` (table across databases).
        #[arg(short, long)]
        target: Vec<String>,

        /// Overwrite existing files
        ///
        /// By default, existing files are skipped to prevent accidental overwrites.
        /// Use this flag to replace existing files with the remote definitions.
        #[arg(long)]
        overwrite: bool,
    },
}

impl Cli {
    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Commands::Init {
                config,
                debug: _,
                force,
            } => init::execute(config, *force).await,
            Commands::Plan {
                config,
                debug: _,
                target,
                show_unchanged,
                json,
            } => plan::execute(config, target, *show_unchanged, *json).await,
            Commands::Apply {
                config,
                debug: _,
                target,
                auto_approve,
                dry_run,
            } => apply::execute(config, target, *auto_approve, *dry_run).await,
            Commands::Export {
                config,
                debug: _,
                target,
                overwrite,
            } => export::execute(config, target, *overwrite).await,
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
        match cli.command {
            Commands::Plan { config, debug, .. } => {
                assert_eq!(config, "athenadef.yaml");
                assert!(!debug);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_custom_config() {
        let args = vec!["athenadef", "plan", "--config", "custom.yaml"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan { config, .. } => {
                assert_eq!(config, "custom.yaml");
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_debug_flag() {
        let args = vec!["athenadef", "plan", "--debug"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan { debug, .. } => {
                assert!(debug);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_target_single() {
        let args = vec!["athenadef", "plan", "--target", "salesdb.customers"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan { target, .. } => {
                assert_eq!(target.len(), 1);
                assert_eq!(target[0], "salesdb.customers");
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_target_multiple() {
        let args = vec![
            "athenadef",
            "plan",
            "--target",
            "salesdb.*",
            "--target",
            "marketingdb.leads",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan { target, .. } => {
                assert_eq!(target.len(), 2);
                assert_eq!(target[0], "salesdb.*");
                assert_eq!(target[1], "marketingdb.leads");
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_plan_command() {
        let args = vec!["athenadef", "plan"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan {
                target,
                show_unchanged,
                json,
                ..
            } => {
                assert_eq!(target.len(), 0);
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
                target,
                show_unchanged,
                json,
                ..
            } => {
                assert_eq!(target.len(), 0);
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
                target,
                auto_approve,
                dry_run,
                ..
            } => {
                assert_eq!(target.len(), 0);
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
                target,
                auto_approve,
                dry_run,
                ..
            } => {
                assert_eq!(target.len(), 0);
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
                target,
                auto_approve,
                dry_run,
                ..
            } => {
                assert_eq!(target.len(), 0);
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
            Commands::Export {
                target, overwrite, ..
            } => {
                assert_eq!(target.len(), 0);
                assert!(!overwrite);
            }
            _ => panic!("Expected Export command"),
        }
    }

    #[test]
    fn test_cli_export_command_with_flags() {
        let args = vec!["athenadef", "export", "--overwrite"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Export {
                target, overwrite, ..
            } => {
                assert_eq!(target.len(), 0);
                assert!(overwrite);
            }
            _ => panic!("Expected Export command"),
        }
    }

    #[test]
    fn test_cli_combined_flags() {
        let args = vec![
            "athenadef",
            "plan",
            "--config",
            "prod.yaml",
            "--debug",
            "--target",
            "db.table",
            "--json",
        ];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Plan {
                config,
                debug,
                target,
                show_unchanged,
                json,
            } => {
                assert_eq!(config, "prod.yaml");
                assert!(debug);
                assert_eq!(target.len(), 1);
                assert_eq!(target[0], "db.table");
                assert!(!show_unchanged);
                assert!(json);
            }
            _ => panic!("Expected Plan command"),
        }
    }

    #[test]
    fn test_cli_init_command() {
        let args = vec!["athenadef", "init"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Init {
                config,
                debug,
                force,
            } => {
                assert_eq!(config, "athenadef.yaml");
                assert!(!debug);
                assert!(!force);
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_init_command_with_force() {
        let args = vec!["athenadef", "init", "--force"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Commands::Init {
                config,
                debug,
                force,
            } => {
                assert_eq!(config, "athenadef.yaml");
                assert!(!debug);
                assert!(force);
            }
            _ => panic!("Expected Init command"),
        }
    }
}
