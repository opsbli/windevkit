use std::path::PathBuf;
use clap::Args;

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

pub fn execute(args: &InstallArgs) -> anyhow::Result<()> {
    println!(
        "📦  Installing {} {}... (from: {:?})",
        args.tool, args.version, args.from
    );
    // TODO: implement
    Ok(())
}
