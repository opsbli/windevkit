//! Manifest serialization — read/write the toolbox manifest.toml.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::app::{AppEntry, AppRuntimeEntry};

/// The exported toolbox manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema version
    pub version: String,
    /// List of application entries
    pub apps: Vec<AppEntry>,
    /// List of runtime entries (for offline reinstall)
    pub runtimes: Vec<AppRuntimeEntry>,
    /// Timestamp of export
    pub exported_at: String,
}

impl Manifest {
    /// Create a new manifest.
    pub fn new(apps: Vec<AppEntry>, runtimes: Vec<AppRuntimeEntry>) -> Self {
        Self {
            version: "1.0".into(),
            apps,
            runtimes,
            exported_at: chrono_now(),
        }
    }

    /// Read manifest from a file path.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Manifest = toml::from_str(&content)?;
        Ok(manifest)
    }

    /// Write manifest to a file path.
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Get a simple timestamp string.
fn chrono_now() -> String {
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", start.as_secs())
}
