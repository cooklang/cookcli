use clap::Args;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct VersionArgs {}

pub fn run(_context: &crate::Context, _args: VersionArgs) -> anyhow::Result<()> {
    println!("Hello!!!");
    Ok(())
}
