use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

use crate::config::Config;
use crate::runtime::{self, RuntimeKind};

#[derive(Args, Debug)]
pub struct InstallArgs {
    /// Runtime name: node, java, maven
    pub tool: String,

    /// Version to install (e.g., "22.11.0", or "latest")
    pub version: String,

    /// Install from local file instead of downloading
    #[arg(long)]
    pub from: Option<PathBuf>,
}

/// Execute install, using the default mirror from config.
pub fn execute(args: &InstallArgs) -> anyhow::Result<()> {
    let config = Config::load()?;
    execute_with(args, &config.core.mirror)
}

/// Execute install with an explicit mirror override.
pub fn execute_with(args: &InstallArgs, mirror: &str) -> anyhow::Result<()> {
    let kind: RuntimeKind = args.tool.parse()?;
    let mut config = Config::load()?;
    config.core.mirror = mirror.to_string();
    let from_path = args.from.as_deref();

    println!(
        "{} Installing {} {}...",
        "📦".bold(),
        kind.to_string().bold(),
        args.version
    );

    runtime::install(kind, &args.version, from_path, &config)?;
    Ok(())
}
