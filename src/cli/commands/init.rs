use clap::Args;

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Force re-initialization even if already configured
    #[arg(long)]
    pub force: bool,
}

pub fn execute(args: &InitArgs) -> anyhow::Result<()> {
    println!("🏗️  Initializing windevkit... (--force: {})", args.force);
    // TODO: implement
    Ok(())
}
