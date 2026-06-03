use clap::Args;

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
    println!(
        "⚡  Running '{:?}' with {} {}...",
        args.command, args.tool, args.version
    );
    // TODO: implement
    Ok(())
}
