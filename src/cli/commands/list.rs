use clap::Args;

#[derive(Args, Debug)]
pub struct ListArgs {
    /// Runtime name: node, java, maven
    pub tool: String,
}

pub fn execute(args: &ListArgs) -> anyhow::Result<()> {
    println!("📋  Installed {} versions:", args.tool);
    // TODO: implement
    Ok(())
}
