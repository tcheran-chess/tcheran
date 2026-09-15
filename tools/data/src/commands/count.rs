use std::{fs::File, io::BufReader, ops::AddAssign, path::PathBuf};

use anyhow::Result;
use clap::Args;
use rayon::prelude::*;
use viriformat::dataformat::Game;

#[derive(Debug, Args)]
pub struct CountOptions {
    pub dir: PathBuf,
}

#[derive(Default)]
struct FileStats {
    games: u64,

    wins: u64,
    losses: u64,
    draws: u64,

    positions: u64,
    kept_positions: u64,
    filtered_positions: u64,
}

impl FileStats {
    fn kept_percent(&self) -> f32 {
        (self.kept_positions as f32 / self.positions as f32) * 100.0
    }
}

impl AddAssign for FileStats {
    fn add_assign(&mut self, rhs: Self) {
        self.games += rhs.games;

        self.wins += rhs.wins;
        self.losses += rhs.losses;
        self.draws += rhs.draws;

        self.positions += rhs.positions;
        self.kept_positions += rhs.kept_positions;
        self.filtered_positions += rhs.filtered_positions;
    }
}

fn file_stats(file: &PathBuf) -> Result<FileStats> {
    let file = File::open(file)?;
    let mut reader = BufReader::new(file);
    let mut buffer = Vec::new();

    let mut stats = FileStats::default();
    let filter = viriformat::dataformat::Filter::default();

    while let Ok(game) = Game::deserialise_from(&mut reader, buffer) {
        stats.games += 1;

        let all_positions = game.len();
        stats.positions += all_positions as u64;

        let actual_positions_after_filtering = usize::try_from(game.filter_pass_count(&filter))?;
        let filtered_in_this_game = game.moves.len() - actual_positions_after_filtering;
        stats.kept_positions += actual_positions_after_filtering as u64;
        stats.filtered_positions += filtered_in_this_game as u64;

        if game.moves.is_empty() {
            buffer = game.moves;
            buffer.clear();
            continue;
        }

        match game.outcome() {
            viriformat::dataformat::WDL::Win => stats.wins += 1,
            viriformat::dataformat::WDL::Draw => stats.draws += 1,
            viriformat::dataformat::WDL::Loss => stats.losses += 1,
        }

        buffer = game.moves;
        buffer.clear();
    }

    Ok(stats)
}

pub fn run(options: &CountOptions) {
    let mut data_paths = std::fs::read_dir(&options.dir)
        .expect("Unable to find data dir")
        .map(|path| path.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();

    data_paths.sort();
    assert!(!data_paths.is_empty(), "No data files found");

    println!("paths:");
    for file in &data_paths {
        println!("  - {}", file.display());
    }

    println!();

    let mut aggregate_stats = FileStats::default();

    println!("stats:");
    let stats = data_paths
        .par_iter()
        .map(|file| {
            let stats = file_stats(file).unwrap();

            println!(
                "- {} positions: {}, kept {} ({:.2}%)",
                file.display(),
                stats.positions,
                stats.kept_positions,
                stats.kept_percent()
            );

            stats
        })
        .collect::<Vec<_>>();

    for s in stats {
        aggregate_stats += s;
    }

    println!();

    println!("summary:");
    println!("games: {}", aggregate_stats.games);
    println!(
        "positions: {}, kept: {} ({:.2}%)",
        aggregate_stats.positions,
        aggregate_stats.kept_positions,
        aggregate_stats.kept_percent()
    );
    println!(
        "wins: {}, draws: {}, losses: {}",
        aggregate_stats.wins, aggregate_stats.draws, aggregate_stats.losses
    );
}
