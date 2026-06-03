//! Windows application management — scan, export, import.

use std::path::Path;
use serde::{Deserialize, Serialize};

/// Source of an application entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppSource {
    Winget,
    Registry,
    Portable,
    Manual,
}

/// An application entry detected on the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: AppSource,
    pub selected: bool,
    /// Path to installed location (for portable apps)
    pub install_path: Option<String>,
    /// Silent install arguments (for known apps)
    pub silent_args: Option<String>,
}

/// The exported manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub apps: Vec<AppEntry>,
    pub runtimes: Vec<RuntimeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub tool: String,
    pub version: String,
    pub archive_path: Option<String>,
}

/// Scan the system for installed applications.
pub fn scan() -> anyhow::Result<Vec<AppEntry>> {
    // TODO: implement winget + registry scan
    Ok(Vec::new())
}

/// Export selected apps to a toolbox directory.
pub fn export_to(directory: &Path, apps: &[AppEntry], runtimes: &[RuntimeEntry]) -> anyhow::Result<Manifest> {
    // TODO: implement
    let _ = (directory, apps, runtimes);
    anyhow::bail!("export not yet implemented")
}

/// Import and restore from a toolbox directory.
pub fn import_from(directory: &Path, interactive: bool) -> anyhow::Result<()> {
    // TODO: implement
    let _ = (directory, interactive);
    anyhow::bail!("import not yet implemented")
}
