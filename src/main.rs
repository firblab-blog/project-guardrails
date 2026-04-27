mod cli;
mod commands;
mod config;
mod diagnostics;
mod enforcement;
mod git;
mod managed_block;
mod mcp;
mod operations;
mod output;
mod profile;
mod profile_lock;
mod rule_engine;
mod state;

use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    cli.run()
}
