use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use clap::Subcommand;
use colored::Colorize;
use inquire::{Confirm, MultiSelect};
use serde::{Deserialize, Serialize};

use crate::app::{self, AppEntry};
use crate::config::Config;

#[derive(Subcommand, Debug)]
pub enum AppCommands {
    /// Scan installed applications
    Scan {
        /// Open interactive multi-select after scanning
        #[arg(long)]
        interactive: bool,

        /// Filter apps by keyword (name/id contains)
        #[arg(long)]
        filter: Option<String>,

        /// Filter apps by category: Browser | IDE | Runtime | Dev Tool | Utility | Other
        #[arg(long)]
        category: Option<String>,
    },

    /// Open full-screen TUI selector
    Tui {
        /// Filter apps by keyword before opening TUI
        #[arg(long)]
        filter: Option<String>,

        /// Filter apps by category before opening TUI
        #[arg(long)]
        category: Option<String>,
    },

    /// Add a portable app directory
    AddPath {
        /// Path to the portable app directory
        dir: PathBuf,

        /// Name for the app (optional, defaults to directory name)
        #[arg(long)]
        name: Option<String>,
    },

    /// Export app toolbox
    Export {
        /// Output directory (defaults to ~/.windevkit/export/)
        #[arg(long)]
        output: Option<PathBuf>,

        /// Filter apps by keyword before export
        #[arg(long)]
        filter: Option<String>,

        /// Filter apps by category before export
        #[arg(long)]
        category: Option<String>,

        /// Non-interactive mode (export without prompts)
        #[arg(long)]
        yes: bool,
    },

    /// Import and restore toolbox on a new machine
    Import {
        /// Path to the exported toolbox directory
        path: PathBuf,

        /// Non-interactive mode (install without prompts)
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct LastScanSelection {
    ids: Vec<String>,
}

pub fn execute(cmd: &AppCommands) -> anyhow::Result<()> {
    match cmd {
        AppCommands::Scan { interactive, filter, category } => cmd_scan(*interactive, filter.as_deref(), category.as_deref()),
        AppCommands::Tui { filter, category } => cmd_tui(filter.as_deref(), category.as_deref()),
        AppCommands::AddPath { dir, name } => cmd_add_path(dir, name.as_deref()),
        AppCommands::Export { output, filter, category, yes } => cmd_export(output.as_deref(), filter.as_deref(), category.as_deref(), *yes),
        AppCommands::Import { path, yes } => cmd_import(path, *yes),
    }
}

fn cmd_scan(interactive: bool, filter: Option<&str>, category: Option<&str>) -> anyhow::Result<()> {
    println!("{} Scanning installed applications...", "🔍".bold());

    let config = Config::load()?;
    let mut apps = app::scan(&config.app_scan.exclude_patterns)?;

    if let Some(f) = filter {
        let f = f.to_lowercase();
        apps.retain(|a| a.name.to_lowercase().contains(&f) || a.id.to_lowercase().contains(&f));
    }
    if let Some(c) = category {
        apps.retain(|a| app::category_for_app(a).eq_ignore_ascii_case(c));
    }

    if apps.is_empty() {
        println!("  {} No applications found.", "ℹ".yellow());
        return Ok(());
    }

    // Apply saved selection if present
    if let Some(saved) = load_last_selection()? {
        let selected: HashSet<String> = saved.ids.into_iter().collect();
        for app in &mut apps {
            app.selected = selected.contains(&app.id);
        }
    }

    println!("  Found {} applications", apps.len().to_string().green().bold());
    print_grouped_apps(&apps);

    if interactive {
        println!();
        let selected = select_apps_interactive(apps.clone())?;
        save_last_selection(&selected)?;
        println!(
            "  {} Saved selection: {} apps",
            "💾".bold(),
            selected.len().to_string().green().bold()
        );
        println!(
            "  Use {} to export using this selection.",
            "windevkit app export".bold().cyan()
        );
    } else {
        println!();
        println!(
            "  {} Tip: run {} to pick apps now, then export later.",
            "💡".yellow(),
            "windevkit app scan --interactive".bold().cyan()
        );
        println!(
            "  {} You can also use {} and {}.",
            "🔎".bold(),
            "--filter <keyword>".bold().cyan(),
            "--category <Browser|IDE|Runtime|Dev Tool|Utility|Other>".bold().cyan()
        );
        println!(
            "  {} Or run {} / {}.",
            "📦".bold(),
            "windevkit app export".bold().cyan(),
            "windevkit app tui".bold().cyan()
        );
    }

    Ok(())
}

fn cmd_tui(filter: Option<&str>, category: Option<&str>) -> anyhow::Result<()> {
    println!("{} Loading app TUI...", "🖥️".bold());
    let config = Config::load()?;
    let mut apps = app::scan(&config.app_scan.exclude_patterns)?;

    if let Some(f) = filter {
        let f = f.to_lowercase();
        apps.retain(|a| a.name.to_lowercase().contains(&f) || a.id.to_lowercase().contains(&f));
    }
    if let Some(c) = category {
        apps.retain(|a| app::category_for_app(a).eq_ignore_ascii_case(c));
    }

    if let Some(saved) = load_last_selection()? {
        let selected: HashSet<String> = saved.ids.into_iter().collect();
        for app in &mut apps {
            app.selected = selected.contains(&app.id);
        }
    }

    let selected = app::tui::run(apps)?;
    if selected.is_empty() {
        println!("  {} No selection saved.", "ℹ".yellow());
        return Ok(());
    }
    save_last_selection(&selected)?;
    println!("  {} Saved selection: {} apps", "💾".bold(), selected.len().to_string().green().bold());
    Ok(())
}

fn cmd_add_path(dir: &PathBuf, name: Option<&str>) -> anyhow::Result<()> {
    if !dir.exists() {
        anyhow::bail!("Directory not found: {}", dir.display());
    }

    let app_name = name.unwrap_or_else(|| {
        dir.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
    });

    let entry = AppEntry {
        id: app_name.to_string().replace(' ', "."),
        name: app_name.to_string(),
        version: "portable".into(),
        source: app::AppSource::Portable,
        selected: true,
        install_path: Some(dir.to_string_lossy().to_string()),
        silent_args: None,
    };

    let config_dir = Config::home_dir();
    let portable_file = config_dir.join("portable-apps.toml");
    let mut existing: Vec<AppEntry> = if portable_file.exists() {
        let content = std::fs::read_to_string(&portable_file)?;
        toml::from_str(&content).unwrap_or_default()
    } else {
        Vec::new()
    };

    existing.push(entry);
    let content = toml::to_string_pretty(&existing)?;
    std::fs::write(&portable_file, content)?;

    println!(
        "  {} Added portable app: {} → {}",
        "✓".green(),
        app_name.green().bold(),
        dir.display()
    );

    Ok(())
}

fn cmd_export(output: Option<&Path>, filter: Option<&str>, category: Option<&str>, yes: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let home = Config::home_dir();
    let export_dir = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| home.join("export"));

    let mut apps = app::scan(&config.app_scan.exclude_patterns)?;

    if let Some(f) = filter {
        let f = f.to_lowercase();
        apps.retain(|a| a.name.to_lowercase().contains(&f) || a.id.to_lowercase().contains(&f));
    }
    if let Some(c) = category {
        apps.retain(|a| app::category_for_app(a).eq_ignore_ascii_case(c));
    }

    if apps.is_empty() {
        println!("  {} No applications to export.", "ℹ".yellow());
        return Ok(());
    }

    if let Some(saved) = load_last_selection()? {
        let selected: HashSet<String> = saved.ids.into_iter().collect();
        for app in &mut apps {
            app.selected = selected.contains(&app.id);
        }
    }

    let selected_apps = if yes {
        apps.into_iter().filter(|a| a.selected).collect()
    } else {
        println!();
        println!("{} One-step export flow: scan → TUI select → export", "🚀".bold());
        let selected = app::tui::run(apps)?;
        save_last_selection(&selected)?;
        println!(
            "  {} Saved selection: {} apps",
            "💾".bold(),
            selected.len().to_string().green().bold()
        );
        selected
    };

    if selected_apps.is_empty() {
        println!("  {} No apps selected. Nothing to export.", "ℹ".yellow());
        return Ok(());
    }

    println!(
        "  {} Exporting {} apps...",
        "📦".bold(),
        selected_apps.len().to_string().green().bold()
    );

    let include_runtimes = if yes {
        true
    } else {
        Confirm::new("Include installed runtimes (Node/Java/Maven) for offline reinstall?")
            .with_default(true)
            .prompt()?
    };

    app::export_to(&export_dir, &selected_apps, include_runtimes)?;
    Ok(())
}

fn cmd_import(path: &Path, yes: bool) -> anyhow::Result<()> {
    if !path.exists() {
        anyhow::bail!("Toolbox directory not found: {}", path.display());
    }

    if !yes {
        println!(
            "{} This will install applications from {}",
            "⚠".yellow().bold(),
            path.display()
        );
        if !Confirm::new("Continue?")
            .with_default(true)
            .prompt()?
        {
            println!("Import cancelled.");
            return Ok(());
        }
    }

    app::import_from(path, !yes)?;
    Ok(())
}

fn print_grouped_apps(apps: &[AppEntry]) {
    let mut groups: BTreeMap<&'static str, Vec<&AppEntry>> = BTreeMap::new();
    for app in apps {
        let category = app::category_for_app(app);
        groups.entry(Box::leak(category.into_boxed_str())).or_default().push(app);
    }

    for (category, group) in groups {
        println!();
        println!("  {} {} ({})", category_icon(category), category.bold(), group.len());
        for app in group {
            let source_tag = match app.source {
                app::AppSource::Winget => "winget".cyan(),
                app::AppSource::Registry => "reg".dimmed(),
                app::AppSource::Portable => "portable".yellow(),
                app::AppSource::Manual => "manual".blue(),
            };
            println!(
                "    {} {:35} v{} [{}]",
                if app.selected { "☑".green() } else { "☐".dimmed() },
                truncate_name(&app.name, 35),
                app.version,
                source_tag
            );
        }
    }

    println!();
    println!(
        "  {} Total: {} apps detected.",
        "📊".bold(),
        apps.len().to_string().green().bold()
    );
}

fn select_apps_interactive(apps: Vec<AppEntry>) -> anyhow::Result<Vec<AppEntry>> {
    let options: Vec<String> = apps
        .iter()
        .map(|a| {
            format!(
                "[{}] {} v{} [{}]",
                app::category_for_app(a),
                a.name,
                a.version,
                source_label(&a.source)
            )
        })
        .collect();

    let defaults: Vec<usize> = apps
        .iter()
        .enumerate()
        .filter(|(_, a)| a.selected)
        .map(|(i, _)| i)
        .collect();

    let selections = MultiSelect::new("Select apps to export:", options)
        .with_default(&defaults)
        .with_page_size(20)
        .prompt()?;

    Ok(apps
        .into_iter()
        .filter(|a| {
            let label = format!(
                "[{}] {} v{} [{}]",
                app::category_for_app(a),
                a.name,
                a.version,
                source_label(&a.source)
            );
            selections.contains(&label)
        })
        .collect())
}

fn category_icon(category: &str) -> &'static str {
    match category {
        "Browser" => "🌐",
        "IDE" => "🧠",
        "Runtime" => "⚙️",
        "Dev Tool" => "🛠️",
        "Utility" => "📦",
        _ => "📁",
    }
}

fn truncate_name(name: &str, max: usize) -> String {
    let count = name.chars().count();
    if count <= max {
        return name.to_string();
    }
    let mut s: String = name.chars().take(max.saturating_sub(1)).collect();
    s.push('…');
    s
}

fn source_label(source: &app::AppSource) -> &'static str {
    match source {
        app::AppSource::Winget => "winget",
        app::AppSource::Registry => "registry",
        app::AppSource::Portable => "portable",
        app::AppSource::Manual => "manual",
    }
}

fn last_selection_path() -> PathBuf {
    Config::home_dir().join("last-scan-selection.json")
}

fn save_last_selection(apps: &[AppEntry]) -> anyhow::Result<()> {
    let data = LastScanSelection {
        ids: apps.iter().map(|a| a.id.clone()).collect(),
    };
    let path = last_selection_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(&data)?)?;
    Ok(())
}

fn load_last_selection() -> anyhow::Result<Option<LastScanSelection>> {
    let path = last_selection_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let data = serde_json::from_str::<LastScanSelection>(&content)?;
    Ok(Some(data))
}
