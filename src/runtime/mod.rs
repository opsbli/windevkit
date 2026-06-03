//! Runtime management — Node.js, Java (JDK), Maven.
//!
//! Handles download, extraction, symlink activation, PATH management.

use std::path::Path;

/// Supported runtime kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Node,
    Java,
    Maven,
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeKind::Node => write!(f, "node"),
            RuntimeKind::Java => write!(f, "java"),
            RuntimeKind::Maven => write!(f, "maven"),
        }
    }
}

impl std::str::FromStr for RuntimeKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" => Ok(RuntimeKind::Node),
            "java" | "jdk" => Ok(RuntimeKind::Java),
            "maven" | "mv" => Ok(RuntimeKind::Maven),
            _ => Err(anyhow::anyhow!("Unknown runtime: {}. Supported: node, java, maven", s)),
        }
    }
}

/// A single installed version.
#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub kind: RuntimeKind,
    pub version: String,
    pub path: std::path::PathBuf,
    pub active: bool,
}

/// Errors specific to runtime operations.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Version {version} of {tool} is not installed")]
    NotInstalled { tool: String, version: String },
    #[error("Version {version} of {tool} is already installed")]
    AlreadyInstalled { tool: String, version: String },
    #[error("Failed to download {url}: {message}")]
    DownloadFailed { url: String, message: String },
    #[error("Failed to create symlink: {0}")]
    SymlinkFailed(String),
}

/// Download a runtime version.
pub fn download(kind: RuntimeKind, version: &str, mirror: &str) -> anyhow::Result<std::path::PathBuf> {
    // TODO: implement
    let _ = (kind, version, mirror);
    anyhow::bail!("download not yet implemented")
}

/// Extract a downloaded archive to the versions directory.
pub fn extract(kind: RuntimeKind, version: &str, archive: &Path) -> anyhow::Result<std::path::PathBuf> {
    // TODO: implement
    let _ = (kind, version, archive);
    anyhow::bail!("extract not yet implemented")
}

/// Activate a specific version (update symlink).
pub fn activate(kind: RuntimeKind, version: &str) -> anyhow::Result<()> {
    // TODO: implement
    let _ = (kind, version);
    anyhow::bail!("activate not yet implemented")
}

/// List all installed versions of a runtime.
pub fn list_installed(kind: RuntimeKind) -> anyhow::Result<Vec<InstalledVersion>> {
    // TODO: implement
    let _ = kind;
    Ok(Vec::new())
}

/// Remove a specific version.
pub fn uninstall(kind: RuntimeKind, version: &str) -> anyhow::Result<()> {
    // TODO: implement
    let _ = (kind, version);
    anyhow::bail!("uninstall not yet implemented")
}
