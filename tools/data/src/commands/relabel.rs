use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Args;
use viriformat::{
    chess::board::{DrawType, GameOutcome, WinType},
    dataformat::{Game, WDL},
};

#[derive(Debug, Args)]
pub struct RelabelOptions {
    pub input: PathBuf,
}

pub fn run(options: &RelabelOptions) -> Result<()> {
    println!("Reading from [{}]", options.input.display());

    let file = File::open(&options.input)?;

    let mut reader = BufReader::new(file);
    let mut games = 0usize;

    let mut wins = 0;
    let mut losses = 0;
    let mut draws = 0;

    let mut relabelled_draw = 0;
    let mut relabelled_win = 0;
    let mut empty_games = 0;

    let mut buffer = Vec::new();
    let mut writer = BufWriter::new(File::create("relabelled.viri")?);

    while let Ok(mut game) = Game::deserialise_from(&mut reader, buffer) {
        games += 1;

        if games.is_multiple_of(16384) {
            print!("Saw {games} games\r");
        }

        if game.moves.is_empty() {
            empty_games += 1;
            buffer = game.moves;
            buffer.clear();
            continue;
        }

        let (_, score) = game.moves.last().unwrap();
        let score = score.get();

        if game.outcome() == WDL::Loss {
            if score.abs() <= 10 {
                relabelled_draw += 1;
                game.set_outcome(GameOutcome::Draw(DrawType::Adjudication));
            } else if score.is_positive() {
                relabelled_win += 1;
                game.set_outcome(GameOutcome::WhiteWin(WinType::Adjudication));
            }
        }

        game.serialise_into(&mut writer)?;

        match game.outcome() {
            viriformat::dataformat::WDL::Win => wins += 1,
            viriformat::dataformat::WDL::Draw => draws += 1,
            viriformat::dataformat::WDL::Loss => losses += 1,
        }

        buffer = game.moves;
        buffer.clear();
    }

    writer.flush()?;

    println!();
    println!("Summary:");
    println!("Games = {games}");
    println!("Wins = {wins} | Losses = {losses} | Draws = {draws}");
    println!("Relabelled to draw: {relabelled_draw}");
    println!("Relabelled to win: {relabelled_win}");
    println!("Empty games: {empty_games}");

    Ok(())
}
