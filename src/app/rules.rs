use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::app::AppEntry;
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RulesFile {
    #[serde(default)]
    pub rules: Vec<AppRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppRule {
    pub r#match: String,
    pub download_url: Option<String>,
    pub silent_args: Option<String>,
    pub installer_type: Option<String>,
    pub category: Option<String>,
    pub portable: Option<bool>,
}

pub fn user_rules_path() -> PathBuf {
    Config::home_dir().join("rules.toml")
}

pub fn ensure_user_rules_file() -> anyhow::Result<()> {
    let path = user_rules_path();
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, default_rules_template())?;
    Ok(())
}

pub fn load_all_rules() -> Vec<AppRule> {
    let mut rules = built_in_rules();
    if let Ok(user) = load_user_rules()
        && !user.rules.is_empty()
    {
        // user rules override built-ins by precedence, so put them first
        let mut merged = user.rules;
        merged.extend(rules);
        rules = merged;
    }
    rules
}

pub fn resolve_rule(app: &AppEntry) -> Option<AppRule> {
    let name = app.name.to_lowercase();
    let id = app.id.to_lowercase();
    load_all_rules().into_iter().find(|r| {
        let m = r.r#match.to_lowercase();
        !m.is_empty() && (name.contains(&m) || id.contains(&m))
    })
}

pub fn effective_category(app: &AppEntry) -> String {
    if let Some(rule) = resolve_rule(app)
        && let Some(cat) = rule.category
        && !cat.trim().is_empty()
    {
        return cat;
    }
    heuristic_category(app).to_string()
}

pub fn effective_silent_args(app: &AppEntry) -> Option<String> {
    if let Some(args) = &app.silent_args
        && !args.trim().is_empty()
    {
        return Some(args.clone());
    }
    resolve_rule(app).and_then(|r| r.silent_args)
}

pub fn effective_installer_type(app: &AppEntry) -> Option<String> {
    if let Some(installer_type) = &app.installer_type
        && !installer_type.trim().is_empty()
    {
        return Some(installer_type.clone());
    }
    resolve_rule(app).and_then(|r| r.installer_type)
}

pub fn effective_portable(app: &AppEntry) -> bool {
    if let Some(portable) = app.portable {
        return portable;
    }
    resolve_rule(app).and_then(|r| r.portable).unwrap_or(false)
}

fn load_user_rules() -> anyhow::Result<RulesFile> {
    ensure_user_rules_file()?;
    let content = std::fs::read_to_string(user_rules_path())?;
    let rules = toml::from_str::<RulesFile>(&content)?;
    Ok(rules)
}

fn built_in_rules() -> Vec<AppRule> {
    vec![
        AppRule {
            r#match: "google chrome".into(),
            download_url: Some(
                "https://dl.google.com/chrome/install/ChromeStandaloneSetup64.exe".into(),
            ),
            silent_args: Some("/silent /install".into()),
            installer_type: Some("exe".into()),
            category: Some("Browser".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "firefox".into(),
            download_url: Some(
                "https://download.mozilla.org/?product=firefox-latest&os=win64&lang=zh-CN".into(),
            ),
            silent_args: Some("-ms".into()),
            installer_type: Some("exe".into()),
            category: Some("Browser".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "visual studio code".into(),
            download_url: Some(
                "https://update.code.visualstudio.com/latest/win32-x64-user/stable".into(),
            ),
            silent_args: Some("/verysilent /suppressmsgboxes".into()),
            installer_type: Some("exe".into()),
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "git".into(),
            download_url: None,
            silent_args: Some("/SILENT".into()),
            installer_type: Some("exe".into()),
            category: Some("Dev Tool".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "7-zip".into(),
            download_url: None,
            silent_args: Some("/S".into()),
            installer_type: Some("exe".into()),
            category: Some("Utility".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "bandizip".into(),
            download_url: None,
            silent_args: Some("/S".into()),
            installer_type: Some("exe".into()),
            category: Some("Utility".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "docker desktop".into(),
            download_url: None,
            silent_args: Some("install --quiet".into()),
            installer_type: Some("exe".into()),
            category: Some("Dev Tool".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "jetbrains".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "intellij".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "pycharm".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "webstorm".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "goland".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "rustrover".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("IDE".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "python".into(),
            download_url: None,
            silent_args: Some("--quiet InstallAllUsers=0 PrependPath=1".into()),
            installer_type: Some("exe".into()),
            category: Some("Runtime".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "jdk".into(),
            download_url: None,
            silent_args: Some("/s".into()),
            installer_type: Some("exe".into()),
            category: Some("Runtime".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "maven".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("Runtime".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "qq".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("Utility".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "wechat".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("Utility".into()),
            portable: Some(false),
        },
        AppRule {
            r#match: "wps".into(),
            download_url: None,
            silent_args: None,
            installer_type: None,
            category: Some("Utility".into()),
            portable: Some(false),
        },
    ]
}

fn heuristic_category(app: &AppEntry) -> &'static str {
    let name = app.name.to_lowercase();
    let id = app.id.to_lowercase();

    if contains_any(&name, &["chrome", "firefox", "edge", "browser"]) {
        "Browser"
    } else if contains_any(
        &name,
        &[
            "intellij",
            "pycharm",
            "webstorm",
            "goland",
            "rustrover",
            "android studio",
            "visual studio code",
            "zed",
            "windsurf",
            "cursor",
            "idea",
        ],
    ) {
        "IDE"
    } else if contains_any(
        &name,
        &[
            "jdk",
            "java",
            "python",
            "node",
            "go programming language",
            "rustup",
            "maven",
            "powershell",
            "wsl",
        ],
    ) {
        "Runtime"
    } else if contains_any(
        &name,
        &[
            "git", "docker", "cmake", "navicat", "dbeaver", "apifox", "postman", "zellij", "warp",
            "termius", "obsidian",
        ],
    ) {
        "Dev Tool"
    } else if contains_any(
        &name,
        &[
            "qq",
            "微信",
            "wechat",
            "wps",
            "迅雷",
            "todesk",
            "bandizip",
            "listary",
            "directory opus",
            "clash",
            "vpn",
            "music",
            "player",
            "office",
        ],
    ) {
        "Utility"
    } else if contains_any(&id, &["chrome", "firefox", "edge"]) {
        "Browser"
    } else {
        "Other"
    }
}

fn default_rules_template() -> String {
    r#"# windevkit user rules
# User rules override built-in rules by precedence.
# installer_type: exe | msi | zip | portable

[[rules]]
match = "Google Chrome"
download_url = "https://dl.google.com/chrome/install/ChromeStandaloneSetup64.exe"
silent_args = "/silent /install"
installer_type = "exe"
category = "Browser"
portable = false

[[rules]]
match = "Visual Studio Code"
download_url = "https://update.code.visualstudio.com/latest/win32-x64-user/stable"
silent_args = "/verysilent /suppressmsgboxes"
installer_type = "exe"
category = "IDE"
portable = false

[[rules]]
match = "Some Portable Tool"
download_url = "https://example.com/tool.zip"
installer_type = "zip"
category = "Utility"
portable = true
"#
    .to_string()
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| value.contains(&n.to_lowercase()))
}
