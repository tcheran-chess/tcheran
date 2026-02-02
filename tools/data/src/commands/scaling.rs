use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};

use anyhow::Result;
use clap::Args;
use engine::{
    chess::game::Game,
    engine::eval::{nnue, nnue::NetworkStack},
};
use rayon::{iter::ParallelIterator, prelude::ParallelSlice};

#[derive(Debug, Args)]
pub struct ScalingOptions {
    pub input: PathBuf,
}

#[derive(Default)]
struct Stats {
    total: i128,
    count: i128,
    abs_total: i128,
    min: i32,
    max: i32,
}

#[expect(clippy::cast_precision_loss, reason = "All calculations are approximate")]
pub fn run(options: &ScalingOptions) -> Result<()> {
    let file = File::open(&options.input)?;

    let fens = BufReader::new(file)
        .lines()
        .collect::<Result<Vec<_>, _>>()?;

    let stats = fens
        .par_chunks(100_000)
        .map(chunk_stats)
        .collect::<Vec<_>>();

    let stats = aggregate_stats(&stats);

    println!("Stats:");
    println!("FENs: {:>7}", stats.count);

    let mean = stats.total as f64 / stats.count as f64;
    let abs_mean = stats.abs_total as f64 / stats.count as f64;
    let min = f64::from(stats.min);
    let max = f64::from(stats.max);

    println!("Average: {mean}");
    println!("Average (abs): {abs_mean}");
    println!("Min: {min}");
    println!("Max: {max}");

    // Average from network v10 run against the Lichess Big3 dataset
    let original_avg = 838.36;
    let scale = original_avg / abs_mean * f64::from(nnue::SCALE);

    println!("\nScale: {scale:.6}");

    Ok(())
}

fn chunk_stats(fens: &[String]) -> Stats {
    let mut stats = Stats::default();

    for fen in fens {
        let game = Game::from_fen(fen).unwrap();
        let mut nnue = NetworkStack::new();
        nnue.setup(&game.board);

        let eval = nnue.evaluate(&game).0;

        stats.count += 1;
        stats.total += i128::from(eval);
        stats.abs_total += i128::from(eval.abs());

        if eval < stats.min {
            stats.min = eval;
        }
        if eval > stats.max {
            stats.max = eval;
        }
    }

    stats
}

fn aggregate_stats(chunk_stats: &[Stats]) -> Stats {
    let mut stats = Stats::default();

    for s in chunk_stats {
        stats.count += s.count;
        stats.total += s.total;
        stats.abs_total += s.abs_total;

        if s.min < stats.min {
            stats.min = s.min;
        }

        if s.max > stats.max {
            stats.max = s.max;
        }
    }

    stats
}
