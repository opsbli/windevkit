use clap::Parser;
use windevkit::cli::Cli;

fn main() -> anyhow::Result<()> {
    // Initialize tracing/logging
    windevkit::cli::init_logging()?;

    // Parse CLI args and execute
    let cli = Cli::parse();
    cli.execute()?;

    Ok(())
}
