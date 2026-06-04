use colored::Colorize;

use crate::config::Config;

pub fn execute() -> anyhow::Result<()> {
    let home = Config::home_dir();
    let backups_dir = home.join("backups");

    if !backups_dir.exists() {
        anyhow::bail!("No PATH snapshots found at {}", backups_dir.display());
    }

    // List available snapshots
    let mut snapshots: Vec<_> = std::fs::read_dir(&backups_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("txt"))
        .collect();

    snapshots.sort();
    snapshots.reverse();

    if snapshots.is_empty() {
        anyhow::bail!("No PATH snapshots found at {}", backups_dir.display());
    }

    // Use the most recent snapshot
    let latest = &snapshots[0];
    let content = std::fs::read_to_string(latest)?;
    let path_value = content.trim();

    println!("{} Restoring PATH from snapshot...", "⏪".bold());
    println!("   Snapshot: {}", latest.display());
    println!("   PATH length: {} entries", path_value.split(';').count());

    // Write the snapshot back to registry
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let status = std::process::Command::new("reg")
        .args([
            "add",
            "HKCU\\Environment",
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
            path_value,
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;

    if status.success() {
        println!("  {} PATH restored from snapshot.", "✓".green().bold());
        println!(
            "  {} Restart your terminal for changes to take effect.",
            "💡".yellow()
        );
    } else {
        println!("  {} Failed to restore PATH", "✗".red());
    }

    Ok(())
}
