use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use oorandom::Rand64;

mod commands;
mod viriformat_ext;

use commands::*;

fn seeded_rng() -> Rand64 {
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("valid")
        .as_nanos();

    Rand64::new(seed)
}

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
    Scaling(scaling::ScalingOptions),
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
        Command::Scaling(opts) => scaling::run(opts)?,
    }

    Ok(ExitCode::SUCCESS)
}
