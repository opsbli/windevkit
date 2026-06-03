use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum CacheCommands {
    /// Clean download cache
    Clean,
}

pub fn execute(cmd: &CacheCommands) -> anyhow::Result<()> {
    match cmd {
        CacheCommands::Clean => {
            println!("🧹  Cleaning cache...");
            // TODO: implement
        }
    }
    Ok(())
}
