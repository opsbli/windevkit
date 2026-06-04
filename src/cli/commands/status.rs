use colored::Colorize;

use crate::config::Config;
use crate::runtime::{self, RuntimeKind};

pub fn execute(verbose: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let home = Config::home_dir();

    println!();
    println!("{}", "📊 windevkit status".bold());
    println!("{}", "═".repeat(50).dimmed());
    println!("  Home:       {}", home.display().to_string().cyan());
    println!("  Mirror:     {}", config.core.mirror.cyan());
    println!(
        "  Dev Mode:   {}",
        if config.core.dev_mode {
            "Enabled".green()
        } else {
            "Disabled".yellow()
        }
    );

    // Check if home exists
    if !home.exists() {
        println!();
        println!(
            "  {} windevkit is not initialized. Run `windevkit init` first.",
            "⚠".yellow()
        );
        return Ok(());
    }

    println!();

    // Show runtimes
    for kind in &[RuntimeKind::Node, RuntimeKind::Java, RuntimeKind::Maven] {
        let versions = runtime::list_installed(*kind)?;
        let active = versions.iter().find(|v| v.active);
        let count = versions.len();

        if count > 0 {
            println!(
                "  {}  {} ({} installed)",
                kind_icon(*kind),
                kind.to_string().bold(),
                count
            );
            if let Some(a) = active {
                println!("     └─ Active: v{} {}", a.version, "✓".green());
            }
            if verbose {
                for v in &versions {
                    let prefix = if v.active { "  ✅" } else { "    " };
                    println!("     {} v{}", prefix, v.version);
                }
            }
        } else {
            println!(
                "  {}  {} {}",
                kind_icon(*kind),
                kind.to_string().bold(),
                "— not installed".dimmed()
            );
        }
    }

    println!();
    println!("{}", "💡 Tips:".yellow());
    println!("  Install:  windevkit install node 22.11.0");
    println!("  Switch:   windevkit use node 22.11.0");
    println!("  Apps:     windevkit app scan");
    println!("  Diagnose: windevkit doctor");

    Ok(())
}

fn kind_icon(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::Node => "🟢",
        RuntimeKind::Java => "🟠",
        RuntimeKind::Maven => "🔵",
    }
}
