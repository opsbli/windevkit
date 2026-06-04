//! Runtime management — Node.js, Java (JDK), Maven.
//!
//! Handles download, extraction, symlink activation, PATH management.

pub mod download;
pub mod extract;
pub mod path;
pub mod symlink;
pub mod url;

use std::path::{Path, PathBuf};

use colored::Colorize;

use crate::config::Config;

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
            _ => Err(anyhow::anyhow!(
                "Unknown runtime: {s}. Supported: node, java, maven"
            )),
        }
    }
}

/// A single installed version.
#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub kind: RuntimeKind,
    pub version: String,
    pub path: PathBuf,
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

/// Install a runtime version.
///
/// If `from` is `Some(path)`, copies from local file instead of downloading.
pub fn install(
    kind: RuntimeKind,
    version: &str,
    from: Option<&Path>,
    config: &Config,
) -> anyhow::Result<()> {
    let home = Config::home_dir();
    let version_dir = symlink::version_dir(&home, kind, version);
    let active_link = symlink::active_link_path(&home, kind);

    // Check if already installed
    if version_dir.exists() {
        tracing::info!("{} {} is already installed", kind, version);
        println!(
            "  {} {} {} already installed",
            "ℹ".yellow(),
            kind.to_string().bold(),
            version
        );
        return Ok(());
    }

    // Step 1: Download
    let archive_filename = url::archive_filename(kind, version);
    let cache_dir = download::cache_dir(&home);

    let archive_path = if let Some(local_path) = from {
        download::download("", &cache_dir, &archive_filename, Some(local_path))?
    } else {
        let download_url = url::build_download_url(kind, version, &config.core.mirror);
        println!(
            "  {} Downloading {} {}...",
            "📥".bold(),
            kind.to_string().bold(),
            version
        );
        download::download(&download_url, &cache_dir, &archive_filename, None)?
    };

    // Step 2: Extract
    println!(
        "  {} Extracting {} {}...",
        "📦".bold(),
        kind.to_string().bold(),
        version
    );
    extract::extract(&archive_path, version_dir.parent().unwrap_or(&home))?;

    // Rename extracted directory to v{version}
    let extracted = version_dir
        .parent()
        .unwrap_or(&home)
        .join(url::archive_root_dir(kind, version));
    if extracted.exists() && extracted != version_dir {
        std::fs::rename(&extracted, &version_dir)?;
    }

    // Step 3: Activate (set symlink)
    println!(
        "  {} Activating {} {}...",
        "🔗".bold(),
        kind.to_string().bold(),
        version
    );
    symlink::set_active(&active_link, &version_dir)?;

    // Step 4: Add to PATH if not already
    if !path::is_in_path(&home) {
        println!("  {} Adding to PATH...", "🔧".bold());
        let _ = path::snapshot_path(&home);
        path::add_to_path(&home)?;
    }

    // Update config with this version as default
    update_default_version(kind, version)?;

    println!(
        "  {} {} {}",
        "✅".green().bold(),
        format!("{} {}", kind, version).green().bold(),
        "installed and activated".green()
    );

    Ok(())
}

/// Activate (switch to) a specific version.
pub fn activate(kind: RuntimeKind, version: &str) -> anyhow::Result<()> {
    let home = Config::home_dir();
    let version_dir = symlink::version_dir(&home, kind, version);

    if !version_dir.exists() {
        anyhow::bail!(
            "{} {} {} is not installed. Use `windevkit install {} {}` first.",
            "✗".red(),
            kind,
            version,
            kind,
            version
        );
    }

    let active_link = symlink::active_link_path(&home, kind);
    symlink::set_active(&active_link, &version_dir)?;

    // Update config default
    update_default_version(kind, version)?;

    println!(
        "  {} Switched {} to {}",
        "✅".green().bold(),
        kind.to_string().bold(),
        version
    );

    Ok(())
}

/// List all installed versions of a runtime.
pub fn list_installed(kind: RuntimeKind) -> anyhow::Result<Vec<InstalledVersion>> {
    let home = Config::home_dir();
    let versions = symlink::list_installed(&home, kind)?;

    let result = versions
        .into_iter()
        .map(|(version, path)| {
            let active = symlink::is_active(&home, kind, &version);
            InstalledVersion {
                kind,
                version,
                path,
                active,
            }
        })
        .collect();

    Ok(result)
}

/// Remove a specific version.
pub fn uninstall(kind: RuntimeKind, version: &str) -> anyhow::Result<()> {
    let home = Config::home_dir();
    let version_dir = symlink::version_dir(&home, kind, version);
    let active_link = symlink::active_link_path(&home, kind);

    if !version_dir.exists() {
        anyhow::bail!("{} {} is not installed", kind, version);
    }

    // If this version is active, remove the symlink
    let is_active = symlink::is_active(&home, kind, version);
    if is_active {
        std::fs::remove_file(&active_link)?;
    }

    // Remove the version directory
    std::fs::remove_dir_all(&version_dir)?;

    // Check if there are other versions, and if so, activate the latest
    let remaining = symlink::list_installed(&home, kind)?;
    if let Some((latest_ver, _)) = remaining.first() {
        if is_active {
            let latest_dir = symlink::version_dir(&home, kind, latest_ver);
            symlink::set_active(&active_link, &latest_dir)?;
            update_default_version(kind, latest_ver)?;
            println!("  {} Auto-switched to {} {}", "🔄".bold(), kind, latest_ver);
        }
    } else {
        // No more versions, clean up PATH
        if is_active {
            path::remove_from_path(&home)?;
        }
        update_default_version(kind, "")?;
    }

    println!(
        "  {} {} {} uninstalled",
        "🗑️".bold(),
        kind.to_string().bold(),
        version
    );

    Ok(())
}

/// Get the "bin" directory for a runtime (where executables live).
pub fn bin_dir(kind: RuntimeKind, version_dir: &Path) -> PathBuf {
    match kind {
        RuntimeKind::Node => version_dir.to_path_buf(),
        RuntimeKind::Java => version_dir.join("bin"),
        RuntimeKind::Maven => version_dir.join("bin"),
    }
}

/// Update the default version in config for a runtime.
fn update_default_version(kind: RuntimeKind, version: &str) -> anyhow::Result<()> {
    let mut config = Config::load()?;
    match kind {
        RuntimeKind::Node => config.runtimes.node.default = version.to_string(),
        RuntimeKind::Java => config.runtimes.java.default = version.to_string(),
        RuntimeKind::Maven => config.runtimes.maven.default = version.to_string(),
    }
    config.save()?;
    Ok(())
}
