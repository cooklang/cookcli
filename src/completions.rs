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
    //
    // Generating into a buffer rather than straight onto stdout keeps a
    // closed downstream pipe from becoming a panic: `clap_complete` unwraps
    // its writes, so streaming `cook completions bash | head -1` used to die
    // with a BrokenPipe panic (exit 101). Ignoring a failed write here is
    // correct — once the reader is gone there is nobody left to write to.
    let mut script = Vec::new();
    clap_complete::generate(args.shell, &mut cmd, BIN_NAME, &mut script);

    use std::io::Write;
    match std::io::stdout().write_all(&script) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.into()),
    }
}
