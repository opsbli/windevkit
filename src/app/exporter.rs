//! Exporter — generates the offline toolbox directory.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};

use colored::Colorize;

use crate::app::manifest::Manifest;
use crate::app::{self, AppEntry, AppRuntimeEntry};
use crate::config::Config;
use crate::runtime::{self, RuntimeKind};

#[derive(Debug, Clone)]
struct DownloadTask {
    key: String,
    label: String,
    url: String,
    target_dir: PathBuf,
    filename: String,
}

#[derive(Debug, Clone)]
struct DownloadOutcome {
    key: String,
    label: String,
    result: Result<PathBuf, String>,
}

#[derive(Debug, Clone)]
struct ExportSummary {
    downloaded: usize,
    copied: usize,
    deferred: usize,
    failed: usize,
}

/// Export the toolbox to a directory.
pub fn export_to(
    output_dir: &Path,
    selected_apps: &[AppEntry],
    include_runtimes: bool,
    download_concurrency: usize,
) -> anyhow::Result<()> {
    let installers_dir = output_dir.join("installers");
    let portables_dir = output_dir.join("portables");
    let runtimes_dir = output_dir.join("runtimes");

    std::fs::create_dir_all(&installers_dir)?;
    std::fs::create_dir_all(&portables_dir)?;
    std::fs::create_dir_all(&runtimes_dir)?;

    println!(
        "{} Exporting toolbox to {}",
        "📦".bold(),
        output_dir.display()
    );
    let download_concurrency = download_concurrency.max(1);
    println!(
        "{} Download concurrency: {}",
        "⚡".bold(),
        download_concurrency.to_string().cyan().bold()
    );

    let config = Config::load()?;

    let app_download_plan = build_app_download_plan(selected_apps, &installers_dir);
    let runtime_download_plan = if include_runtimes {
        build_runtime_download_plan(&config, &runtimes_dir)?
    } else {
        RuntimeDownloadPlan {
            tasks: Vec::new(),
            entries: Vec::new(),
        }
    };

    let mut all_tasks = Vec::new();
    all_tasks.extend(app_download_plan.tasks.clone());
    all_tasks.extend(runtime_download_plan.tasks.clone());

    let download_results = run_downloads(all_tasks, download_concurrency)?;
    let mut result_map = HashMap::new();
    for outcome in download_results {
        result_map.insert(outcome.key.clone(), outcome);
    }

    let mut summary = ExportSummary {
        downloaded: 0,
        copied: 0,
        deferred: 0,
        failed: 0,
    };

    let exported_apps = export_apps(
        selected_apps,
        &portables_dir,
        &app_download_plan.tasks,
        &result_map,
        &mut summary,
    )?;

    let exported_runtimes =
        export_runtimes(runtime_download_plan.entries, &result_map, &mut summary);

    let manifest = Manifest::new(exported_apps.clone(), exported_runtimes.clone());
    let manifest_path = output_dir.join("manifest.toml");
    manifest.save(&manifest_path)?;

    let report_path = output_dir.join("apps.md");
    std::fs::write(
        &report_path,
        generate_apps_report(&exported_apps, &exported_runtimes),
    )?;

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
    println!(
        "   Downloads: {}  Copied: {}  Deferred: {}  Failed: {}",
        summary.downloaded.to_string().green().bold(),
        summary.copied.to_string().cyan().bold(),
        summary.deferred.to_string().yellow().bold(),
        summary.failed.to_string().red().bold()
    );

    Ok(())
}

#[derive(Debug, Clone)]
struct AppDownloadPlan {
    tasks: Vec<DownloadTask>,
}

#[derive(Debug, Clone)]
struct RuntimeDownloadPlan {
    tasks: Vec<DownloadTask>,
    entries: Vec<AppRuntimeEntry>,
}

fn build_app_download_plan(selected_apps: &[AppEntry], installers_dir: &Path) -> AppDownloadPlan {
    let mut tasks = Vec::new();

    for app in selected_apps {
        if let Some(rule) = app::rules::resolve_rule(app)
            && let Some(url) = rule.download_url.clone()
        {
            tasks.push(DownloadTask {
                key: app_download_key(app),
                label: format!("app:{}", app.name),
                filename: filename_from_rule_or_url(app, &rule, &url),
                url,
                target_dir: installers_dir.to_path_buf(),
            });
        }
    }

    AppDownloadPlan { tasks }
}

fn build_runtime_download_plan(
    config: &Config,
    runtimes_dir: &Path,
) -> anyhow::Result<RuntimeDownloadPlan> {
    let mut tasks = Vec::new();
    let mut entries = Vec::new();

    for kind in &[RuntimeKind::Node, RuntimeKind::Java, RuntimeKind::Maven] {
        let versions = runtime::list_installed(*kind)?;
        for v in versions {
            let filename = runtime::url::archive_filename(*kind, &v.version);
            let url = runtime::url::build_download_url(*kind, &v.version, &config.core.mirror);
            let dest = runtimes_dir.join(&filename);
            let key = runtime_download_key(*kind, &v.version);

            entries.push(AppRuntimeEntry {
                tool: kind.to_string(),
                version: v.version.clone(),
                archive_path: Some(dest.to_string_lossy().to_string()),
            });

            if dest.exists() {
                println!(
                    "  {} runtime {} {} already cached",
                    "↺".cyan(),
                    kind,
                    v.version
                );
                continue;
            }

            tasks.push(DownloadTask {
                key,
                label: format!("runtime:{} {}", kind, v.version),
                url,
                target_dir: runtimes_dir.to_path_buf(),
                filename,
            });
        }
    }

    Ok(RuntimeDownloadPlan { tasks, entries })
}

fn export_apps(
    selected_apps: &[AppEntry],
    portables_dir: &Path,
    app_tasks: &[DownloadTask],
    result_map: &HashMap<String, DownloadOutcome>,
    summary: &mut ExportSummary,
) -> anyhow::Result<Vec<AppEntry>> {
    let app_task_keys: HashMap<String, &DownloadTask> =
        app_tasks.iter().map(|t| (t.key.clone(), t)).collect();
    let mut exported_apps = Vec::new();

    for app in selected_apps {
        let app_key = app_download_key(app);
        if app_task_keys.contains_key(&app_key)
            && let Some(outcome) = result_map.get(&app_key)
        {
            match &outcome.result {
                Ok(path) => {
                    println!("  {} {} — {} saved", "✓".green(), app.name, path.display());
                    let mut exported = app.clone();
                    exported.install_path = Some(path.to_string_lossy().to_string());
                    exported.silent_args = app::silent_args_for_app(app);
                    exported.installer_type = app::installer_type_for_app(app);
                    exported.portable = Some(app::portable_for_app(app));
                    exported_apps.push(exported);
                    summary.downloaded += 1;
                    continue;
                }
                Err(e) => {
                    println!(
                        "  {} {} — direct rule download failed: {}",
                        "ℹ".yellow(),
                        app.name,
                        e
                    );
                }
            }
        }

        match app.source {
            crate::app::AppSource::Winget => {
                let mut exported = app.clone();
                exported.silent_args = app::silent_args_for_app(app);
                exported.installer_type = app::installer_type_for_app(app);
                exported.portable = Some(app::portable_for_app(app));
                println!(
                    "  {} {} — winget download not yet supported; will install via winget",
                    "ℹ".yellow(),
                    app.name
                );
                exported_apps.push(exported);
                summary.deferred += 1;
            }
            crate::app::AppSource::Portable | crate::app::AppSource::Manual => {
                if let Some(ref src_path) = app.install_path {
                    let src = Path::new(src_path);
                    if src.exists() {
                        let dest = portables_dir.join(&app.id);
                        copy_dir_recursive(src, &dest)?;
                        println!("  {} {} — portable copied", "✓".green(), app.name);
                        let mut exported = app.clone();
                        exported.install_path = Some(dest.to_string_lossy().to_string());
                        exported.installer_type = Some("portable".into());
                        exported.portable = Some(true);
                        exported_apps.push(exported);
                        summary.copied += 1;
                    } else {
                        println!("  {} {} — source not found", "✗".red(), app.name);
                        exported_apps.push(app.clone());
                        summary.failed += 1;
                    }
                } else {
                    exported_apps.push(app.clone());
                    summary.deferred += 1;
                }
            }
            _ => {
                let mut exported = app.clone();
                exported.silent_args = app::silent_args_for_app(app);
                exported.installer_type = app::installer_type_for_app(app);
                exported.portable = Some(app::portable_for_app(app));
                exported_apps.push(exported);
                summary.deferred += 1;
            }
        }
    }

    Ok(exported_apps)
}

fn export_runtimes(
    entries: Vec<AppRuntimeEntry>,
    result_map: &HashMap<String, DownloadOutcome>,
    summary: &mut ExportSummary,
) -> Vec<AppRuntimeEntry> {
    for rt in &entries {
        let key = runtime_download_key_str(&rt.tool, &rt.version);
        if let Some(outcome) = result_map.get(&key) {
            match &outcome.result {
                Ok(path) => {
                    println!(
                        "  {} runtime {} {} — {} saved",
                        "✓".green(),
                        rt.tool,
                        rt.version,
                        path.display()
                    );
                    summary.downloaded += 1;
                }
                Err(e) => {
                    println!("  {} runtime {} {} — {}", "✗".red(), rt.tool, rt.version, e);
                    summary.failed += 1;
                }
            }
        }
    }
    entries
}

fn run_downloads(
    tasks: Vec<DownloadTask>,
    concurrency: usize,
) -> anyhow::Result<Vec<DownloadOutcome>> {
    if tasks.is_empty() {
        return Ok(Vec::new());
    }

    println!(
        "{} Queued {} downloads",
        "📥".bold(),
        tasks.len().to_string().green().bold()
    );

    let queue = Arc::new(Mutex::new(VecDeque::from(tasks)));
    let (tx, rx) = mpsc::channel();
    let workers = concurrency.max(1);

    for _ in 0..workers {
        let queue = Arc::clone(&queue);
        let tx = tx.clone();
        std::thread::spawn(move || {
            loop {
                let task = {
                    let mut guard = queue.lock().expect("download queue poisoned");
                    guard.pop_front()
                };

                let Some(task) = task else {
                    break;
                };
                println!("  {} {}", "↓".cyan(), task.label);
                let result = crate::runtime::download::download(
                    &task.url,
                    &task.target_dir,
                    &task.filename,
                    None,
                )
                .map_err(|e| e.to_string());

                let _ = tx.send(DownloadOutcome {
                    key: task.key,
                    label: task.label,
                    result,
                });
            }
        });
    }
    drop(tx);

    let mut outcomes = Vec::new();
    for outcome in rx {
        outcomes.push(outcome);
    }
    outcomes.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(outcomes)
}

fn app_download_key(app: &AppEntry) -> String {
    format!("app:{}", app.id)
}

fn runtime_download_key(kind: RuntimeKind, version: &str) -> String {
    runtime_download_key_str(&kind.to_string(), version)
}

fn runtime_download_key_str(tool: &str, version: &str) -> String {
    format!("runtime:{}:{}", tool, version)
}

fn filename_from_rule_or_url(
    app: &AppEntry,
    rule: &crate::app::rules::AppRule,
    url: &str,
) -> String {
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
        let name = path
            .strip_prefix(base)?
            .to_string_lossy()
            .replace('\\', "/");

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
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            app.name.replace('|', "\\|"),
            app.version.replace('|', "\\|"),
            source,
            category.replace('|', "\\|"),
            path.replace('|', "\\|")
        ));
    }

    out.push_str("\n## Runtimes\n\n");
    out.push_str("| Tool | Version | Archive |\n");
    out.push_str("|---|---:|---|\n");
    for rt in runtimes {
        let path = rt.archive_path.as_deref().unwrap_or("");
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            rt.tool.replace('|', "\\|"),
            rt.version.replace('|', "\\|"),
            path.replace('|', "\\|")
        ));
    }
    out
}
