//! Archive extraction — ZIP and TGZ support.

use std::path::{Path, PathBuf};
use indicatif::{ProgressBar, ProgressStyle};

/// Extract an archive to the target directory.
///
/// Returns the path to the extracted root directory.
/// Supports `.zip` and `.tar.gz`/`.tgz` archives.
pub fn extract(archive: &Path, target_dir: &Path) -> anyhow::Result<PathBuf> {
    if !archive.exists() {
        anyhow::bail!("Archive not found: {}", archive.display());
    }

    let file_name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    tracing::info!("Extracting {} to {}", file_name, target_dir.display());
    std::fs::create_dir_all(target_dir)?;

    let ext = archive
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "zip" => extract_zip(archive, target_dir),
        "gz" | "tgz" => extract_tgz(archive, target_dir),
        other => anyhow::bail!("Unsupported archive format: .{} (supported: .zip, .tar.gz)", other),
    }
}

/// Extract a ZIP archive.
fn extract_zip(archive: &Path, target_dir: &Path) -> anyhow::Result<PathBuf> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    let total = zip.len();
    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Track the first directory entry to determine root
    let mut root_dir: Option<PathBuf> = None;

    for i in 0..total {
        let mut entry = zip.by_index(i)?;
        let entry_path = entry
            .name()
            .trim_start_matches('/')
            .to_string();

        // Determine root directory from first entry
        if root_dir.is_none() {
            if let Some(slash) = entry_path.find('/') {
                root_dir = Some(PathBuf::from(&entry_path[..slash]));
            }
        }

        let target_path = target_dir.join(&entry_path);

        if entry.is_dir() {
            std::fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&target_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }

        pb.inc(1);
    }

    pb.finish_and_clear();
    tracing::info!("Extracted {} files from zip", total);

    Ok(root_dir.unwrap_or_else(|| PathBuf::from(".")))
}

/// Extract a tar.gz archive.
fn extract_tgz(archive: &Path, target_dir: &Path) -> anyhow::Result<PathBuf> {
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive_tar = tar::Archive::new(decoder);

    let mut root_dir: Option<PathBuf> = None;

    for entry in archive_tar.entries()? {
        let mut entry = entry?;
        let entry_path = entry
            .path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string();

        if root_dir.is_none() && entry_path.contains('/') {
            if let Some(slash) = entry_path.find('/') {
                root_dir = Some(PathBuf::from(&entry_path[..slash]));
            }
        }

        let target_path = target_dir.join(&entry_path);

        if entry.header().entry_type().is_dir() {
            std::fs::create_dir_all(&target_path)?;
        } else {
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            entry.unpack(&target_path)?;
        }
    }

    tracing::info!("Extracted tgz archive to {}", target_dir.display());
    Ok(root_dir.unwrap_or_else(|| PathBuf::from(".")))
}
