use clap::Args;

#[derive(Args, Debug)]
pub struct UseArgs {
    /// Runtime name: node, java, maven
    pub tool: String,

    /// Version to activate
    pub version: String,
}

pub fn execute(args: &UseArgs) -> anyhow::Result<()> {
    println!("🔗  Switching {} to {}...", args.tool, args.version);
    // TODO: implement
    Ok(())
}
