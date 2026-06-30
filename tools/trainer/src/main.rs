use clap::{Parser, Subcommand};

mod bullet_extensions;
mod evals;
mod trainer;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Train { name: String },
    Evals { checkpoint: String },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Train { name } => trainer::run(name),
        Command::Evals { checkpoint } => evals::run_evals(checkpoint),
    }
}
