use console::Style;

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
