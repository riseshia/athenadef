use anyhow::Result;
use athenadef::cli::Cli;
use clap::Parser;
use console::Style;
use std::process;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing subscriber with debug level if --debug flag is set
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(log_level)),
        )
        .init();

    // Run the CLI and handle errors with better formatting
    if let Err(e) = cli.run().await {
        let error_style = Style::new().red().bold();
        eprintln!("\n{}", error_style.apply_to("Error:"));
        eprintln!("{}", e);

        // Print the error chain if available
        if e.chain().count() > 1 {
            eprintln!("\n{}", Style::new().dim().apply_to("Caused by:"));
            for cause in e.chain().skip(1) {
                eprintln!("  {}", Style::new().dim().apply_to(format!("{}", cause)));
            }
        }

        process::exit(1);
    }

    Ok(())
}
