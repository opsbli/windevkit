use colored::Colorize;

use crate::config::Config;
use crate::runtime::{self, RuntimeKind, path, symlink};

pub fn execute(fix: bool) -> anyhow::Result<()> {
    let home = Config::home_dir();
    let mut all_ok = true;

    println!("{}", "🔧 windevkit diagnostics".bold());
    println!("{}", "═".repeat(50).dimmed());

    // Check 1: Home directory exists
    print_check("Home directory", home.exists());
    if !home.exists() {
        println!("     {} Run `windevkit init` first.", "→".yellow());
        all_ok = false;
    }

    // Check 2: Config file
    let config_path = Config::config_path();
    print_check("Config file", config_path.exists());

    // Check 3: Active symlinks
    for kind in &[RuntimeKind::Node, RuntimeKind::Java, RuntimeKind::Maven] {
        let link = symlink::active_link_path(&home, *kind);
        let is_valid = link.is_symlink() && link.exists();

        let versions = runtime::list_installed(*kind).unwrap_or_default();
        let has_versions = !versions.is_empty();

        if has_versions {
            if is_valid {
                let target = symlink::read_link(&link)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                println!(
                    "  {} {} symlink ✓ → {}",
                    kind_icon(*kind),
                    kind.to_string().bold(),
                    target.dimmed()
                );
            } else if fix {
                // Try to auto-fix: activate the latest version
                if let Some(latest) = versions.first() {
                    println!(
                        "     {} Fixing {} symlink → v{}",
                        "🔧".bold(),
                        kind,
                        latest.version
                    );
                    if symlink::set_active(
                        &link,
                        &symlink::version_dir(&home, *kind, &latest.version),
                    )
                    .is_ok()
                    {
                        let target = symlink::read_link(&link)
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        println!(
                            "  {} {} symlink ✓ → {}",
                            kind_icon(*kind),
                            kind.to_string().bold(),
                            target.dimmed()
                        );
                    } else {
                        all_ok = false;
                    }
                } else {
                    all_ok = false;
                }
            } else {
                println!(
                    "  {} {} {} (use --fix to repair)",
                    kind_icon(*kind),
                    kind.to_string().bold(),
                    "⚠ symlink broken".yellow()
                );
                all_ok = false;
            }
        }
    }

    // Check 4: PATH
    let in_path = path::is_in_path(&home);
    print_check("PATH configured", in_path);
    if !in_path {
        if fix {
            println!("     {} Adding windevkit to PATH...", "🔧".bold());
            match path::add_to_path(&home) {
                Ok(_) => {
                    print_check("PATH configured", true);
                }
                Err(e) => {
                    println!("     {} Failed: {}", "✗".red(), e);
                    all_ok = false;
                }
            }
        } else {
            all_ok = false;
        }
    }

    // Check 5: Developer Mode
    let dev_mode = detect_dev_mode();
    print_check("Windows Developer Mode", dev_mode);
    if !dev_mode {
        println!(
            "     {} Enable for symlink support without admin rights",
            "→".yellow()
        );
        println!(
            "       reg add HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock /t REG_DWORD /v AllowDevelopmentWithoutDevLicense /d 1 /f"
        );
        if fix {
            println!(
                "     {} Cannot auto-enable Developer Mode (requires admin rights).",
                "⚠".yellow()
            );
            println!("       Run the reg command above as Administrator.");
        }
        all_ok = false;
    }

    println!();
    if all_ok {
        println!("{} All checks passed!", "✅".green().bold());
    } else {
        println!(
            "{} Some issues detected. Use --fix to auto-repair.",
            "⚠".yellow().bold()
        );
    }

    Ok(())
}

fn print_check(label: &str, ok: bool) {
    let mark = if ok { "✓".green() } else { "✗".red() };
    println!("  {}  {}", mark, label);
}

fn kind_icon(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::Node => "🟢",
        RuntimeKind::Java => "🟠",
        RuntimeKind::Maven => "🔵",
    }
}

fn detect_dev_mode() -> bool {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-ItemPropertyValue -Path 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\AppModelUnlock' -Name AllowDevelopmentWithoutDevLicense -ErrorAction SilentlyContinue",
        ])
        .output();
    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim() == "1",
        Err(_) => false,
    }
}
