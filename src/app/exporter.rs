//! Exporter — generates the offline toolbox directory.

use std::path::Path;

use colored::Colorize;

use crate::app::{self, AppEntry, AppRuntimeEntry};
use crate::app::manifest::Manifest;
use crate::config::Config;
use crate::runtime::{self, RuntimeKind};

/// Export the toolbox to a directory.
///
/// Scans apps, collects selected ones, downloads installers, and
/// saves the manifest along with runtime archives.
pub fn export_to(
    output_dir: &Path,
    selected_apps: &[AppEntry],
    include_runtimes: bool,
) -> anyhow::Result<()> {
    let installers_dir = output_dir.join("installers");
    let portables_dir = output_dir.join("portables");
    let runtimes_dir = output_dir.join("runtimes");

    // Create output directory structure
    std::fs::create_dir_all(&installers_dir)?;
    std::fs::create_dir_all(&portables_dir)?;
    std::fs::create_dir_all(&runtimes_dir)?;

    println!("{} Exporting toolbox to {}", "📦".bold(), output_dir.display());

    // Export app installers
    let mut exported_apps = Vec::new();
    for app in selected_apps {
        if let Some(rule) = app::rules::resolve_rule(app) {
            if let Some(url) = rule.download_url.clone() {
                let filename = filename_from_rule_or_url(app, &rule, &url);
                match crate::runtime::download::download(&url, &installers_dir, &filename, None) {
                    Ok(path) => {
                        println!("  {} {} — {} saved", "✓".green(), app.name, path.display());
                        let mut exported = app.clone();
                        exported.install_path = Some(path.to_string_lossy().to_string());
                        exported.silent_args = app::silent_args_for_app(app);
                        exported_apps.push(exported);
                        continue;
                    }
                    Err(e) => {
                        println!("  {} {} — direct rule download failed: {}", "ℹ".yellow(), app.name, e);
                    }
                }
            }
        }

        match app.source {
            crate::app::AppSource::Winget => {
                let installer_path = download_winget_installer(app, &installers_dir);
                match installer_path {
                    Ok(path) => {
                        println!("  {} {} — {} saved", "✓".green(), app.name, path.display());
                        let mut exported = app.clone();
                        exported.install_path = Some(path.to_string_lossy().to_string());
                        exported.silent_args = app::silent_args_for_app(app);
                        exported_apps.push(exported);
                    }
                    Err(e) => {
                        println!("  {} {} — {} (will install via winget)", "ℹ".yellow(), app.name, e);
                        let mut exported = app.clone();
                        exported.silent_args = app::silent_args_for_app(app);
                        exported_apps.push(exported);
                    }
                }
            }
            crate::app::AppSource::Portable | crate::app::AppSource::Manual => {
                // Copy portable app directory
                if let Some(ref src_path) = app.install_path {
                    let src = Path::new(src_path);
                    if src.exists() {
                        let dest = portables_dir.join(&app.id);
                        copy_dir_recursive(src, &dest)?;
                        println!("  {} {} — portable copied", "✓".green(), app.name);
                        let mut exported = app.clone();
                        exported.install_path = Some(dest.to_string_lossy().to_string());
                        exported_apps.push(exported);
                    } else {
                        println!("  {} {} — source not found", "✗".red(), app.name);
                        exported_apps.push(app.clone());
                    }
                } else {
                    exported_apps.push(app.clone());
                }
            }
            _ => {
                let mut exported = app.clone();
                exported.silent_args = app::silent_args_for_app(app);
                exported_apps.push(exported);
            }
        }
    }

    // Export runtime archives
    let mut exported_runtimes = Vec::new();
    if include_runtimes {
        let config = Config::load()?;
        for kind in &[RuntimeKind::Node, RuntimeKind::Java, RuntimeKind::Maven] {
            let versions = runtime::list_installed(*kind)?;
            for v in &versions {
                let filename = runtime::url::archive_filename(*kind, &v.version);
                let url = runtime::url::build_download_url(*kind, &v.version, &config.core.mirror);
                let dest = runtimes_dir.join(&filename);

                if !dest.exists() {
                    println!("  {} Downloading {} v{}...", "📥".bold(), kind, v.version);
                    match runtime::download::download(&url, &runtimes_dir, &filename, None) {
                        Ok(path) => {
                            println!("  {} {} saved", "✓".green(), path.display());
                        }
                        Err(e) => {
                            println!("  {} Failed to download {}: {}", "✗".red(), kind, e);
                        }
                    }
                }

                exported_runtimes.push(AppRuntimeEntry {
                    tool: kind.to_string(),
                    version: v.version.clone(),
                    archive_path: Some(dest.to_string_lossy().to_string()),
                });
            }
        }
    }

    // Write manifest
    let manifest = Manifest::new(exported_apps.clone(), exported_runtimes.clone());
    let manifest_path = output_dir.join("manifest.toml");
    manifest.save(&manifest_path)?;

    // Write human-readable report
    let report_path = output_dir.join("apps.md");
    std::fs::write(&report_path, generate_apps_report(&exported_apps, &exported_runtimes))?;

    // Create zip archive next to the output directory
    let zip_path = output_dir.with_extension("zip");
    create_zip_archive(output_dir, &zip_path)?;

    println!();
    println!(
        "{} Toolbox exported to {}",
        "✅".green().bold(),
        output_dir.display()
    );
    println!("   Size: {}", format_size(dir_size(output_dir)?));
    println!("   Report: {}", report_path.display());
    println!("   Zip: {}", zip_path.display());

    Ok(())
}

/// Attempt to download a winget package's installer.
/// This uses winget's download feature when available, or falls back.
fn download_winget_installer(_app: &AppEntry, _target_dir: &Path) -> anyhow::Result<std::path::PathBuf> {
    // winget doesn't have a built-in download command in older versions.
    // For now, we'll attempt to use the internal download logic or mark for winget install.
    // Future: use `winget download` on newer Windows builds.
    anyhow::bail!("winget download not yet supported; will install via winget on target machine");
}

/// Recursively copy a directory.
fn filename_from_rule_or_url(app: &AppEntry, rule: &crate::app::rules::AppRule, url: &str) -> String {
    if let Some(installer_type) = &rule.installer_type {
        let ext = installer_type.trim().trim_start_matches('.');
        return format!("{}.{}", app.id.replace(' ', "_"), ext);
    }
    let no_query = url.split('?').next().unwrap_or(url);
    let last = no_query.rsplit('/').next().unwrap_or("installer.exe");
    if last.is_empty() || !last.contains('.') {
        format!("{}.exe", app.id.replace(' ', "_"))
    } else {
        last.to_string()
    }
}

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

/// Calculate directory size recursively.
fn dir_size(path: &Path) -> std::io::Result<u64> {
    let mut total = 0u64;
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                total += dir_size(&path)?;
            } else {
                total += std::fs::metadata(&path)?.len();
            }
        }
    }
    Ok(total)
}

/// Format bytes as human-readable size.
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;
    while size > 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

fn create_zip_archive(src_dir: &Path, zip_path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut zip, src_dir, src_dir, options)?;
    zip.finish()?;
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::SimpleFileOptions,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(base)?.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            if !name.is_empty() {
                zip.add_directory(format!("{}/", name), options)?;
            }
            add_dir_to_zip(zip, base, &path, options)?;
        } else {
            zip.start_file(name, options)?;
            let mut f = std::fs::File::open(&path)?;
            std::io::copy(&mut f, zip)?;
        }
    }
    Ok(())
}

fn generate_apps_report(apps: &[AppEntry], runtimes: &[AppRuntimeEntry]) -> String {
    let mut out = String::new();
    out.push_str("# windevkit Export Report\n\n");
    out.push_str(&format!("- Apps: {}\n", apps.len()));
    out.push_str(&format!("- Runtimes: {}\n\n", runtimes.len()));

    out.push_str("## Applications\n\n");
    out.push_str("| Name | Version | Source | Category | Local Path |\n");
    out.push_str("|---|---:|---|---|---|\n");
    for app in apps {
        let source = match app.source {
            crate::app::AppSource::Winget => "winget",
            crate::app::AppSource::Registry => "registry",
            crate::app::AppSource::Portable => "portable",
            crate::app::AppSource::Manual => "manual",
        };
        let category = app::category_for_app(app);
        let path = app.install_path.as_deref().unwrap_or("");
        out.push_str(&format!("| {} | {} | {} | {} | {} |\n", app.name.replace('|', "\\|"), app.version.replace('|', "\\|"), source, category.replace('|', "\\|"), path.replace('|', "\\|")));
    }

    out.push_str("\n## Runtimes\n\n");
    out.push_str("| Tool | Version | Archive |\n");
    out.push_str("|---|---:|---|\n");
    for rt in runtimes {
        let path = rt.archive_path.as_deref().unwrap_or("");
        out.push_str(&format!("| {} | {} | {} |\n", rt.tool.replace('|', "\\|"), rt.version.replace('|', "\\|"), path.replace('|', "\\|")));
    }
    out
}
