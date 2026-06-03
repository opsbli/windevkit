use clap::Args;

#[derive(Args, Debug)]
pub struct UninstallArgs {
    /// Runtime name: node, java, maven
    pub tool: String,

    /// Version to uninstall. If omitted, requires --all
    pub version: Option<String>,

    /// Remove all installed versions of this runtime
    #[arg(long)]
    pub all: bool,
}

pub fn execute(args: &UninstallArgs) -> anyhow::Result<()> {
    println!(
        "🗑️  Uninstalling {} {:?} (--all: {})...",
        args.tool, args.version, args.all
    );
    // TODO: implement
    Ok(())
}
