//! Application scanner — detects installed apps via winget and registry.

use std::collections::HashMap;
use std::os::windows::process::CommandExt;

use base64::Engine;
use serde::Deserialize;
use crate::app::{AppEntry, AppSource};

/// Scan the system for installed applications.
///
/// Returns a merged, deduplicated list from all available sources.
pub fn scan(exclude_patterns: &[String]) -> anyhow::Result<Vec<AppEntry>> {
    let mut all: Vec<AppEntry> = Vec::new();
    let built_in = built_in_excludes();

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

    // Apply built-in excludes first, then user excludes
    let filtered = apply_excludes(apply_excludes(merged, &built_in), exclude_patterns);

    tracing::info!("scan complete: {} apps after dedup+filter", filtered.len());
    Ok(filtered)
}

/// Scan apps via `winget list`.
fn scan_winget() -> anyhow::Result<Vec<AppEntry>> {
    let winget = find_winget_exe().ok_or_else(|| anyhow::anyhow!("winget not found: program not found"))?;
    let output = std::process::Command::new(winget)
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
                installer_type: None,
                portable: None,
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

/// Try to locate winget.exe reliably.
fn find_winget_exe() -> Option<std::path::PathBuf> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // 1) PowerShell Get-Command (works better with App Execution Aliases)
    if let Ok(output) = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-Command winget -ErrorAction SilentlyContinue).Source",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = stdout.lines().map(|s| s.trim()).find(|s| !s.is_empty()) {
                let p = std::path::PathBuf::from(first);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // 2) where.exe PATH resolution
    if let Ok(output) = std::process::Command::new("where.exe")
        .arg("winget.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = stdout.lines().map(|s| s.trim()).find(|s| !s.is_empty()) {
                let p = std::path::PathBuf::from(first);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }

    // 3) WindowsApps app execution alias under LocalAppData
    if let Some(local) = dirs::data_local_dir() {
        let candidate = local.join("Microsoft").join("WindowsApps").join("winget.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

#[derive(Debug, Deserialize)]
struct RegistryAppRow {
    #[serde(rename = "Id")]
    id: Option<String>,
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Version")]
    version: Option<String>,
    #[serde(rename = "InstallPath")]
    install_path: Option<String>,
}

/// Scan apps from Windows registry using ONE PowerShell process.
fn scan_registry() -> anyhow::Result<Vec<AppEntry>> {
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let script = r#"
$paths = @(
  'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall',
  'HKCU:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall'
)

$result = foreach ($path in $paths) {
  if (Test-Path $path) {
    Get-ChildItem -Path $path -ErrorAction SilentlyContinue | ForEach-Object {
      try {
        $p = Get-ItemProperty -Path $_.PSPath -ErrorAction SilentlyContinue
        if ($null -ne $p -and $null -ne $p.DisplayName -and [string]$p.DisplayName -ne '') {
          [PSCustomObject]@{
            Id = if ($_.PSChildName) { [string]$_.PSChildName } else { [string]$p.DisplayName }
            Name = [string]$p.DisplayName
            Version = if ($null -ne $p.DisplayVersion) { [string]$p.DisplayVersion } else { '' }
            InstallPath = if ($null -ne $p.InstallLocation) { [string]$p.InstallLocation } else { '' }
          }
        }
      } catch {}
    }
  }
}

$json = $result | ConvertTo-Json -Compress -Depth 3
if ($null -eq $json) { $json = '[]' }
[Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes([string]$json))
"#;

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()?;

    if !output.status.success() {
        anyhow::bail!("powershell registry scan failed")
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(stdout)
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {e}"))?;
    let json = String::from_utf8(decoded)
        .map_err(|e| anyhow::anyhow!("utf8 decode failed: {e}"))?;

    let rows: Vec<RegistryAppRow> = serde_json::from_str(&json)
        .or_else(|_| serde_json::from_str::<RegistryAppRow>(&json).map(|row| vec![row]))?;

    let apps = rows
        .into_iter()
        .filter_map(|row| {
            let name = row.name?.trim().to_string();
            if name.is_empty()
                || name.starts_with("KB")
                || name.contains("Update for")
                || name.contains("Hotfix")
            {
                return None;
            }

            Some(AppEntry {
                id: row.id.unwrap_or_else(|| name.clone()),
                name,
                version: row.version.unwrap_or_default().trim().to_string(),
                source: AppSource::Registry,
                selected: true,
                install_path: row
                    .install_path
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty()),
                silent_args: None,
                installer_type: None,
                portable: None,
            })
        })
        .collect();

    Ok(apps)
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
    let value = value.to_lowercase();
    let pattern = pattern.to_lowercase();
    if pattern == "*" {
        return true;
    }
    if pattern.starts_with('*') && pattern.ends_with('*') && pattern.len() > 2 {
        let inner = &pattern[1..pattern.len() - 1];
        value.contains(inner)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        value.ends_with(suffix)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        value.starts_with(prefix)
    } else {
        value == pattern
    }
}

fn built_in_excludes() -> Vec<String> {
    vec![
        "KB*".into(),
        "Microsoft Visual C++*".into(),
        ".NET*".into(),
        "Windows SDK*".into(),
        "WinRT Intellisense*".into(),
        "Universal CRT*".into(),
        "Application Verifier*".into(),
        "Windows App Certification Kit*".into(),
        "Windows Desktop Extension SDK*".into(),
        "Windows Team Extension SDK*".into(),
        "Windows Mobile Extension SDK*".into(),
        "Windows IoT Extension SDK*".into(),
        "Windows Software Development Kit*".into(),
        "Kits Configuration Installer".into(),
        "Windows SDK EULA".into(),
        "Windows SDK Redistributables".into(),
        "Windows SDK AddOn".into(),
        "WPT*".into(),
        "vs_*".into(),
        "icecap_*".into(),
        "DiagnosticsHub_*".into(),
        "VBA (*".into(),
        "Python * Documentation*".into(),
        "Python * Tcl/Tk Support*".into(),
        "Python * Development Libraries*".into(),
        "Python * pip Bootstrap*".into(),
        "Python * Test Suite*".into(),
        "Python * Standard Library*".into(),
        "Microsoft Visual Studio Setup *".into(),
        "Microsoft Visual Studio Installer".into(),
        "VS *".into(),
        "Microsoft System CLR Types for SQL Server*".into(),
        "Mozilla Maintenance Service".into(),
        "Microsoft Edge WebView2 Runtime".into(),
        "Windows Subsystem for Linux".into(),
        "Python Launcher".into(),
        "vcpp_crt.redist.clickonce".into(),
    ]
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
                installer_type: None,
                portable: None,
            },
            AppEntry {
                id: "{GUID}".into(),
                name: "Google Chrome".into(),
                version: "131.0".into(),
                source: AppSource::Registry,
                selected: true,
                install_path: None,
                silent_args: None,
                installer_type: None,
                portable: None,
            },
        ];
        let deduped = deduplicate(apps);
        assert_eq!(deduped.len(), 1);
    }
}
