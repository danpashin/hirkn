#![deny(clippy::pedantic)]
#![warn(clippy::future_not_send)]

#[macro_use]
extern crate async_trait;

mod commands;
mod config;
mod source;

use self::commands::{CliCommand, Command};
use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[clap(about, version)]
struct CLIOptions {
    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    let options: CLIOptions = CLIOptions::parse();
    match options.command {
        Command::Update(command) => command.run().await,
    }
}
