//! Symlink management for version switching.

use std::path::{Path, PathBuf};

use super::RuntimeKind;

/// Get the path to the active symlink for a runtime.
pub fn active_link_path(home: &Path, kind: RuntimeKind) -> PathBuf {
    home.join("active").join(kind.to_string())
}

/// Get the path to the extracted version directory.
pub fn version_dir(home: &Path, kind: RuntimeKind, version: &str) -> PathBuf {
    home.join("versions")
        .join(kind.to_string())
        .join(format!("v{version}"))
}

/// Create or update the symlink for a runtime version.
///
/// `target` is the extracted runtime directory.
/// `link` is the symlink path under `active/`.
pub fn set_active(link: &Path, target: &Path) -> anyhow::Result<()> {
    // Ensure parent directory exists
    if let Some(parent) = link.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Remove existing symlink or directory at the link path
    if link.exists() || link.is_symlink() {
        remove_link(link)?;
    }

    // Create the symlink
    create_symlink(link, target)?;

    tracing::info!("Symlink created: {} → {}", link.display(), target.display());
    Ok(())
}

/// Remove a symlink.
pub fn remove_link(link: &Path) -> anyhow::Result<()> {
    if link.is_symlink() {
        std::fs::remove_file(link)?;
    } else if link.exists() {
        if link.is_dir() {
            std::fs::remove_dir_all(link)?;
        } else {
            std::fs::remove_file(link)?;
        }
    }
    Ok(())
}

/// Create a directory symlink on Windows.
#[cfg(windows)]
fn create_symlink(link: &Path, target: &Path) -> anyhow::Result<()> {
    use std::os::windows::fs as winfs;

    if target.is_dir() {
        winfs::symlink_dir(target, link)?;
    } else {
        winfs::symlink_file(target, link)?;
    }
    Ok(())
}

/// Create a directory symlink on Unix (for development/testing).
#[cfg(unix)]
fn create_symlink(link: &Path, target: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs as unixfs;
    unixfs::symlink(target, link)?;
    Ok(())
}

/// Read the target of a symlink.
pub fn read_link(link: &Path) -> Option<PathBuf> {
    if link.is_symlink() {
        std::fs::read_link(link).ok()
    } else {
        None
    }
}

/// Check if a specific version is the active one.
pub fn is_active(home: &Path, kind: RuntimeKind, version: &str) -> bool {
    let link = active_link_path(home, kind);
    if let Some(target) = read_link(&link) {
        let target_version_dir = version_dir(home, kind, version);
        target == target_version_dir
    } else {
        false
    }
}

/// List all installed versions for a runtime kind.
pub fn list_installed(home: &Path, kind: RuntimeKind) -> anyhow::Result<Vec<(String, PathBuf)>> {
    let versions_dir = home.join("versions").join(kind.to_string());
    if !versions_dir.exists() {
        return Ok(Vec::new());
    }

    let mut versions = Vec::new();
    for entry in std::fs::read_dir(&versions_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                // Strip the "v" prefix from directory names
                if let Some(ver) = dir_name.strip_prefix('v') {
                    versions.push((ver.to_string(), path));
                } else {
                    versions.push((dir_name.to_string(), path));
                }
            }
        }
    }

    // Sort by version descending (newest first)
    versions.sort_by(|a, b| {
        let va = semver_version(&a.0);
        let vb = semver_version(&b.0);
        vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(versions)
}

/// Parse a version string for rough comparison (not full semver, just sorting).
fn semver_version(v: &str) -> Vec<u32> {
    v.split('.')
        .filter_map(|part| part.parse::<u32>().ok())
        .collect()
}

/// Get the currently active version for a runtime.
pub fn get_active_version(home: &Path, kind: RuntimeKind) -> Option<String> {
    let link = active_link_path(home, kind);
    let target = read_link(&link)?;

    // Extract version from the directory path
    // Path looks like: ~/.windevkit/versions/node/v22.11.0
    if let Some(dir_name) = target.file_name().and_then(|n| n.to_str()) {
        Some(dir_name.trim_start_matches('v').to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parsing() {
        assert_eq!(semver_version("22.11.0"), vec![22, 11, 0]);
        assert_eq!(semver_version("21.0.3"), vec![21, 0, 3]);
        assert_eq!(semver_version("3.9.6"), vec![3, 9, 6]);
        assert_eq!(semver_version("invalid"), Vec::<u32>::new());
    }
}
