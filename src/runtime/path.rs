//! PATH environment variable management for Windows.

use colored::Colorize;
use std::path::Path;

/// Get the `active/bin` path that should be in PATH.
pub fn active_bin_path(home: &Path) -> std::path::PathBuf {
    home.join("active").join("bin")
}

/// Check if the active/bin path is already in the user PATH.
pub fn is_in_path(home: &Path) -> bool {
    let bin_path = active_bin_path(home);
    let bin_str = bin_path.to_string_lossy().to_lowercase();

    current_user_path().iter().any(|p| p.to_lowercase() == bin_str)
}

/// Add the active/bin path to the user PATH (persistent, via registry).
pub fn add_to_path(home: &Path) -> anyhow::Result<()> {
    let bin_path = active_bin_path(home);
    let bin_str = bin_path.to_string_lossy().to_string();

    if is_in_path(home) {
        tracing::info!("PATH already contains: {}", bin_str);
        return Ok(());
    }

    let mut current = current_user_path();

    // Ensure the path uses the actual full path (resolve ~ if needed)
    let expanded = std::fs::canonicalize(&bin_path).unwrap_or(bin_path.clone());
    let expanded_str = expanded.to_string_lossy().to_string();

    current.push(expanded_str);
    set_user_path(&current)?;

    tracing::info!("Added to PATH: {}", bin_str);
    // NOTE: The change only affects NEW command prompts, not the current one
    println!(
        "  {} Added to PATH: {}",
        "✓".green(),
        bin_str.cyan()
    );
    println!(
        "  {} Restart your terminal or run: $env:Path = [Environment]::GetEnvironmentVariable('Path','User')",
        "💡".yellow()
    );

    Ok(())
}

/// Remove the active/bin path from the user PATH.
pub fn remove_from_path(home: &Path) -> anyhow::Result<()> {
    let bin_path = active_bin_path(home);
    let bin_str = bin_path.to_string_lossy().to_lowercase();

    let current = current_user_path();
    let filtered: Vec<String> = current
        .into_iter()
        .filter(|p| {
            let lower = p.to_lowercase();
            lower != bin_str && !lower.contains(".windevkit")
        })
        .collect();

    if filtered.len() < current_user_path().len() {
        set_user_path(&filtered)?;
        tracing::info!("Removed windevkit paths from PATH");
    }

    Ok(())
}

/// Get the current user PATH as a list of strings.
fn current_user_path() -> Vec<String> {
    // Read from registry (persistent user PATH)
    get_registry_path().unwrap_or_else(|_| {
        // Fallback: read from environment
        std::env::var("PATH")
            .unwrap_or_default()
            .split(';')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

/// Read user PATH from Windows registry.
fn get_registry_path() -> std::io::Result<Vec<String>> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let output = std::process::Command::new("reg")
        .args([
            "query",
            "HKCU\\Environment",
            "/v",
            "Path",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse reg.exe output: look for the REG_EXPAND_SZ or REG_SZ value
    for line in stdout.lines() {
        let line = line.trim();
        // Typical output: "    Path    REG_EXPAND_SZ    C:\Users\..."
        // or: "    Path    REG_SZ    C:\Users\..."
        if let Some(value) = line
            .splitn(4, ' ')
            .nth(3)
            .or_else(|| line.split("    ").nth(2))
        {
            let expanded = expand_environment_variables(value);
            return Ok(expanded
                .split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect());
        }
    }

    Ok(Vec::new())
}

/// Write user PATH to Windows registry.
fn set_user_path(paths: &[String]) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let path_str = paths.join(";");

    // Use REG_EXPAND_SZ to support %USERPROFILE% etc.
    let status = std::process::Command::new("reg")
        .args([
            "add",
            "HKCU\\Environment",
            "/v",
            "Path",
            "/t",
            "REG_EXPAND_SZ",
            "/d",
            &path_str,
            "/f",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to update PATH in registry (exit code: {:?})", status.code());
    }

    // Broadcast WM_SETTINGCHANGE so Explorer and new processes pick up the change
    let _ = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "& { [Environment]::SetEnvironmentVariable('Path', $env:Path, 'User'); \
             [Environment]::SetEnvironmentVariable('Path', $env:Path, 'Process') }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    Ok(())
}

/// Expand Windows environment variables like %USERPROFILE%.
fn expand_environment_variables(s: &str) -> String {
    let mut result = s.to_string();
    // Simple %VAR% expansion using cmd.exe /c echo
    // Only expand if there's a % enclosed variable
    if result.contains('%') && !result.contains("%%") {
        if let Ok(output) = std::process::Command::new("cmd.exe")
            .args(["/c", "echo", &result])
            .output()
        {
            if let Ok(expanded) = String::from_utf8(output.stdout) {
                let trimmed = expanded.trim().to_string();
                if !trimmed.is_empty() && trimmed != result {
                    result = trimmed;
                }
            }
        }
    }
    result
}

/// Snapshot current PATH for backup/restore.
pub fn snapshot_path(home: &Path) -> anyhow::Result<std::path::PathBuf> {
    let backups_dir = home.join("backups");
    std::fs::create_dir_all(&backups_dir)?;

    let timestamp = chrono_now();
    let snapshot_path = backups_dir.join(format!("path-{timestamp}.txt"));

    let current = current_user_path().join(";");
    std::fs::write(&snapshot_path, &current)?;

    tracing::info!("PATH snapshot saved to {}", snapshot_path.display());
    Ok(snapshot_path)
}

/// Get a crude timestamp string without chrono dependency.
fn chrono_now() -> String {
    // Use a simple timestamp: unix seconds
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", start.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_active_bin_path() {
        let home = Path::new("C:\\Users\\test\\.windevkit");
        let bin = active_bin_path(home);
        assert!(bin.to_string_lossy().contains(".windevkit\\active\\bin"));
    }
}
