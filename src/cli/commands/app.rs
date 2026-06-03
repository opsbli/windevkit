use std::path::{Path, PathBuf};

use clap::Subcommand;
use colored::Colorize;
use inquire::{Confirm, MultiSelect};

use crate::app::{self, AppEntry};
use crate::config::Config;

#[derive(Subcommand, Debug)]
pub enum AppCommands {
    /// Scan installed applications
    Scan,

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

pub fn execute(cmd: &AppCommands) -> anyhow::Result<()> {
    match cmd {
        AppCommands::Scan => cmd_scan(),
        AppCommands::AddPath { dir, name } => cmd_add_path(dir, name.as_deref()),
        AppCommands::Export { output, yes } => cmd_export(output.as_deref(), *yes),
        AppCommands::Import { path, yes } => cmd_import(path, *yes),
    }
}

/// Scan system and interactively select apps.
fn cmd_scan() -> anyhow::Result<()> {
    println!("{} Scanning installed applications...", "🔍".bold());

    let config = Config::load()?;
    let apps = app::scan(&config.app_scan.exclude_patterns)?;

    if apps.is_empty() {
        println!("  {} No applications found.", "ℹ".yellow());
        return Ok(());
    }

    println!("  Found {} applications", apps.len());

    // Display in a readable format
    for app in &apps {
        let source_tag = match app.source {
            app::AppSource::Winget => "winget".cyan(),
            app::AppSource::Registry => "reg".dimmed(),
            app::AppSource::Portable => "portable".yellow(),
            app::AppSource::Manual => "manual".blue(),
        };
        println!(
            "  {} {:35} v{} [{}]",
            if app.selected { "☑".green() } else { "☐".dimmed() },
            app.name,
            app.version,
            source_tag
        );
    }

    println!();
    println!(
        "  {} Total: {} apps detected. Use `windevkit app export` to create a toolbox.",
        "📊".bold(),
        apps.len()
    );

    Ok(())
}

/// Add a portable app directory to the config.
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

    // Save to a local portable apps list in config
    // For now, store in a simple sidecar file
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

/// Export toolbox.
fn cmd_export(output: Option<&Path>, yes: bool) -> anyhow::Result<()> {
    let config = Config::load()?;
    let home = Config::home_dir();
    let export_dir = output
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| home.join("export"));

    // Scan apps
    let apps = app::scan(&config.app_scan.exclude_patterns)?;

    if apps.is_empty() {
        println!("  {} No applications to export.", "ℹ".yellow());
        return Ok(());
    }

    // Interactive selection
    let selected_apps = if yes {
        apps
    } else {
        select_apps_interactive(apps)?
    };

    if selected_apps.is_empty() {
        println!("  {} No apps selected. Nothing to export.", "ℹ".yellow());
        return Ok(());
    }

    println!(
        "  {} Exporting {} apps...",
        "📦".bold(),
        selected_apps.len()
    );

    // Ask about including runtimes
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

/// Import and restore toolbox.
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

/// Interactive multi-selection of apps.
fn select_apps_interactive(apps: Vec<AppEntry>) -> anyhow::Result<Vec<AppEntry>> {
    let options: Vec<String> = apps
        .iter()
        .map(|a| format!("{} v{} [{}]", a.name, a.version, source_label(&a.source)))
        .collect();

    let defaults: Vec<usize> = apps
        .iter()
        .enumerate()
        .filter(|(_, a)| a.selected)
        .map(|(i, _)| i)
        .collect();

    let selections = MultiSelect::new("Select apps to export:", options)
        .with_default(&defaults)
        .with_page_size(15)
        .prompt()?;

    // Map selected labels back to app entries
    Ok(apps
        .into_iter()
        .filter(|a| {
            let label = format!("{} v{} [{}]", a.name, a.version, source_label(&a.source));
            selections.contains(&label)
        })
        .collect())
}

fn source_label(source: &app::AppSource) -> &'static str {
    match source {
        app::AppSource::Winget => "winget",
        app::AppSource::Registry => "registry",
        app::AppSource::Portable => "portable",
        app::AppSource::Manual => "manual",
    }
}
