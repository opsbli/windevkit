//! PATH backup/restore — snapshots PATH before mutations, enables rollback.

use std::path::PathBuf;

/// Snapshot the current PATH to a backup file.
pub fn snapshot_path() -> anyhow::Result<PathBuf> {
    // TODO: implement
    anyhow::bail!("snapshot not yet implemented")
}

/// Restore PATH from the most recent snapshot.
pub fn restore_path() -> anyhow::Result<()> {
    // TODO: implement
    anyhow::bail!("restore not yet implemented")
}

/// List all available PATH snapshots.
pub fn list_snapshots() -> anyhow::Result<Vec<PathBuf>> {
    // TODO: implement
    Ok(Vec::new())
}
