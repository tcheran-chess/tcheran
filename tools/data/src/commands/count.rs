use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;
use clap::Args;
use viriformat::dataformat::Game;

#[derive(Debug, Args)]
pub struct CountOptions {
    pub input: PathBuf,
}

pub fn run(options: &CountOptions) -> Result<()> {
    println!("Reading from [{}]", options.input.display());

    let file = File::open(&options.input)?;
    let bytes = file.metadata()?.len();

    let mut reader = BufReader::new(file);
    let mut games = 0usize;
    let mut positions = 0usize;
    let mut kept = 0;
    let mut filtered = 0;

    let mut wins = 0;
    let mut losses = 0;
    let mut draws = 0;

    let mut buffer = Vec::new();

    let filter = viriformat::dataformat::Filter::default();

    while let Ok(game) = Game::deserialise_from(&mut reader, buffer) {
        games += 1;

        let all_positions = game.len();
        positions += all_positions;

        let actual_positions_after_filtering = usize::try_from(game.filter_pass_count(&filter))?;
        let filtered_in_this_game = game.moves.len() - actual_positions_after_filtering;
        kept += actual_positions_after_filtering;
        filtered += filtered_in_this_game;

        if games.is_multiple_of(16384) {
            print!("Counted {games} games\r");
        }

        if game.moves.is_empty() {
            buffer = game.moves;
            buffer.clear();
            continue;
        }

        match game.outcome() {
            viriformat::dataformat::WDL::Win => wins += 1,
            viriformat::dataformat::WDL::Draw => draws += 1,
            viriformat::dataformat::WDL::Loss => losses += 1,
        }

        buffer = game.moves;
        buffer.clear();
    }

    println!();
    println!("Summary:");
    println!("Games = {games}");
    println!("Positions = {positions} (kept={kept}, filtered={filtered})");
    println!("Wins = {wins}, Draws = {draws}, Losses = {losses}");
    println!("Bytes per position = {}", bytes as f64 / positions as f64);

    Ok(())
}
