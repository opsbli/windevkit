//! Global configuration management.
//!
//! Reads/writes `~/.windevkit/config.toml`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global windevkit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub core: CoreConfig,
    pub env: EnvConfig,
    pub runtimes: RuntimesConfig,
    pub app_scan: AppScanConfig,
    pub app_export: AppExportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub dev_mode: bool,
    pub mirror: String,
    pub export_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub path_scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimesConfig {
    pub node: RuntimeEntry,
    pub java: RuntimeEntry,
    pub maven: RuntimeEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub default: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppScanConfig {
    pub exclude_patterns: Vec<String>,
    pub include_scoop: bool,
    pub include_choco: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppExportConfig {
    pub auto_download_installers: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            core: CoreConfig {
                dev_mode: false,
                mirror: "direct".into(),
                export_dir: PathBuf::from("~/.windevkit/export"),
            },
            env: EnvConfig {
                path_scope: "user".into(),
            },
            runtimes: RuntimesConfig {
                node: RuntimeEntry { default: String::new() },
                java: RuntimeEntry { default: String::new() },
                maven: RuntimeEntry { default: String::new() },
            },
            app_scan: AppScanConfig {
                exclude_patterns: vec![
                    "KB*".into(),
                    "Microsoft Visual C++*".into(),
                    ".NET*".into(),
                ],
                include_scoop: false,
                include_choco: false,
            },
            app_export: AppExportConfig {
                auto_download_installers: true,
            },
        }
    }
}

impl Config {
    /// Home directory path (~/.windevkit)
    pub fn home_dir() -> PathBuf {
        dirs_data_dir().join(".windevkit")
    }

    /// Config file path (~/.windevkit/config.toml)
    pub fn config_path() -> PathBuf {
        Self::home_dir().join("config.toml")
    }

    /// Load config from disk, or create default if not found.
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

/// Get the user's data directory.
fn dirs_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from("~"))
}
