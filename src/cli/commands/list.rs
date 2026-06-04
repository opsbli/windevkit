use clap::Args;
use colored::Colorize;

use crate::runtime::{self, RuntimeKind};

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Runtime name: node, java, maven
    pub tool: String,
}

pub fn execute(args: &ListArgs) -> anyhow::Result<()> {
    let kind: RuntimeKind = args.tool.parse()?;
    let versions = runtime::list_installed(kind)?;

    if versions.is_empty() {
        println!(
            "  {} No {} versions installed.",
            "ℹ".yellow(),
            kind.to_string().bold()
        );
        println!("  Use: windevkit install {} <version>", kind);
        return Ok(());
    }

    println!(
        "{} Installed {} versions:",
        "📋".bold(),
        kind.to_string().bold()
    );

    for v in &versions {
        let active_mark = if v.active {
            format!(" {}", "← active".green().bold())
        } else {
            String::new()
        };
        println!("  {} v{}{}", "•".cyan(), v.version, active_mark);
    }

    Ok(())
}
