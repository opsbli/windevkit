//! Windows application management — scan, export, import.

pub mod exporter;
pub mod importer;
pub mod manifest;
pub mod scanner;

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

/// An application entry detected on or exported from a system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub source: AppSource,
    pub selected: bool,
    /// Path to the installer or portable app directory
    pub install_path: Option<String>,
    /// Silent install arguments (for known apps)
    pub silent_args: Option<String>,
}

/// A runtime entry for offline reinstall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppRuntimeEntry {
    pub tool: String,
    pub version: String,
    /// Path to the local archive file
    pub archive_path: Option<String>,
}

/// Scan the system for installed applications.
pub fn scan(exclude_patterns: &[String]) -> anyhow::Result<Vec<AppEntry>> {
    scanner::scan(exclude_patterns)
}

/// Export selected apps to a toolbox directory.
pub fn export_to(
    directory: &Path,
    apps: &[AppEntry],
    include_runtimes: bool,
) -> anyhow::Result<()> {
    exporter::export_to(directory, apps, include_runtimes)
}

/// Import and restore from a toolbox directory.
pub fn import_from(directory: &Path, interactive: bool) -> anyhow::Result<()> {
    importer::import_from(directory, interactive)
}
