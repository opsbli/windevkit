use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

use crate::config::Config;

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Force re-initialization even if already configured
    #[arg(long)]
    pub force: bool,
}

/// Execute init, accepting an optional mirror override.
pub fn execute(args: &InitArgs) -> anyhow::Result<()> {
    execute_with(args, None)
}

/// Execute init with explicit mirror override from global --mirror flag.
pub fn execute_with(args: &InitArgs, mirror: Option<&str>) -> anyhow::Result<()> {
    let home = Config::home_dir();

    // Check if already initialized
    if home.join("config.toml").exists() && !args.force {
        println!(
            "{}",
            "⚠️  windevkit is already initialized. Use --force to re-initialize.".yellow()
        );
        println!("   Home: {}", home.display());
        return Ok(());
    }

    println!("{}", "🏗️  Initializing windevkit...".bold());

    // Step 1: Detect Windows Developer Mode
    let dev_mode = detect_developer_mode();
    let dev_mode_status = if dev_mode {
        "✓ Enabled".green().bold()
    } else {
        "✗ Disabled".red().bold()
    };
    println!("  {}  Windows Developer Mode... {}", "🔍".bold(), dev_mode_status);

    // Step 2: Create directory structure
    let dirs = create_directories(&home)?;
    println!("  {}  Creating directory structure... {}",
        "📂".bold(),
        format!("{} directories created", dirs.len()).green().bold()
    );

    // Step 3: Write default config
    let mut config = Config::default();
    config.core.dev_mode = dev_mode;
    if let Some(m) = mirror {
        if !m.is_empty() {
            config.core.mirror = m.to_string();
        }
    }
    config.save()?;
    println!("  {}  Writing config.toml... {}", "📝".bold(), "✓".green().bold());

    // Step 4: Summary
    println!();
    println!("{}", "✅ windevkit has been initialized!".green().bold());
    println!();
    println!("  Home directory:  {}", home.display());
    println!("  Config file:     {}", Config::config_path().display());
    println!("  Developer Mode:  {}", if dev_mode { "Enabled" } else { "Disabled" });

    // Guide user if Developer Mode is not enabled
    if !dev_mode {
        println!();
        println!("{}", "💡 Recommended: Enable Windows Developer Mode for symlink support.".yellow());
        println!("   Run the following in PowerShell as Administrator:");
        println!();
        println!("   {}", "reg add HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock".cyan());
        println!("   {}",
            "/t REG_DWORD /v AllowDevelopmentWithoutDevLicense /d 1 /f".cyan());
        println!();
        println!("   Or go to: Settings → Privacy & security → For developers → Developer Mode");
        println!("   Then run {} again.", "windevkit doctor --fix".bold());
    }

    // Remind about PATH
    let active_bin = home.join("active").join("bin");
    println!();
    println!("{}", "💡 After installing runtimes, add to your PATH:".yellow());
    println!("   {}", active_bin.display().to_string().cyan());
    println!("   (windevkit will handle this automatically during install)");

    Ok(())
}

/// Create the full windevkit directory structure.
/// Returns a list of created directory paths.
fn create_directories(home: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let dirs = vec![
        home.clone(),
        home.join("versions"),
        home.join("versions").join("node"),
        home.join("versions").join("java"),
        home.join("versions").join("maven"),
        home.join("active"),
        home.join("export"),
        home.join("export").join("installers"),
        home.join("export").join("portables"),
        home.join("export").join("runtimes"),
        home.join("backups"),
        home.join("logs"),
        home.join("cache"),
        home.join("cache").join("downloads"),
    ];

    let mut created = Vec::new();
    for dir in &dirs {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
            created.push(dir.clone());
        }
    }

    Ok(created)
}

/// Detect whether Windows Developer Mode is enabled.
fn detect_developer_mode() -> bool {
    // Use PowerShell to check the registry key (reliable on Windows 10 1803+)
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-ItemPropertyValue -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock' -Name AllowDevelopmentWithoutDevLicense -ErrorAction SilentlyContinue",
        ])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            stdout == "1"
        }
        Err(_) => {
            // Fallback: try reg.exe
            let fallback = std::process::Command::new("reg")
                .args([
                    "query",
                    "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock",
                    "/v",
                    "AllowDevelopmentWithoutDevLicense",
                ])
                .output();

            match fallback {
                Ok(fb) => {
                    let stdout = String::from_utf8_lossy(&fb.stdout);
                    stdout.contains("0x1")
                }
                Err(_) => false,
            }
        }
    }
}
