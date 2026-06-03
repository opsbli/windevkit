//! CLI command tree and argument parsing.

pub mod commands;

use clap::{Parser, Subcommand};

/// Windows Development Environment Toolkit
///
/// One-command runtime install, version switching, and app export/import
/// for fresh Windows setup.
#[derive(Parser, Debug)]
#[command(name = "windevkit", version, about, long_about = None)]
pub struct Cli {
    /// Verbose mode
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Mirror for downloads (aliyun, huawei, npmmirror, direct)
    #[arg(long = "mirror", global = true)]
    pub mirror: Option<String>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize ~/.windevkit directory and config
    Init(commands::init::InitArgs),

    /// Install a runtime (node, java, maven)
    Install(commands::install::InstallArgs),

    /// Switch active version of a runtime
    Use(commands::use_version::UseArgs),

    /// Run a command in a specific version's environment
    Exec(commands::exec::ExecArgs),

    /// List installed runtime versions
    List(commands::list::ListArgs),

    /// Uninstall a runtime version
    Uninstall(commands::uninstall::UninstallArgs),

    /// Application management (scan, export, import)
    #[command(subcommand)]
    App(commands::app::AppCommands),

    /// Update windevkit to the latest version
    SelfUpdate,

    /// Diagnose and repair environment issues
    Doctor {
        /// Automatically fix detected issues
        #[arg(long)]
        fix: bool,
    },

    /// Rollback PATH to last snapshot
    Restore,

    /// Show environment overview
    Status {
        /// Show detailed status
        #[arg(long, short)]
        verbose: bool,
    },

    /// Cache management
    #[command(subcommand)]
    Cache(commands::cache::CacheCommands),
}

impl Cli {
    /// Get the effective mirror: CLI flag overrides config.
    fn effective_mirror(&self) -> String {
        self.mirror
            .clone()
            .filter(|m| !m.is_empty())
            .or_else(|| crate::config::Config::load().ok().map(|c| c.core.mirror))
            .unwrap_or_else(|| "direct".to_string())
    }

    /// Execute the selected command
    pub fn execute(&self) -> anyhow::Result<()> {
        match &self.command {
            Commands::Init(args) => commands::init::execute_with(args, self.mirror.as_deref()),
            Commands::Install(args) => commands::install::execute_with(args, &self.effective_mirror()),
            Commands::Use(args) => commands::use_version::execute(args),
            Commands::Exec(args) => commands::exec::execute(args),
            Commands::List(args) => commands::list::execute(args),
            Commands::Uninstall(args) => commands::uninstall::execute(args),
            Commands::App(cmd) => commands::app::execute(cmd),
            Commands::SelfUpdate => commands::self_update::execute(),
            Commands::Doctor { fix } => commands::doctor::execute(*fix),
            Commands::Restore => commands::restore::execute(),
            Commands::Status { verbose } => commands::status::execute(*verbose),
            Commands::Cache(cmd) => commands::cache::execute(cmd),
        }
    }
}

/// Initialize tracing/logging
pub fn init_logging() -> anyhow::Result<()> {
    use tracing_subscriber::FmtSubscriber;

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
