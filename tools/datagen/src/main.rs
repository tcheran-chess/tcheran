use std::{
    io,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    sync::atomic::{AtomicBool, Ordering},
};

use clap::Parser;
use engine::{
    chess::{game::Game, moves::Move, player::Player},
    engine::{
        eval::{Eval, WhiteEval},
        options::EngineOptions,
        search::{CapturingReporter, PersistentState, TimeControl, search},
        tablebases::{Tablebase, Wdl},
    },
};
use jiff::{SpanRound, ToSpan, Unit};
use rand::{Rng, prelude::IndexedRandom};

const DATA_DIR: &str = "datagen";

const DEFAULT_DEPTH: u8 = 8;
const DEFAULT_STARTING_MOVES: usize = 8;
const ADJUDICATION_THRESHOLD: i32 = 2000;
const DRAW_THRESHOLD: i32 = 10;
const ADJUDICATE_WINS_AFTER: usize = 4;
const ADJUDICATE_DRAWS_AFTER: usize = 10;

static STOP: AtomicBool = AtomicBool::new(false);

mod stats {
    use std::sync::atomic::AtomicU64;

    pub static GAMES: AtomicU64 = AtomicU64::new(0);
    pub static FENS: AtomicU64 = AtomicU64::new(0);
    pub static WHITE_WINS: AtomicU64 = AtomicU64::new(0);
    pub static BLACK_WINS: AtomicU64 = AtomicU64::new(0);
    pub static DRAWS: AtomicU64 = AtomicU64::new(0);
    pub static ADJUDICATED_WHITE_WINS: AtomicU64 = AtomicU64::new(0);
    pub static ADJUDICATED_BLACK_WINS: AtomicU64 = AtomicU64::new(0);
    pub static ADJUDICATED_DRAWS: AtomicU64 = AtomicU64::new(0);
    pub static TB_WHITE_WINS: AtomicU64 = AtomicU64::new(0);
    pub static TB_BLACK_WINS: AtomicU64 = AtomicU64::new(0);
    pub static TB_DRAWS: AtomicU64 = AtomicU64::new(0);
}

#[derive(Parser)]
struct Cli {
    games: usize,
    threads: usize,
    depth: Option<u8>,

    #[clap(long)]
    syzygy_path: Option<PathBuf>,
}

enum DatagenMode {
    Depth(u8),
}

struct DatagenConfig {
    games: usize,
    threads: usize,
    mode: DatagenMode,
    tb: Option<Tablebase>,
}

struct PlayerStates {
    white_persistent_state: PersistentState,
    black_persistent_state: PersistentState,
}

impl PlayerStates {
    pub fn new(tt_size: usize, datagen_config: &DatagenConfig) -> Self {
        match &datagen_config.tb {
            Some(tb) => Self {
                white_persistent_state: PersistentState::with_tablebase(tt_size, tb),
                black_persistent_state: PersistentState::with_tablebase(tt_size, tb),
            },
            None => Self {
                white_persistent_state: PersistentState::new(tt_size),
                black_persistent_state: PersistentState::new(tt_size),
            },
        }
    }

    pub fn for_player(&mut self, player: Player) -> &mut PersistentState {
        match player {
            Player::White => &mut self.white_persistent_state,
            Player::Black => &mut self.black_persistent_state,
        }
    }

    pub fn reset(&mut self) {
        self.white_persistent_state.reset();
        self.black_persistent_state.reset();
    }
}

fn datagen(config: &DatagenConfig) {
    let run_id = jiff::Zoned::now().strftime("%Y%m%d-%H%M%S").to_string();
    let dir = format!("{DATA_DIR}/{run_id}");

    println!("Generated data will be saved in {dir}");
    std::fs::create_dir_all(&dir).unwrap();

    assert_eq!(
        config.games % config.threads,
        0,
        "Number of games must be divisible by number of threads"
    );

    let games_per_thread = config.games / config.threads;

    std::thread::scope(|s| {
        for id in 0..config.threads {
            let dir = dir.clone();
            s.spawn(move || datagen_thread(id, games_per_thread, &dir, config));
        }

        s.spawn(move || progress_thread(config.games));
    });

    merge_files(&dir, &run_id);
}

fn merge_files(dir: &str, run_id: &str) {
    let files = std::fs::read_dir(dir).unwrap();
    let mut output_file = std::fs::File::create(format!("{DATA_DIR}/{run_id}.txt")).unwrap();

    for file in files.filter_map(Result::ok) {
        let file_name = file.path();
        let mut file = std::fs::File::open(file_name).unwrap();
        io::copy(&mut file, &mut output_file).unwrap();
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "We are doing approximate progress calculations"
)]
#[expect(
    clippy::cast_possible_truncation,
    reason = "We are doing approximate progress calculations"
)]
fn progress_thread(ngames: usize) {
    let start_time = jiff::Timestamp::now();

    loop {
        if STOP.load(Ordering::SeqCst) {
            break;
        }

        let games_played = stats::GAMES.load(Ordering::SeqCst);
        if games_played == 0 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        if games_played == ngames as u64 {
            // Stop on the next run
            println!("All games generated!");
            STOP.store(true, Ordering::SeqCst);
        }

        let fens_generated = stats::FENS.load(Ordering::SeqCst);
        let elapsed_time = jiff::Timestamp::now() - start_time;
        let elapsed_seconds = elapsed_time.total(Unit::Second).unwrap();
        let fens_per_second = f64::from(u32::try_from(fens_generated).unwrap()) / elapsed_seconds;
        let fens_per_game = fens_generated as f64 / games_played as f64;

        let approx_time_per_game = elapsed_seconds / games_played as f64;
        let number_of_games_remaining = ngames as u64 - games_played;
        let approx_seconds = (approx_time_per_game * number_of_games_remaining as f64) as i64;
        let approx_time_remaining = approx_seconds.seconds();
        let approx_time_remaining = approx_time_remaining
            .round(SpanRound::new().largest(Unit::Day).days_are_24_hours())
            .unwrap();

        let white_wins = stats::WHITE_WINS.load(Ordering::SeqCst);
        let black_wins = stats::BLACK_WINS.load(Ordering::SeqCst);
        let draws = stats::DRAWS.load(Ordering::SeqCst);
        let adjudicated_white_wins = stats::ADJUDICATED_WHITE_WINS.load(Ordering::SeqCst);
        let adjudicated_black_wins = stats::ADJUDICATED_BLACK_WINS.load(Ordering::SeqCst);
        let adjudicated_draws = stats::ADJUDICATED_DRAWS.load(Ordering::SeqCst);
        let tb_white_wins = stats::TB_WHITE_WINS.load(Ordering::SeqCst);
        let tb_black_wins = stats::TB_BLACK_WINS.load(Ordering::SeqCst);
        let tb_draws = stats::TB_DRAWS.load(Ordering::SeqCst);

        let total_white_wins = white_wins + adjudicated_white_wins + tb_white_wins;
        let total_black_wins = black_wins + adjudicated_black_wins + tb_black_wins;
        let total_draws = draws + adjudicated_draws + tb_draws;

        println!(
            "{fens_generated} FENs generated [{fens_per_second:.2}/s] in {elapsed_time:#} from {games_played} games (out of {ngames}) | {approx_time_remaining:#} remaining"
        );

        println!(
            "Avg time per game: {approx_time_per_game:.2}s | Avg positions per game: {fens_per_game:.2} | W: {total_white_wins} B: {total_black_wins} D: {total_draws}"
        );

        println!(
            "White: ({white_wins}/{adjudicated_white_wins}/{tb_white_wins}) | Black: ({black_wins}/{adjudicated_black_wins}/{tb_black_wins}) | Draws: ({draws}/{adjudicated_draws}/{tb_draws})"
        );
        println!();

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

fn update_stats(result: &GameResult) {
    stats::GAMES.fetch_add(1, Ordering::SeqCst);
    stats::FENS.fetch_add(result.positions.len() as u64, Ordering::SeqCst);

    match (&result.outcome, &result.source) {
        (Outcome::WinForWhite, ResultSource::Actual) => {
            stats::WHITE_WINS.fetch_add(1, Ordering::Relaxed)
        }
        (Outcome::Draw, ResultSource::Actual) => stats::DRAWS.fetch_add(1, Ordering::Relaxed),
        (Outcome::LossForWhite, ResultSource::Actual) => {
            stats::BLACK_WINS.fetch_add(1, Ordering::Relaxed)
        }

        (Outcome::WinForWhite, ResultSource::Adjudicated) => {
            stats::ADJUDICATED_WHITE_WINS.fetch_add(1, Ordering::Relaxed)
        }
        (Outcome::Draw, ResultSource::Adjudicated) => {
            stats::ADJUDICATED_DRAWS.fetch_add(1, Ordering::Relaxed)
        }
        (Outcome::LossForWhite, ResultSource::Adjudicated) => {
            stats::ADJUDICATED_BLACK_WINS.fetch_add(1, Ordering::Relaxed)
        }

        (Outcome::WinForWhite, ResultSource::Tablebase) => {
            stats::TB_WHITE_WINS.fetch_add(1, Ordering::Relaxed)
        }
        (Outcome::Draw, ResultSource::Tablebase) => stats::TB_DRAWS.fetch_add(1, Ordering::Relaxed),
        (Outcome::LossForWhite, ResultSource::Tablebase) => {
            stats::TB_BLACK_WINS.fetch_add(1, Ordering::Relaxed)
        }
    };
}

fn datagen_thread(id: usize, games: usize, dir: &str, config: &DatagenConfig) {
    let mut rand = rand::rng();
    let data_file_name = format!("{dir}/data-{id}.txt");
    let mut data_file = std::fs::File::create(&data_file_name).unwrap();
    let mut data_file_writer = BufWriter::new(&mut data_file);

    let mut player_states = PlayerStates::new(16, config);

    for _ in 0..games {
        if STOP.load(Ordering::SeqCst) {
            data_file_writer
                .flush()
                .expect("Should be able to flush data file buffer");

            break;
        }

        let r = play_game(&mut rand, config, &mut player_states);
        update_stats(&r);

        for position in r.positions {
            let fen = position.0;
            let eval = position.1;

            writeln!(
                data_file_writer,
                "{} | {} | {}",
                fen,
                eval.0,
                match r.outcome {
                    Outcome::WinForWhite => "1",
                    Outcome::Draw => "0.5",
                    Outcome::LossForWhite => "0",
                }
            )
            .expect("Failed to write to file");
        }
    }

    data_file_writer
        .flush()
        .expect("Should be able to flush data file buffer");
}

struct GamePosition(pub String, pub WhiteEval);

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum Outcome {
    WinForWhite,
    Draw,
    LossForWhite,
}

#[derive(Eq, PartialEq)]
enum ResultSource {
    Actual,
    Adjudicated,
    Tablebase,
}

struct GameResult {
    positions: Vec<GamePosition>,
    outcome: Outcome,
    source: ResultSource,
}

fn random_starting_position(rand: &mut impl Rng) -> Result<Game, ()> {
    let mut game = Game::new();

    // We want to see games where the first non-random move was made by either player, so we want
    // to sometimes make an extra random move so that black can make the first non-random move.
    let black_starts = usize::from(rand.random::<bool>());

    let number_of_random_moves = DEFAULT_STARTING_MOVES + black_starts;

    for _ in 0..number_of_random_moves {
        let moves = game.moves();
        let random_move = moves.choose(rand);

        // We stumbled into a checkmate or draw
        let Some(random_move) = random_move else {
            return Err(());
        };

        game.make_move(*random_move);
    }

    // The last move we made may have ended the game.
    let moves = game.moves();
    if moves.is_empty() {
        return Err(());
    }

    Ok(game)
}

fn acceptable_starting_position(rand: &mut impl Rng, states: &mut PlayerStates) -> Game {
    const UNBALANCED_STARTING_EVAL: i32 = 1000;

    loop {
        // Skip any games that ended before we got to our starting position
        let Ok(game) = random_starting_position(rand) else {
            continue;
        };

        let state = states.for_player(game.player);

        let (_, eval) = search_position(&game, &TimeControl::Depth(DEFAULT_DEPTH), state);
        if eval.0.abs() >= UNBALANCED_STARTING_EVAL {
            continue;
        }

        return game;
    }
}

fn search_position(
    game: &Game,
    time_control: &TimeControl,
    persistent_state: &mut PersistentState,
) -> (Move, Eval) {
    let options = EngineOptions::default();
    let mut reporter = CapturingReporter::new();

    let best_move = search(
        game,
        persistent_state,
        time_control,
        None,
        &options,
        &mut reporter,
    );

    (best_move, reporter.eval.unwrap())
}

fn game_result(game: &Game, config: &DatagenConfig) -> Option<(Outcome, ResultSource)> {
    if let Some(tb) = &config.tb {
        if let Some(r) = tb.wdl(game) {
            return Some((
                match game.player {
                    Player::White => match r {
                        Wdl::Win => Outcome::WinForWhite,
                        Wdl::Draw => Outcome::Draw,
                        Wdl::Loss => Outcome::LossForWhite,
                    },
                    Player::Black => match r {
                        Wdl::Win => Outcome::LossForWhite,
                        Wdl::Draw => Outcome::Draw,
                        Wdl::Loss => Outcome::WinForWhite,
                    },
                },
                ResultSource::Tablebase,
            ));
        }
    }

    let nmoves = game.moves().len();

    if nmoves == 0 {
        return Some((
            if game.is_king_in_check() {
                match game.player {
                    Player::White => Outcome::LossForWhite,
                    Player::Black => Outcome::WinForWhite,
                }
            } else {
                Outcome::Draw
            },
            ResultSource::Actual,
        ));
    }

    // ?: Is this a problem if we're using 2-repetition?
    if game.is_repeated_position()
        || game.is_stalemate_by_fifty_move_rule()
        || game.is_stalemate_by_insufficient_material()
    {
        return Some((Outcome::Draw, ResultSource::Actual));
    }

    None
}

struct AdjudicationStats {
    white_winning: usize,
    black_winning: usize,
    drawing: usize,
}

impl AdjudicationStats {
    fn new() -> Self {
        Self {
            white_winning: 0,
            black_winning: 0,
            drawing: 0,
        }
    }
}

fn adjudicate_forced_mate(eval: WhiteEval) -> Option<Outcome> {
    let eval_for_white = eval.for_player(Player::White);

    if eval_for_white.mating() {
        return Some(Outcome::WinForWhite);
    }

    if eval_for_white.being_mated() {
        return Some(Outcome::LossForWhite);
    }

    None
}

fn adjudicate_result(
    eval: WhiteEval,
    adjudication_stats: &mut AdjudicationStats,
) -> Option<Outcome> {
    const WHITE_ADJUDICATION_SCORE: WhiteEval = WhiteEval(ADJUDICATION_THRESHOLD);
    const BLACK_ADJUDICATION_SCORE: WhiteEval = WhiteEval(-ADJUDICATION_THRESHOLD);

    if eval > WHITE_ADJUDICATION_SCORE {
        adjudication_stats.white_winning += 1;
        adjudication_stats.black_winning = 0;
        adjudication_stats.drawing = 0;
    } else if eval < BLACK_ADJUDICATION_SCORE {
        adjudication_stats.black_winning += 1;
        adjudication_stats.white_winning = 0;
        adjudication_stats.drawing = 0;
    } else if eval.0.abs() < DRAW_THRESHOLD {
        adjudication_stats.drawing += 1;
        adjudication_stats.white_winning = 0;
        adjudication_stats.black_winning = 0;
    } else {
        adjudication_stats.white_winning = 0;
        adjudication_stats.black_winning = 0;
        adjudication_stats.drawing = 0;
    }

    if adjudication_stats.white_winning > ADJUDICATE_WINS_AFTER {
        return Some(Outcome::WinForWhite);
    }

    if adjudication_stats.black_winning > ADJUDICATE_WINS_AFTER {
        return Some(Outcome::LossForWhite);
    }

    if adjudication_stats.drawing > ADJUDICATE_DRAWS_AFTER {
        return Some(Outcome::Draw);
    }

    None
}

fn should_include_position(game: &Game, mv: Move, eval: Eval) -> bool {
    if game.is_king_in_check() {
        return false;
    }

    if eval.mating() || eval.being_mated() {
        return false;
    }

    // ?: Should we exclude promotions too?
    if mv.is_capture() {
        return false;
    }

    // ?: Should we wait until we're in a more stable position?

    true
}

fn play_game(rand: &mut impl Rng, config: &DatagenConfig, states: &mut PlayerStates) -> GameResult {
    states.reset();
    let mut game = acceptable_starting_position(rand, states);

    let mut positions: Vec<GamePosition> = Vec::new();
    let mut adjudication_stats = AdjudicationStats::new();
    let outcome: Option<Outcome>;
    let source: Option<ResultSource>;

    let time_control = match config.mode {
        DatagenMode::Depth(d) => TimeControl::Depth(d),
    };

    states.reset();

    loop {
        if let Some((r, s)) = game_result(&game, config) {
            outcome = Some(r);
            source = Some(s);
            break;
        }

        let state = states.for_player(game.player);
        let (next_move, eval) = search_position(&game, &time_control, state);
        let white_eval = eval.to_white_eval(game.player);

        if should_include_position(&game, next_move, eval) {
            positions.push(GamePosition(game.to_fen(), white_eval));
        }

        if let Some(r) = adjudicate_forced_mate(white_eval) {
            outcome = Some(r);
            source = Some(ResultSource::Actual);
            break;
        }

        if let Some(r) = adjudicate_result(white_eval, &mut adjudication_stats) {
            outcome = Some(r);
            source = Some(ResultSource::Adjudicated);
            break;
        }

        game.make_move(next_move);
    }

    let outcome = outcome.expect("Game ended without a result");
    let source = source.expect("Unknown result source");

    GameResult {
        positions,
        outcome,
        source,
    }
}

pub fn main() -> ExitCode {
    engine::init();

    let cli = Cli::parse();
    let config = get_config_from_args(&cli);

    ctrlc::set_handler(move || {
        STOP.store(true, Ordering::SeqCst);
        println!("Waiting for the remaining games to finish before exiting");
    })
    .unwrap();

    datagen(&config);

    ExitCode::SUCCESS
}

fn get_config_from_args(args: &Cli) -> DatagenConfig {
    let mode = DatagenMode::Depth(args.depth.unwrap_or(DEFAULT_DEPTH));
    let tb = args.syzygy_path.as_ref().map(|p| load_tablebases(p));

    DatagenConfig {
        games: args.games,
        threads: args.threads,
        mode,
        tb,
    }
}

fn load_tablebases(syzygy_path: &Path) -> Tablebase {
    let mut tb = Tablebase::new();
    tb.set_paths(syzygy_path.to_str().unwrap());

    tb
}
