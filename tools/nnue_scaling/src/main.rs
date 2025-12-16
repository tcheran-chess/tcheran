use std::{
    fs::File,
    io::{BufRead, BufReader},
    process::ExitCode,
};

use engine::{
    chess::game::Game,
    engine::eval::{nnue, nnue::NetworkStack},
};

#[expect(clippy::cast_precision_loss, reason = "All calculations are approximate")]
fn main() -> ExitCode {
    engine::init();

    let file_path = std::env::args()
        .nth(1)
        .expect("File argument should be supplied");
    let file = File::open(file_path).expect("File should open");

    let fens = BufReader::new(file)
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .expect("Should be able to collect FENs");
    let nfens = fens.len();

    let mut total = 0i128;
    let mut count = 0i128;
    let mut abs_total = 0i128;
    let mut min = i32::MAX;
    let mut max = i32::MIN;

    for (i, fen) in fens.iter().enumerate() {
        let game = Game::from_fen(fen).unwrap();
        let mut nnue = NetworkStack::new();
        nnue.setup(&game.board);

        let eval = nnue.evaluate(&game).0;

        count += 1;
        total += i128::from(eval);
        abs_total += i128::from(eval.abs());

        if eval < min {
            min = eval;
        }
        if eval > max {
            max = eval;
        }

        if i % 1024 == 0 {
            print!("\r{}/{}", i + 1, nfens);
        }
    }

    println!("Stats:");
    println!("FENs: {count:>7}");

    let mean = total as f64 / count as f64;
    let abs_mean = abs_total as f64 / count as f64;
    let min = f64::from(min);
    let max = f64::from(max);

    println!("Average: {mean}");
    println!("Average (abs): {abs_mean}");
    println!("Min: {min}");
    println!("Max: {max}");

    // Average from network v10 run against the Lichess Big3 dataset
    let original_avg = 838.36;
    let scale = original_avg / abs_mean * f64::from(nnue::SCALE);

    println!("\nScale: {scale:.6}");

    ExitCode::SUCCESS
}
