mod commands;
mod daemon_client;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = commands::Cli::parse();
    commands::execute(cli)
}
