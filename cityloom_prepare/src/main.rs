use clap::Parser;

use cityloom_prepare::commands::{self, Cli};

fn main() {
    // Each command installs the Ctrl-C handler itself (`progress::exit_on_ctrl_c`): ctrlc allows one per process.
    let cli = Cli::parse();
    std::process::exit(match commands::dispatch(cli) {
        Ok(()) => 0,
        Err(code) => code,
    });
}
