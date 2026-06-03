//! Importer — restores apps and runtimes from an exported toolbox.

use std::path::Path;

use colored::Colorize;

use crate::app::manifest::Manifest;
use crate::app::AppSource;

/// Import and restore from a toolbox directory.
///
/// If `interactive` is true, prompts the user before each action.
pub fn import_from(toolbox_dir: &Path, interactive: bool) -> anyhow::Result<()> {
    let manifest_path = toolbox_dir.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No manifest.toml found in {}. Is this a valid toolbox directory?",
            toolbox_dir.display()
        );
    }

    let manifest = Manifest::load(&manifest_path)?;
    println!("{} Importing toolbox from {}", "📥".bold(), toolbox_dir.display());
    println!("   Found {} apps, {} runtimes", manifest.apps.len(), manifest.runtimes.len());

    // Step 1: Install runtimes
    for rt in &manifest.runtimes {
        if let Some(ref archive_path) = rt.archive_path {
            let archive = Path::new(archive_path);
            if archive.exists() {
                println!(
                    "  {} Installing {} {} from local archive...",
                    "🔧".bold(),
                    rt.tool,
                    rt.version
                );
                let kind: crate::runtime::RuntimeKind = rt.tool.parse()?;
                let config = crate::config::Config::load()?;
                crate::runtime::install(kind, &rt.version, Some(archive), &config)?;
            }
        }
    }

    // Step 2: Install apps
    for app in &manifest.apps {
        if !app.selected {
            continue;
        }

        match app.source {
            AppSource::Winget => {
                // Try local installer first, then fall back to winget
                if let Some(ref path) = app.install_path {
                    let installer = Path::new(path);
                    if installer.exists() {
                        if interactive {
                            println!(
                                "  {} Install {} {}? [Y/n] ",
                                "❓".bold(),
                                app.name,
                                app.version
                            );
                            // Read user input
                            let input = read_yes_no(true);
                            if !input {
                                continue;
                            }
                        }
                        install_silent(installer, app.silent_args.as_deref())?;
                        println!("  {} {} installed", "✓".green(), app.name);
                    } else {
                        install_via_winget(app, interactive)?;
                    }
                } else {
                    install_via_winget(app, interactive)?;
                }
            }
            AppSource::Portable | AppSource::Manual => {
                // Copy portable app to target directory
                if let Some(ref path) = app.install_path {
                    let src = Path::new(path);
                    if src.exists() {
                        let target_dir = dirs::home_dir()
                            .unwrap_or_else(|| Path::new(".").to_path_buf())
                            .join("tools")
                            .join(&app.id);
                        if interactive {
                            println!(
                                "  {} Extract {} to {}? [Y/n] ",
                                "❓".bold(),
                                app.name,
                                target_dir.display()
                            );
                            if !read_yes_no(true) {
                                continue;
                            }
                        }
                        std::fs::create_dir_all(&target_dir)?;
                        copy_dir_recursive(src, &target_dir)?;
                        println!("  ✓ {} installed to {}", app.name, target_dir.display());
                    }
                }
            }
            _ => {
                // Registry-only apps: try winget as fallback
                install_via_winget(app, interactive)?;
            }
        }
    }

    println!();
    println!("{} Import complete!", "✅".green().bold());
    Ok(())
}

/// Install an app with silent arguments.
fn install_silent(installer: &Path, silent_args: Option<&str>) -> anyhow::Result<()> {
    let ext = installer
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let args = silent_args.unwrap_or("/S");

    match ext.to_lowercase().as_str() {
        "exe" => {
            let status = std::process::Command::new(installer)
                .args(args.split_whitespace())
                .status()?;
            if !status.success() {
                anyhow::bail!("Installer exited with code {:?}", status.code());
            }
        }
        "msi" => {
            let status = std::process::Command::new("msiexec")
                .args(["/i", &installer.to_string_lossy(), "/quiet", "/norestart"])
                .status()?;
            if !status.success() {
                anyhow::bail!("MSI installer exited with code {:?}", status.code());
            }
        }
        other => {
            anyhow::bail!("Unknown installer format: .{}", other);
        }
    }

    Ok(())
}

/// Install an app via winget.
fn install_via_winget(app: &crate::app::AppEntry, interactive: bool) -> anyhow::Result<()> {
    if interactive {
        println!(
            "  {} Install {} via winget? [Y/n] ",
            "❓".bold(),
            app.name
        );
        if !read_yes_no(true) {
            return Ok(());
        }
    }

    let status = std::process::Command::new("winget")
        .args(["install", "--id", &app.id, "--silent", "--accept-package-agreements", "--accept-source-agreements"])
        .status()?;

    if status.success() {
        println!("  {} {} installed via winget", "✓".green(), app.name);
    } else {
        println!("  {} {} failed to install via winget", "✗".red(), app.name);
    }

    Ok(())
}

/// Read a yes/no response from stdin.
fn read_yes_no(default: bool) -> bool {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
    }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
