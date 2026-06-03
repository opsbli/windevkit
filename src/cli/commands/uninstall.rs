use clap::Args;
use colored::Colorize;

use crate::runtime::{self, RuntimeKind};

#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// Runtime name: node, java, maven
    pub tool: String,

    /// Version to uninstall. If omitted, requires --all
    pub version: Option<String>,

    /// Remove all installed versions of this runtime
    #[arg(long)]
    pub all: bool,
}

pub fn execute(args: &UninstallArgs) -> anyhow::Result<()> {
    let kind: RuntimeKind = args.tool.parse()?;

    if args.all {
        // Uninstall all versions
        let versions = runtime::list_installed(kind)?;
        if versions.is_empty() {
            println!("  {} No {} versions to remove.", "ℹ".yellow(), kind);
            return Ok(());
        }
        for v in &versions {
            runtime::uninstall(kind, &v.version)?;
        }
        return Ok(());
    }

    let version = match &args.version {
        Some(v) => v,
        None => anyhow::bail!(
            "Please specify a version to uninstall, or use --all to remove all versions."
        ),
    };

    println!(
        "{} Uninstalling {} {}...",
        "🗑️".bold(),
        kind.to_string().bold(),
        version
    );

    runtime::uninstall(kind, version)?;
    Ok(())
}
