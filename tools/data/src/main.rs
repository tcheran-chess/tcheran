use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod rand;

use commands::*;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Count(count::CountOptions),
    Head(head::HeadOptions),
    Interleave(interleave::InterleaveOptions),
    Relabel(relabel::RelabelOptions),
    Convert(convert::ConvertOptions),
}

pub fn main() -> Result<ExitCode> {
    engine::init();

    let cli = Cli::parse();

    match &cli.command {
        Command::Head(opts) => head::run(opts)?,
        Command::Count(opts) => count::run(opts)?,
        Command::Interleave(opts) => interleave::run(opts)?,
        Command::Relabel(opts) => relabel::run(opts)?,
        Command::Convert(opts) => convert::run(opts)?,
    }

    Ok(ExitCode::SUCCESS)
}
