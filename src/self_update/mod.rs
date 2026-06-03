//! Self-update mechanism — checks GitHub Releases and replaces the current binary.

use std::io::{Read, Write};
use std::path::PathBuf;

use colored::Colorize;

const REPO_OWNER: &str = "opsbli";
const REPO_NAME: &str = "windevkit";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the self-update check and apply if a newer version is found.
pub fn run() -> anyhow::Result<()> {
    println!("{} Checking for updates...", "🔄".bold());

    let latest = match fetch_latest_release() {
        Ok(v) => v,
        Err(e) => {
            println!("  {} Failed to check for updates: {}", "✗".red(), e);
            println!("  Check manually: https://github.com/{}/{}", REPO_OWNER, REPO_NAME);
            return Ok(());
        }
    };

    if latest == CURRENT_VERSION {
        println!(
            "  {} You're already on the latest version ({})",
            "✓".green(),
            CURRENT_VERSION
        );
        return Ok(());
    }

    println!(
        "  {} New version available: {} (current: {})",
        "📢".yellow(),
        latest.green().bold(),
        CURRENT_VERSION
    );

    // Download and install
    let asset_name = format!("windevkit-x86_64-pc-windows-msvc.zip");
    let download_url = format!(
        "https://github.com/{owner}/{repo}/releases/download/v{version}/{asset}",
        owner = REPO_OWNER,
        repo = REPO_NAME,
        version = latest,
        asset = asset_name
    );

    println!("  {} Downloading {}...", "📥".bold(), asset_name);
    let temp_dir = std::env::temp_dir().join("windevkit-update");
    std::fs::create_dir_all(&temp_dir)?;

    let zip_path = temp_dir.join(&asset_name);
    download_file(&download_url, &zip_path)?;

    // Extract the zip
    println!("  {} Extracting...", "📦".bold());
    let file = std::fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut extracted_exe: Option<PathBuf> = None;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        if name.ends_with(".exe") || name.ends_with(".exe") {
            let out_path = temp_dir.join(&name);
            let mut outfile = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
            extracted_exe = Some(out_path);
        }
    }

    let new_exe = extracted_exe.ok_or_else(|| anyhow::anyhow!("No executable found in release archive"))?;

    // Replace the current executable
    let current_exe = std::env::current_exe()?;
    let old_exe = current_exe.with_extension("old");

    // Remove any previous .old file
    if old_exe.exists() {
        std::fs::remove_file(&old_exe)?;
    }

    // Rename current exe to .old
    std::fs::rename(&current_exe, &old_exe)?;

    // Copy new exe to current location
    std::fs::copy(&new_exe, &current_exe)?;

    println!("  {} Updated to v{}!", "✅".green().bold(), latest);

    // Clean up temp files
    let _ = std::fs::remove_dir_all(&temp_dir);

    println!(
        "  {} Restart windevkit to use the new version.",
        "💡".yellow()
    );
    println!(
        "  {} Previous version saved as: {}",
        "📁".dimmed(),
        old_exe.display()
    );

    Ok(())
}

/// Fetch the latest release version from GitHub API.
fn fetch_latest_release() -> Result<String, anyhow::Error> {
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/releases/latest",
        owner = REPO_OWNER,
        repo = REPO_NAME
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("windevkit/0.1.0")
        .build()?;

    let response = client.get(&url).send()?;
    let body: serde_json::Value = response.json()?;

    let tag_name = body["tag_name"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Could not parse release tag"))?;

    // Strip leading "v" from tag
    Ok(tag_name.trim_start_matches('v').to_string())
}

/// Download a file via HTTP.
fn download_file(url: &str, path: &PathBuf) -> anyhow::Result<()> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("windevkit/0.1.0")
        .build()?;

    let response = client.get(url).send()?;
    let total_size = response.content_length().unwrap_or(0);

    let pb = indicatif::ProgressBar::new(total_size);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let mut source = response;
    let mut dest = std::fs::File::create(path)?;
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
    Ok(())
}
