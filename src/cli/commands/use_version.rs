use clap::Args;
use colored::Colorize;

use crate::runtime::{self, RuntimeKind};

#[derive(Args, Debug)]
pub struct UseArgs {
    /// Runtime name: node, java, maven
    pub tool: String,

    /// Version to activate
    pub version: String,
}

pub fn execute(args: &UseArgs) -> anyhow::Result<()> {
    let kind: RuntimeKind = args.tool.parse()?;

    println!(
        "{} Switching {} to {}...",
        "🔗".bold(),
        kind.to_string().bold(),
        args.version
    );

    runtime::activate(kind, &args.version)?;
    Ok(())
}
