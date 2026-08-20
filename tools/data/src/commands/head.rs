use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Args;
use viriformat::dataformat::Game;

#[derive(Debug, Args)]
pub struct HeadOptions {
    pub input: PathBuf,
    pub output: PathBuf,
    pub games: usize,
}

pub fn run(options: &HeadOptions) -> Result<()> {
    println!("Writing to [{}]", options.output.display());
    println!("Reading from [{}]", options.input.display());

    let mut reader = BufReader::new(File::open(&options.input)?);
    let mut writer = BufWriter::new(File::create(&options.output)?);

    let mut buffer = Vec::new();
    let mut games = 0usize;
    let total = options.games;

    while Game::deserialise_fast_into_buffer(&mut reader, &mut buffer).is_ok() {
        writer.write_all(&buffer)?;
        buffer.clear();

        games += 1;

        if games.is_multiple_of(16384) {
            print!("Written {games} / {total} ({:.2}%)\r", games as f64 / total as f64 * 100.0);
        }

        if games == total {
            break;
        }
    }

    println!("Written {games} / {total} ({:.2}%)", games as f64 / total as f64 * 100.0);

    Ok(())
}
