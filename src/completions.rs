use anyhow::Result;
use clap::{Args, CommandFactory};
use clap_complete::Shell;

use crate::args::CliArgs;

/// Name of the installed binary, which the generated script completes.
const BIN_NAME: &str = "cook";

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// Shell to generate the completion script for
    #[arg(value_enum)]
    pub shell: Shell,
}

/// Print a completion script for `shell` to stdout.
///
/// The script is generated from the live clap definition, so it always
/// matches the binary that produced it: subcommands compiled out by a cargo
/// feature are absent from the script too.
pub fn run(args: CompletionsArgs) -> Result<()> {
    let mut cmd = CliArgs::command();
    // `cmd.get_name()` is the package name ("cookcli"); the script has to
    // register against the binary users actually type.
    clap_complete::generate(args.shell, &mut cmd, BIN_NAME, &mut std::io::stdout());
    Ok(())
}
