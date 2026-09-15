use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod viriformat_ext;

use commands::*;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Count(count::CountOptions),
    Convert(convert::ConvertOptions),
    Scaling(scaling::ScalingOptions),
}

pub fn main() -> Result<ExitCode> {
    engine::init();

    let cli = Cli::parse();

    match &cli.command {
        Command::Count(opts) => count::run(opts)?,
        Command::Convert(opts) => convert::run(opts)?,
        Command::Scaling(opts) => scaling::run(opts)?,
    }

    Ok(ExitCode::SUCCESS)
}
