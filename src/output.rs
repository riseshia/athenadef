use anyhow::Result;
use console::Style;

use crate::types::diff_result::{DiffOperation, DiffResult};

/// Styles for different types of output
pub struct OutputStyles {
    pub create: Style,
    pub update: Style,
    pub delete: Style,
    pub unchanged: Style,
    pub error: Style,
    pub success: Style,
    pub warning: Style,
    pub info: Style,
    pub bold: Style,
}

impl OutputStyles {
    pub fn new() -> Self {
        Self {
            create: Style::new().green().bold(),
            update: Style::new().yellow().bold(),
            delete: Style::new().red().bold(),
            unchanged: Style::new().dim(),
            error: Style::new().red().bold(),
            success: Style::new().green(),
            warning: Style::new().yellow(),
            info: Style::new().cyan(),
            bold: Style::new().bold(),
        }
    }
}

impl Default for OutputStyles {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a progress message
pub fn format_progress(message: &str) -> String {
    let style = Style::new().cyan();
    format!("{}", style.apply_to(message))
}

/// Format a success message
pub fn format_success(message: &str) -> String {
    let style = Style::new().green().bold();
    format!("{}", style.apply_to(message))
}

/// Format an error message
pub fn format_error(message: &str) -> String {
    let style = Style::new().red().bold();
    format!("{}", style.apply_to(message))
}

/// Format a warning message
pub fn format_warning(message: &str) -> String {
    let style = Style::new().yellow().bold();
    format!("{}", style.apply_to(message))
}

/// Format a create operation indicator
pub fn format_create() -> String {
    let style = Style::new().green().bold();
    format!("{}", style.apply_to("+"))
}

/// Format an update operation indicator
pub fn format_update() -> String {
    let style = Style::new().yellow().bold();
    format!("{}", style.apply_to("~"))
}

/// Format a delete operation indicator
pub fn format_delete() -> String {
    let style = Style::new().red().bold();
    format!("{}", style.apply_to("-"))
}

/// Format a table name
pub fn format_table_name(name: &str, is_bold: bool) -> String {
    if is_bold {
        let style = Style::new().bold();
        format!("{}", style.apply_to(name))
    } else {
        name.to_string()
    }
}

/// Display diff result in human-readable format
///
/// # Arguments
/// * `diff_result` - The diff result to display
/// * `show_unchanged` - Whether to show tables with no changes (only for plan command)
pub fn display_diff_result(diff_result: &DiffResult, show_unchanged: bool) -> Result<()> {
    let styles = OutputStyles::new();

    // Print summary with colors
    let summary_msg = format!(
        "Plan: {} to add, {} to change, {} to destroy.",
        diff_result.summary.to_add, diff_result.summary.to_change, diff_result.summary.to_destroy
    );
    println!("{}", styles.bold.apply_to(summary_msg));

    if diff_result.no_change {
        println!(
            "\n{}",
            styles
                .success
                .apply_to("No changes. Your infrastructure matches the configuration.")
        );
        return Ok(());
    }

    println!();

    // Collect databases that will be created (databases that only appear in Create operations)
    let mut databases_to_create: std::collections::HashSet<String> =
        std::collections::HashSet::new();
    for table_diff in &diff_result.table_diffs {
        if matches!(table_diff.operation, DiffOperation::Create) {
            databases_to_create.insert(table_diff.database_name.clone());
        }
    }

    // Display database creation notices first
    if !databases_to_create.is_empty() {
        let mut db_list: Vec<_> = databases_to_create.iter().collect();
        db_list.sort();
        for db in db_list {
            println!(
                "{} database: {}",
                format_create(),
                styles.create.apply_to(db)
            );
            println!("  Will create database if it does not exist");
            println!();
        }
    }

    // Display each table diff with color coding
    for table_diff in &diff_result.table_diffs {
        let qualified_name = table_diff.qualified_name();

        match table_diff.operation {
            DiffOperation::Create => {
                println!(
                    "{} {}",
                    format_create(),
                    styles.create.apply_to(&qualified_name)
                );
                println!("  Will create table");
                println!();
            }
            DiffOperation::Update => {
                println!(
                    "{} {}",
                    format_update(),
                    styles.update.apply_to(&qualified_name)
                );
                println!("  Will update table");
                if let Some(ref text_diff) = table_diff.text_diff {
                    // Color the diff lines
                    for line in text_diff.lines() {
                        if line.starts_with('+') && !line.starts_with("+++") {
                            println!("{}", styles.create.apply_to(line));
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            println!("{}", styles.delete.apply_to(line));
                        } else {
                            println!("{}", line);
                        }
                    }
                }
                println!();
            }
            DiffOperation::Delete => {
                println!(
                    "{} {}",
                    format_delete(),
                    styles.delete.apply_to(&qualified_name)
                );
                println!("  Will destroy table");
                println!();
            }
            DiffOperation::NoChange => {
                if show_unchanged {
                    println!("  {}", styles.unchanged.apply_to(&qualified_name));
                    println!("  No changes");
                    println!();
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_styles_new() {
        let _styles = OutputStyles::new();
        // Just verify that we can create the styles without errors
        // Styles don't have to_string() method, but we can verify they exist
    }

    #[test]
    fn test_format_progress() {
        let message = format_progress("Processing...");
        assert!(message.contains("Processing..."));
    }

    #[test]
    fn test_format_success() {
        let message = format_success("Success!");
        assert!(message.contains("Success!"));
    }

    #[test]
    fn test_format_error() {
        let message = format_error("Error occurred");
        assert!(message.contains("Error occurred"));
    }

    #[test]
    fn test_format_warning() {
        let message = format_warning("Warning message");
        assert!(message.contains("Warning message"));
    }

    #[test]
    fn test_format_operations() {
        assert!(!format_create().is_empty());
        assert!(!format_update().is_empty());
        assert!(!format_delete().is_empty());
    }

    #[test]
    fn test_format_table_name() {
        let name = format_table_name("test_table", false);
        assert_eq!(name, "test_table");

        let bold_name = format_table_name("test_table", true);
        assert!(bold_name.contains("test_table"));
    }
}
