use clap::Args;
use colored::Colorize;

use crate::config::Config;
use crate::runtime::{self, RuntimeKind, symlink};

#[derive(Args, Debug)]
pub struct ExecArgs {
    /// Runtime name: node, java
    pub tool: String,

    /// Version to use for this command
    pub version: String,

    /// Command and arguments to run
    #[arg(last = true, required = true)]
    pub command: Vec<String>,
}

pub fn execute(args: &ExecArgs) -> anyhow::Result<()> {
    let kind: RuntimeKind = args.tool.parse()?;
    let home = Config::home_dir();
    let version_dir = symlink::version_dir(&home, kind, &args.version);

    if !version_dir.exists() {
        anyhow::bail!(
            "{} {} {} is not installed. Use `windevkit install {} {}` first.",
            "✗".red(),
            kind,
            args.version,
            kind,
            args.version
        );
    }

    let bin_dir = runtime::bin_dir(kind, &version_dir);

    // Build the PATH with the target runtime's bin dir first
    let current_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{};{}", bin_dir.display(), current_path);

    println!(
        "⚡ Running with {} {}: {}",
        kind.to_string().bold(),
        args.version,
        args.command.join(" ").dimmed()
    );

    let status = std::process::Command::new(&args.command[0])
        .args(&args.command[1..])
        .env("PATH", &new_path)
        .spawn()?
        .wait()?;

    std::process::exit(status.code().unwrap_or(1));
}
