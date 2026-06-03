//! HTTP download with progress bar and local file support.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use indicatif::{ProgressBar, ProgressStyle};

/// Download a runtime archive.
///
/// If `from` is `Some(path)`, the file is copied locally instead of downloaded.
/// `target_dir` is where the downloaded/copied file will be placed.
pub fn download(
    url: &str,
    target_dir: &Path,
    filename: &str,
    from: Option<&Path>,
) -> anyhow::Result<PathBuf> {
    let target_path = target_dir.join(filename);

    // Ensure target directory exists
    std::fs::create_dir_all(target_dir)?;

    if let Some(src) = from {
        // Local file copy
        if !src.exists() {
            anyhow::bail!("Local file not found: {}", src.display());
        }
        tracing::info!("Copying from local file: {}", src.display());
        std::fs::copy(src, &target_path)?;
        return Ok(target_path);
    }

    // HTTP download
    tracing::info!("Downloading: {}", url);
    download_http(url, &target_path)?;
    Ok(target_path)
}

/// Download a file via HTTP with a progress bar.
fn download_http(url: &str, target: &Path) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("windevkit/0.1.0")
        .build()?;

    let response = client.get(url).send()?;
    let total_size = response.content_length().unwrap_or(0);

    // Set up progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Stream download with progress updates
    let mut source = response;
    let mut dest = std::fs::File::create(target)?;
    let mut downloaded: u64 = 0;
    let mut buffer = [0u8; 8192];

    loop {
        let len = source.read(&mut buffer)?;
        if len == 0 {
            break;
        }
        dest.write_all(&buffer[..len])?;
        downloaded += len as u64;
        pb.set_position(downloaded);
    }

    pb.finish_and_clear();
    tracing::info!("Downloaded {} bytes to {}", downloaded, target.display());
    Ok(())
}

/// Get the cache directory for downloads.
pub fn cache_dir(home: &Path) -> PathBuf {
    home.join("cache").join("downloads")
}
