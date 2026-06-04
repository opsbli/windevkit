//! Importer — restores apps and runtimes from an exported toolbox.

use std::path::{Path, PathBuf};

use colored::Colorize;
use inquire::Select;

use crate::app::manifest::Manifest;
use crate::app::{AppEntry, AppSource, AppRuntimeEntry};

#[derive(Debug, Clone)]
struct ImportSummary {
    runtime_results: Vec<ItemResult>,
    app_results: Vec<ItemResult>,
}

#[derive(Debug, Clone)]
struct ItemResult {
    kind: &'static str,
    name: String,
    version: String,
    status: ItemStatus,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ItemStatus {
    Success,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FailureAction {
    Retry,
    Skip,
    Abort,
}

/// Import and restore from a toolbox directory.
///
/// If `interactive` is true, prompts the user before each action and on failure.
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

    let mut summary = ImportSummary {
        runtime_results: Vec::new(),
        app_results: Vec::new(),
    };

    for rt in &manifest.runtimes {
        let result = import_runtime(rt, interactive)?;
        summary.runtime_results.push(result);
    }

    for app in &manifest.apps {
        if !app.selected {
            summary.app_results.push(ItemResult {
                kind: "app",
                name: app.name.clone(),
                version: app.version.clone(),
                status: ItemStatus::Skipped,
                detail: "not selected in manifest".into(),
            });
            continue;
        }

        let result = import_app(app, interactive)?;
        summary.app_results.push(result);
    }

    print_summary(&summary);

    let failed = summary
        .runtime_results
        .iter()
        .chain(summary.app_results.iter())
        .any(|r| r.status == ItemStatus::Failed);

    if failed {
        anyhow::bail!("Import finished with failures");
    }

    Ok(())
}

fn import_runtime(rt: &AppRuntimeEntry, interactive: bool) -> anyhow::Result<ItemResult> {
    let Some(archive_path) = &rt.archive_path else {
        return Ok(ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Skipped,
            detail: "no local archive in manifest".into(),
        });
    };

    let archive = Path::new(archive_path);
    if !archive.exists() {
        return Ok(ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Failed,
            detail: format!("archive not found: {}", archive.display()),
        });
    }

    if interactive {
        println!(
            "  {} Install runtime {} {} from {}? [Y/n] ",
            "❓".bold(),
            rt.tool,
            rt.version,
            archive.display()
        );
        if !read_yes_no(true) {
            return Ok(ItemResult {
                kind: "runtime",
                name: rt.tool.clone(),
                version: rt.version.clone(),
                status: ItemStatus::Skipped,
                detail: "skipped by user".into(),
            });
        }
    }

    run_with_failure_policy(interactive, || {
        println!(
            "  {} Installing {} {} from local archive...",
            "🔧".bold(),
            rt.tool,
            rt.version
        );
        let kind: crate::runtime::RuntimeKind = rt.tool.parse()?;
        let config = crate::config::Config::load()?;
        crate::runtime::install(kind, &rt.version, Some(archive), &config)?;
        Ok(())
    })
    .map(|status| ItemResult {
        kind: "runtime",
        name: rt.tool.clone(),
        version: rt.version.clone(),
        status,
        detail: status_detail("installed from local archive", "user skipped after failure", archive),
    })
    .or_else(|e| {
        Ok(ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Failed,
            detail: e.to_string(),
        })
    })
}

fn import_app(app: &AppEntry, interactive: bool) -> anyhow::Result<ItemResult> {
    match app.source {
        AppSource::Winget => import_winget_app(app, interactive),
        AppSource::Portable | AppSource::Manual => import_portable_or_manual_app(app, interactive),
        AppSource::Registry => import_registry_app(app, interactive),
    }
}

fn import_winget_app(app: &AppEntry, interactive: bool) -> anyhow::Result<ItemResult> {
    if let Some(path) = &app.install_path {
        let installer = Path::new(path);
        if installer.exists() {
            if interactive {
                println!(
                    "  {} Install {} {} from local installer {}? [Y/n] ",
                    "❓".bold(),
                    app.name,
                    app.version,
                    installer.display()
                );
                if !read_yes_no(true) {
                    return Ok(skipped_app(app, "skipped by user"));
                }
            }

            let status = run_with_failure_policy(interactive, || {
                install_silent(installer, app.silent_args.as_deref())
            });

            return Ok(match status {
                Ok(ItemStatus::Success) => success_app(app, format!("installed from {}", installer.display())),
                Ok(ItemStatus::Skipped) => skipped_app(app, "user skipped after failure"),
                Ok(ItemStatus::Failed) => failed_app(app, "installer failed"),
                Err(err) => {
                    println!("  {} {} local install failed, trying winget fallback...", "ℹ".yellow(), app.name);
                    import_winget_fallback(app, interactive, Some(err.to_string()))?
                }
            });
        }
    }

    import_winget_fallback(app, interactive, None)
}

fn import_registry_app(app: &AppEntry, interactive: bool) -> anyhow::Result<ItemResult> {
    import_winget_fallback(app, interactive, None)
}

fn import_portable_or_manual_app(app: &AppEntry, interactive: bool) -> anyhow::Result<ItemResult> {
    let Some(path) = &app.install_path else {
        return Ok(failed_app(app, "missing install_path"));
    };

    let src = Path::new(path);
    if !src.exists() {
        return Ok(failed_app(app, format!("source path not found: {}", src.display())));
    }

    let target_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
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
            return Ok(skipped_app(app, "skipped by user"));
        }
    }

    match run_with_failure_policy(interactive, || {
        std::fs::create_dir_all(&target_dir)?;
        copy_dir_recursive(src, &target_dir)?;
        Ok(())
    }) {
        Ok(ItemStatus::Success) => Ok(success_app(app, format!("copied to {}", target_dir.display()))),
        Ok(ItemStatus::Skipped) => Ok(skipped_app(app, "user skipped after failure")),
        Ok(ItemStatus::Failed) => Ok(failed_app(app, "copy failed")),
        Err(err) => Ok(failed_app(app, err.to_string())),
    }
}

fn import_winget_fallback(
    app: &AppEntry,
    interactive: bool,
    previous_error: Option<String>,
) -> anyhow::Result<ItemResult> {
    if interactive {
        if let Some(err) = &previous_error {
            println!("  {} Previous error: {}", "⚠".yellow().bold(), err);
        }
        println!("  {} Install {} via winget? [Y/n] ", "❓".bold(), app.name);
        if !read_yes_no(true) {
            return Ok(skipped_app(app, "skipped winget fallback"));
        }
    }

    match run_with_failure_policy(interactive, || install_via_winget(app)) {
        Ok(ItemStatus::Success) => Ok(success_app(app, "installed via winget")),
        Ok(ItemStatus::Skipped) => Ok(skipped_app(app, "user skipped after failure")),
        Ok(ItemStatus::Failed) => Ok(failed_app(app, "winget install failed")),
        Err(err) => Ok(failed_app(app, err.to_string())),
    }
}

fn run_with_failure_policy<F>(interactive: bool, mut action: F) -> anyhow::Result<ItemStatus>
where
    F: FnMut() -> anyhow::Result<()>,
{
    loop {
        match action() {
            Ok(()) => return Ok(ItemStatus::Success),
            Err(err) if !interactive => {
                println!("  {} {}", "✗".red(), err);
                return Err(err);
            }
            Err(err) => {
                println!("  {} {}", "✗".red(), err);
                match ask_failure_action()? {
                    FailureAction::Retry => continue,
                    FailureAction::Skip => return Ok(ItemStatus::Skipped),
                    FailureAction::Abort => anyhow::bail!(err),
                }
            }
        }
    }
}

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

fn install_via_winget(app: &AppEntry) -> anyhow::Result<()> {
    let status = std::process::Command::new("winget")
        .args([
            "install",
            "--id",
            &app.id,
            "--silent",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ])
        .status()?;

    if status.success() {
        println!("  {} {} installed via winget", "✓".green(), app.name);
        Ok(())
    } else {
        anyhow::bail!("{} failed to install via winget: {:?}", app.name, status.code())
    }
}

fn ask_failure_action() -> anyhow::Result<FailureAction> {
    let choice = Select::new(
        "Install failed. Choose next action:",
        vec!["Retry", "Skip", "Abort"],
    )
    .prompt()?;

    Ok(match choice {
        "Retry" => FailureAction::Retry,
        "Skip" => FailureAction::Skip,
        _ => FailureAction::Abort,
    })
}

fn print_summary(summary: &ImportSummary) {
    println!();
    println!("{} Import summary", "📋".bold());

    for result in summary
        .runtime_results
        .iter()
        .chain(summary.app_results.iter())
    {
        let status = match result.status {
            ItemStatus::Success => "✓".green(),
            ItemStatus::Skipped => "→".yellow(),
            ItemStatus::Failed => "✗".red(),
        };
        println!(
            "  {} [{}] {} {} — {}",
            status,
            result.kind,
            result.name,
            result.version,
            result.detail
        );
    }

    let total = summary.runtime_results.len() + summary.app_results.len();
    let success = count_status(summary, ItemStatus::Success);
    let skipped = count_status(summary, ItemStatus::Skipped);
    let failed = count_status(summary, ItemStatus::Failed);

    println!();
    println!(
        "{} Total: {}  Success: {}  Skipped: {}  Failed: {}",
        if failed == 0 { "✅".green().bold() } else { "⚠".yellow().bold() },
        total,
        success.to_string().green().bold(),
        skipped.to_string().yellow().bold(),
        failed.to_string().red().bold()
    );
}

fn count_status(summary: &ImportSummary, status: ItemStatus) -> usize {
    summary
        .runtime_results
        .iter()
        .chain(summary.app_results.iter())
        .filter(|r| r.status == status)
        .count()
}

fn success_app(app: &AppEntry, detail: impl Into<String>) -> ItemResult {
    ItemResult {
        kind: "app",
        name: app.name.clone(),
        version: app.version.clone(),
        status: ItemStatus::Success,
        detail: detail.into(),
    }
}

fn skipped_app(app: &AppEntry, detail: impl Into<String>) -> ItemResult {
    ItemResult {
        kind: "app",
        name: app.name.clone(),
        version: app.version.clone(),
        status: ItemStatus::Skipped,
        detail: detail.into(),
    }
}

fn failed_app(app: &AppEntry, detail: impl Into<String>) -> ItemResult {
    ItemResult {
        kind: "app",
        name: app.name.clone(),
        version: app.version.clone(),
        status: ItemStatus::Failed,
        detail: detail.into(),
    }
}

fn status_detail(success: &str, skipped: &str, path: &Path) -> String {
    let _ = skipped;
    format!("{} ({})", success, path.display())
}

fn read_yes_no(default: bool) -> bool {
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    match input.trim().to_lowercase().as_str() {
        "y" | "yes" => true,
        "n" | "no" => false,
        _ => default,
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
