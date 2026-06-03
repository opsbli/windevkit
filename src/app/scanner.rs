//! Application scanner — detects installed apps via winget and registry.

use std::collections::HashMap;
use std::os::windows::process::CommandExt;

use crate::app::{AppEntry, AppSource};

/// Scan the system for installed applications.
///
/// Returns a merged, deduplicated list from all available sources.
pub fn scan(exclude_patterns: &[String]) -> anyhow::Result<Vec<AppEntry>> {
    let mut all: Vec<AppEntry> = Vec::new();

    // Source 1: winget
    match scan_winget() {
        Ok(apps) => {
            tracing::info!("winget scan: {} apps found", apps.len());
            all.extend(apps);
        }
        Err(e) => {
            tracing::warn!("winget scan failed (may not be installed): {}", e);
        }
    }

    // Source 2: Registry
    match scan_registry() {
        Ok(apps) => {
            tracing::info!("registry scan: {} apps found", apps.len());
            all.extend(apps);
        }
        Err(e) => {
            tracing::warn!("registry scan failed: {}", e);
        }
    }

    // Deduplicate: keep the first occurrence (winget > registry)
    let merged = deduplicate(all);

    // Apply exclude patterns
    let filtered = apply_excludes(merged, exclude_patterns);

    tracing::info!("scan complete: {} apps after dedup+filter", filtered.len());
    Ok(filtered)
}

/// Scan apps via `winget list`.
fn scan_winget() -> anyhow::Result<Vec<AppEntry>> {
    let output = std::process::Command::new("winget")
        .args(["list", "--accept-source-agreements"])
        .output()
        .map_err(|e| anyhow::anyhow!("winget not found: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new()); // winget may return non-zero if no results
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();

    // winget list outputs a table like:
    // Name               Id                   Version    Available
    // ----------------------------------------------------------------
    // Google Chrome      Google.Chrome        131.0.6778.205
    // 7-Zip              7zip.7zip            24.09
    //
    // We skip header rows and parse the table.

    for line in stdout.lines() {
        let line = line.trim();
        // Skip empty lines, header separators, and headers
        if line.is_empty()
            || line.starts_with("---")
            || line.starts_with("Name")
            || line.starts_with("─")
        {
            continue;
        }

        // Parse: tab/space-separated columns
        // Format: Name  Id  Version  [Available]
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        // The Id is typically the second field containing a dot (e.g., "Google.Chrome")
        // If the name has spaces, fields are harder to parse.
        // Strategy: find the Id field (contains a dot) and extract name before it.

        let (name, id, version) = parse_winget_line(line);
        if let (Some(n), Some(i), Some(v)) = (name, id, version) {
            apps.push(AppEntry {
                id: i.to_string(),
                name: n.to_string(),
                version: v.to_string(),
                source: AppSource::Winget,
                selected: true,
                install_path: None,
                silent_args: guess_silent_args(&i),
            });
        }
    }

    Ok(apps)
}

/// Extract (name, id, version) from a winget list line.
fn parse_winget_line(line: &str) -> (Option<String>, Option<String>, Option<String>) {
    let trimmed = line.trim();

    // Split by 2+ spaces (winget uses multi-space alignment)
    let fields: Vec<&str> = trimmed.split("  ").map(|s| s.trim()).filter(|s| !s.is_empty()).collect();

    if fields.len() >= 3 {
        let name = fields[0];
        let id = fields[1];
        let version = fields[2];
        if id.contains('.') {
            return (Some(name.to_string()), Some(id.to_string()), Some(version.to_string()));
        }
    }

    // Fallback: try splitting by whitespace, looking for the Id with a dot
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    for (i, token) in tokens.iter().enumerate() {
        if token.contains('.') && !token.ends_with('.') {
            let name = tokens[..i].join(" ");
            let version = tokens.get(i + 1).copied().map(|v| v.to_string());
            return (Some(name), Some(token.to_string()), version);
        }
    }

    (None, None, None)
}

/// Scan apps from Windows registry.
fn scan_registry() -> anyhow::Result<Vec<AppEntry>> {
    let mut apps = Vec::new();

    // Check both 64-bit and 32-bit registry locations
    let reg_paths = [
        r"HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKLM\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        r"HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    for reg_path in &reg_paths {
        match read_registry_uninstall(reg_path) {
            Ok(entries) => apps.extend(entries),
            Err(e) => tracing::debug!("Failed to read registry path {}: {}", reg_path, e),
        }
    }

    Ok(apps)
}

/// Read subkeys from a registry uninstall path and extract app info.
fn read_registry_uninstall(reg_path: &str) -> anyhow::Result<Vec<AppEntry>> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // First list all subkeys
    let output = std::process::Command::new("reg")
        .args(["query", reg_path])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut apps = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        // Each subkey is listed as a registry path
        if line.starts_with(reg_path) {
            let subkey = line.trim();
            // Read DisplayName and DisplayVersion from this subkey
            if let Some(entry) = read_registry_entry(subkey) {
                apps.push(entry);
            }
        }
    }

    Ok(apps)
}

/// Read a single registry subkey for display name and version.
fn read_registry_entry(subkey: &str) -> Option<AppEntry> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // Get DisplayName
    let name = get_registry_value(subkey, "DisplayName", CREATE_NO_WINDOW)?;
    // Skip Windows system entries
    if name.starts_with("KB") || name.contains("Update for") || name.contains("Hotfix") {
        return None;
    }

    let version = get_registry_value(subkey, "DisplayVersion", CREATE_NO_WINDOW)
        .unwrap_or_default();

    Some(AppEntry {
        id: subkey.rsplit('\\').next().unwrap_or(&name).to_string(),
        name,
        version,
        source: AppSource::Registry,
        selected: true,
        install_path: get_registry_value(subkey, "InstallLocation", CREATE_NO_WINDOW),
        silent_args: None,
    })
}

/// Get a single registry value by key and value name.
fn get_registry_value(key: &str, value_name: &str, flags: u32) -> Option<String> {
    let output = std::process::Command::new("reg")
        .args(["query", key, "/v", value_name])
        .creation_flags(flags)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse: "    DisplayName    REG_SZ    Value"
    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(4, ' ')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 3 {
            // The value is the last part
            let value = parts.last()?;
            if !value.is_empty() && value != &value_name {
                return Some(value.to_string());
            }
        }
    }

    None
}

/// Deduplicate apps by name + version, keeping the first occurrence.
fn deduplicate(apps: Vec<AppEntry>) -> Vec<AppEntry> {
    let mut seen: HashMap<String, bool> = HashMap::new();
    let mut result = Vec::new();

    for app in apps {
        let key = format!("{}@{}", app.name.to_lowercase(), app.version.to_lowercase());
        if seen.contains_key(&key) {
            continue;
        }
        seen.insert(key, true);
        result.push(app);
    }

    result
}

/// Apply exclude patterns to filter out unwanted apps.
fn apply_excludes(apps: Vec<AppEntry>, patterns: &[String]) -> Vec<AppEntry> {
    if patterns.is_empty() {
        return apps;
    }

    apps.into_iter()
        .filter(|app| {
            for pattern in patterns {
                if matches_pattern(&app.name, pattern)
                    || matches_pattern(&app.id, pattern)
                {
                    return false;
                }
            }
            true
        })
        .collect()
}

/// Simple wildcard matching (* matches anything).
fn matches_pattern(value: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix('*') {
        value.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        value.starts_with(prefix)
    } else {
        value == pattern
    }
}

/// Guess silent install arguments for well-known apps by winget ID.
fn guess_silent_args(winget_id: &str) -> Option<String> {
    let known = [
        ("Google.Chrome", "/silent /install"),
        ("Google.Chrome.Enterprise", "/silent /install"),
        ("Mozilla.Firefox", "-ms"),
        ("7zip.7zip", "/S"),
        ("RARLab.WinRAR", "/s"),
        ("VideoLAN.VLC", "/S"),
        ("Adobe.Acrobat.Reader", "/sAll /msi EULA_ACCEPT=YES"),
        ("Microsoft.PowerToys", "--silent"),
        ("Microsoft.VisualStudioCode", "/verysilent /suppressmsgboxes"),
        ("Git.Git", "/SILENT"),
        ("Notepad++.Notepad++", "/S"),
        ("OBSProject.OBSStudio", "/S"),
        ("Spotify.Spotify", "--silent"),
        ("Discord.Discord", "-s"),
        ("SlackTechnologies.Slack", "--silent"),
        ("Dropbox.Dropbox", "-s"),
        ("Oracle.Java", "/s"),
        ("Python.Launcher", "--silent"),
    ];

    for (id, args) in &known {
        if winget_id == *id || winget_id.starts_with(id) {
            return Some(args.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_winget_line_parsing() {
        let (name, id, version) = parse_winget_line("Google Chrome  Google.Chrome  131.0.6778.205");
        assert_eq!(name.as_deref(), Some("Google Chrome"));
        assert_eq!(id.as_deref(), Some("Google.Chrome"));
        assert_eq!(version.as_deref(), Some("131.0.6778.205"));
    }

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("KB5041234", "KB*"));
        assert!(matches_pattern("Microsoft Visual C++ 2022", "Microsoft Visual C++*"));
        assert!(!matches_pattern("Google Chrome", "KB*"));
    }

    #[test]
    fn test_deduplicate() {
        let apps = vec![
            AppEntry {
                id: "Google.Chrome".into(),
                name: "Google Chrome".into(),
                version: "131.0".into(),
                source: AppSource::Winget,
                selected: true,
                install_path: None,
                silent_args: None,
            },
            AppEntry {
                id: "{GUID}".into(),
                name: "Google Chrome".into(),
                version: "131.0".into(),
                source: AppSource::Registry,
                selected: true,
                install_path: None,
                silent_args: None,
            },
        ];
        let deduped = deduplicate(apps);
        assert_eq!(deduped.len(), 1);
    }
}
