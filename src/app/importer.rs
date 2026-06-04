//! Importer — restores apps and runtimes from an exported toolbox.

use std::path::{Path, PathBuf};

use colored::Colorize;
use inquire::Select;

use crate::app::manifest::Manifest;
use crate::app::{self, AppEntry, AppRuntimeEntry};

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

pub fn import_from(toolbox_dir: &Path, interactive: bool) -> anyhow::Result<()> {
    let manifest_path = toolbox_dir.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!(
            "No manifest.toml found in {}. Is this a valid toolbox directory?",
            toolbox_dir.display()
        );
    }

    let manifest = Manifest::load(&manifest_path)?;
    println!(
        "{} Importing toolbox from {}",
        "📥".bold(),
        toolbox_dir.display()
    );
    println!(
        "   Found {} apps, {} runtimes",
        manifest.apps.len(),
        manifest.runtimes.len()
    );

    let mut summary = ImportSummary {
        runtime_results: Vec::new(),
        app_results: Vec::new(),
    };

    for rt in &manifest.runtimes {
        summary
            .runtime_results
            .push(import_runtime(rt, interactive)?);
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
        summary.app_results.push(import_app(app, interactive)?);
    }

    print_summary(&summary);

    if summary
        .runtime_results
        .iter()
        .chain(summary.app_results.iter())
        .any(|r| r.status == ItemStatus::Failed)
    {
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

    let result = run_with_failure_policy(interactive, || {
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
    });

    Ok(match result {
        Ok(ItemStatus::Success) => ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Success,
            detail: format!("installed from {}", archive.display()),
        },
        Ok(ItemStatus::Skipped) => ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Skipped,
            detail: "user skipped after failure".into(),
        },
        Ok(ItemStatus::Failed) => ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Failed,
            detail: "runtime install failed".into(),
        },
        Err(e) => ItemResult {
            kind: "runtime",
            name: rt.tool.clone(),
            version: rt.version.clone(),
            status: ItemStatus::Failed,
            detail: e.to_string(),
        },
    })
}

fn import_app(app: &AppEntry, interactive: bool) -> anyhow::Result<ItemResult> {
    if let Some(path) = &app.install_path {
        let local = Path::new(path);
        if local.exists() {
            if interactive {
                println!(
                    "  {} Install {} {} from local artifact {}? [Y/n] ",
                    "❓".bold(),
                    app.name,
                    app.version,
                    local.display()
                );
                if !read_yes_no(true) {
                    return Ok(skipped_app(app, "skipped by user"));
                }
            }

            let local_result =
                run_with_failure_policy(interactive, || install_local_artifact(app, local));
            return Ok(match local_result {
                Ok(ItemStatus::Success) => {
                    success_app(app, format!("installed from {}", local.display()))
                }
                Ok(ItemStatus::Skipped) => skipped_app(app, "user skipped after failure"),
                Ok(ItemStatus::Failed) => failed_app(app, "local artifact install failed"),
                Err(err) => {
                    println!(
                        "  {} {} local install failed, trying winget fallback...",
                        "ℹ".yellow(),
                        app.name
                    );
                    import_winget_fallback(app, interactive, Some(err.to_string()))?
                }
            });
        }
    }

    import_winget_fallback(app, interactive, None)
}

fn install_local_artifact(app: &AppEntry, path: &Path) -> anyhow::Result<()> {
    let installer_type = app::installer_type_for_app(app)
        .or_else(|| infer_installer_type(path))
        .unwrap_or_else(|| "exe".into())
        .to_lowercase();

    if app::portable_for_app(app) || installer_type == "portable" {
        install_portable_dir(app, path)
    } else {
        match installer_type.as_str() {
            "exe" => install_exe(path, app.silent_args.as_deref()),
            "msi" => install_msi(path, app.silent_args.as_deref()),
            "zip" => extract_zip_to_tools(app, path),
            "portable" => install_portable_dir(app, path),
            other => anyhow::bail!("Unsupported installer_type: {}", other),
        }
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

fn install_exe(installer: &Path, silent_args: Option<&str>) -> anyhow::Result<()> {
    let args = silent_args.unwrap_or("/S");
    let status = std::process::Command::new(installer)
        .args(args.split_whitespace())
        .status()?;
    if !status.success() {
        anyhow::bail!("Installer exited with code {:?}", status.code());
    }
    Ok(())
}

fn install_msi(installer: &Path, silent_args: Option<&str>) -> anyhow::Result<()> {
    let mut args = vec!["/i".to_string(), installer.to_string_lossy().to_string()];
    if let Some(extra) = silent_args {
        args.extend(extra.split_whitespace().map(|s| s.to_string()));
    } else {
        args.extend(["/quiet".into(), "/norestart".into()]);
    }
    let status = std::process::Command::new("msiexec").args(&args).status()?;
    if !status.success() {
        anyhow::bail!("MSI installer exited with code {:?}", status.code());
    }
    Ok(())
}

fn extract_zip_to_tools(app: &AppEntry, archive: &Path) -> anyhow::Result<()> {
    let target_dir = tool_target_dir(app);
    std::fs::create_dir_all(&target_dir)?;
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let outpath = match entry.enclosed_name() {
            Some(path) => target_dir.join(path),
            None => continue,
        };
        if entry.name().ends_with('/') {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    println!(
        "  {} {} extracted to {}",
        "✓".green(),
        app.name,
        target_dir.display()
    );
    Ok(())
}

fn install_portable_dir(app: &AppEntry, src: &Path) -> anyhow::Result<()> {
    let target_dir = tool_target_dir(app);
    std::fs::create_dir_all(&target_dir)?;
    if src.is_dir() {
        copy_dir_recursive(src, &target_dir)?;
    } else {
        anyhow::bail!("portable source is not a directory: {}", src.display());
    }
    println!(
        "  {} {} installed to {}",
        "✓".green(),
        app.name,
        target_dir.display()
    );
    Ok(())
}

fn tool_target_dir(app: &AppEntry) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("tools")
        .join(&app.id)
}

fn infer_installer_type(path: &Path) -> Option<String> {
    if path.is_dir() {
        return Some("portable".into());
    }
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
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
        anyhow::bail!(
            "{} failed to install via winget: {:?}",
            app.name,
            status.code()
        )
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
            status, result.kind, result.name, result.version, result.detail
        );
    }

    let total = summary.runtime_results.len() + summary.app_results.len();
    let success = count_status(summary, ItemStatus::Success);
    let skipped = count_status(summary, ItemStatus::Skipped);
    let failed = count_status(summary, ItemStatus::Failed);

    println!();
    println!(
        "{} Total: {}  Success: {}  Skipped: {}  Failed: {}",
        if failed == 0 {
            "✅".green().bold()
        } else {
            "⚠".yellow().bold()
        },
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
