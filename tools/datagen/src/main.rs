use std::{
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::ExitCode,
    sync::atomic::{AtomicBool, Ordering},
};

use clap::Parser;
use engine::{
    chess::{
        game::Game,
        moves::Move,
        piece::PromotionPieceKind,
        player::Player,
        square::{Square, squares},
    },
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
    pub static POSITIONS: AtomicU64 = AtomicU64::new(0);
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

        let positions_generated = stats::POSITIONS.load(Ordering::SeqCst);
        let elapsed_time = jiff::Timestamp::now() - start_time;
        let elapsed_seconds = elapsed_time.total(Unit::Second).unwrap();
        let positions_per_second =
            f64::from(u32::try_from(positions_generated).unwrap()) / elapsed_seconds;
        let positions_per_game = positions_generated as f64 / games_played as f64;

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
            "{positions_generated} positions generated [{positions_per_second:.2}/s] in {elapsed_time:#} from {games_played} games (out of {ngames}) | {approx_time_remaining:#} remaining"
        );

        println!(
            "Avg time per game: {approx_time_per_game:.2}s | Avg positions per game: {positions_per_game:.2} | W: {total_white_wins} B: {total_black_wins} D: {total_draws}"
        );

        println!(
            "White: ({white_wins}/{adjudicated_white_wins}/{tb_white_wins}) | Black: ({black_wins}/{adjudicated_black_wins}/{tb_black_wins}) | Draws: ({draws}/{adjudicated_draws}/{tb_draws})"
        );
        println!();

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

fn update_stats(
    game: &viriformat::dataformat::Game,
    outcome: viriformat::chess::board::GameOutcome,
) {
    use viriformat::chess::board::{DrawType, GameOutcome::*, WinType};

    stats::GAMES.fetch_add(1, Ordering::SeqCst);
    stats::POSITIONS.fetch_add(game.moves.len() as u64, Ordering::SeqCst);

    match outcome {
        WhiteWin(ty) => match ty {
            WinType::Mate => {
                stats::WHITE_WINS.fetch_add(1, Ordering::Relaxed);
            }
            WinType::TB => {
                stats::TB_WHITE_WINS.fetch_add(1, Ordering::Relaxed);
            }
            WinType::Adjudication => {
                stats::ADJUDICATED_WHITE_WINS.fetch_add(1, Ordering::Relaxed);
            }
        },
        BlackWin(ty) => match ty {
            WinType::Mate => {
                stats::BLACK_WINS.fetch_add(1, Ordering::Relaxed);
            }
            WinType::TB => {
                stats::TB_BLACK_WINS.fetch_add(1, Ordering::Relaxed);
            }
            WinType::Adjudication => {
                stats::ADJUDICATED_BLACK_WINS.fetch_add(1, Ordering::Relaxed);
            }
        },
        Draw(ty) => match ty {
            DrawType::TB => {
                stats::TB_DRAWS.fetch_add(1, Ordering::Relaxed);
            }
            DrawType::FiftyMoves
            | DrawType::Repetition
            | DrawType::Stalemate
            | DrawType::InsufficientMaterial => {
                stats::DRAWS.fetch_add(1, Ordering::Relaxed);
            }
            DrawType::Adjudication => {
                stats::ADJUDICATED_DRAWS.fetch_add(1, Ordering::Relaxed);
            }
        },
        Ongoing => {
            unreachable!("ongoing is not used in datagen");
        }
    }
}

fn datagen_thread(id: usize, games: usize, dir: &str, config: &DatagenConfig) {
    let mut rand = rand::rng();
    let data_file_name = format!("{dir}/data-{id}.bin");
    let data_file = std::fs::File::create(&data_file_name).unwrap();
    let mut buffer = BufWriter::new(&data_file);

    let mut player_states = PlayerStates::new(16, config);

    for _ in 0..games {
        if STOP.load(Ordering::SeqCst) {
            buffer
                .flush()
                .expect("Should be able to flush data file buffer");

            break;
        }

        let (game, result_source) = play_game(&mut rand, config, &mut player_states);
        update_stats(&game, result_source);
        game.serialise_into(&mut buffer)
            .expect("Should serialize into data file");
    }

    buffer
        .flush()
        .expect("Should be able to flush data file buffer");
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

fn game_result(
    game: &Game,
    config: &DatagenConfig,
) -> Option<viriformat::chess::board::GameOutcome> {
    use viriformat::chess::board::{DrawType, GameOutcome::*, WinType};

    if let Some(tb) = &config.tb {
        if let Some(r) = tb.wdl(game) {
            return Some(match game.player {
                Player::White => match r {
                    Wdl::Win => WhiteWin(WinType::TB),
                    Wdl::Draw => Draw(DrawType::TB),
                    Wdl::Loss => BlackWin(WinType::TB),
                },
                Player::Black => match r {
                    Wdl::Win => BlackWin(WinType::TB),
                    Wdl::Draw => Draw(DrawType::TB),
                    Wdl::Loss => WhiteWin(WinType::TB),
                },
            });
        }
    }

    let nmoves = game.moves().len();

    if nmoves == 0 {
        return Some(if game.is_king_in_check() {
            match game.player {
                Player::White => BlackWin(WinType::Mate),
                Player::Black => WhiteWin(WinType::Mate),
            }
        } else {
            Draw(DrawType::Stalemate)
        });
    }

    if game.is_repeated_position() {
        return Some(Draw(DrawType::Repetition));
    }

    if game.is_stalemate_by_fifty_move_rule() {
        return Some(Draw(DrawType::FiftyMoves));
    }

    if game.is_stalemate_by_insufficient_material() {
        return Some(Draw(DrawType::InsufficientMaterial));
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

fn adjudicate_forced_mate(eval: WhiteEval) -> Option<viriformat::chess::board::GameOutcome> {
    use viriformat::chess::board::{GameOutcome::*, WinType};

    let eval_for_white = eval.for_player(Player::White);

    if eval_for_white.mating() {
        return Some(WhiteWin(WinType::Mate));
    }

    if eval_for_white.being_mated() {
        return Some(BlackWin(WinType::Mate));
    }

    None
}

fn adjudicate_result(
    eval: WhiteEval,
    adjudication_stats: &mut AdjudicationStats,
) -> Option<viriformat::chess::board::GameOutcome> {
    use viriformat::chess::board::{DrawType, GameOutcome::*, WinType};

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
        return Some(WhiteWin(WinType::Adjudication));
    }

    if adjudication_stats.black_winning > ADJUDICATE_WINS_AFTER {
        return Some(BlackWin(WinType::Adjudication));
    }

    if adjudication_stats.drawing > ADJUDICATE_DRAWS_AFTER {
        return Some(Draw(DrawType::Adjudication));
    }

    None
}

fn game_to_viri(game: &Game) -> viriformat::dataformat::Game {
    let mut board = viriformat::chess::board::Board::new();
    board
        .set_from_fen(&game.to_fen())
        .expect("Should be able to construct game from FEN");
    viriformat::dataformat::Game::new(&board)
}

fn move_to_viri(mv: Move) -> viriformat::chess::chessmove::Move {
    use squares::all::*;
    use viriformat::chess::chessmove::MoveFlags;

    if let Some(promo_piece) = mv.promotion() {
        viriformat::chess::chessmove::Move::new_with_promo(
            square_to_viri(mv.src()),
            square_to_viri(mv.dst()),
            piece_to_viri(promo_piece),
        )
    } else if mv.is_castling() {
        let to_sq = match mv.dst() {
            G1 => H1,
            G8 => H8,
            C1 => A1,
            C8 => A8,
            _ => unreachable!("invalid castle square"),
        };

        viriformat::chess::chessmove::Move::new_with_flags(
            square_to_viri(mv.src()),
            square_to_viri(to_sq),
            MoveFlags::Castle,
        )
    } else if mv.is_en_passant() {
        viriformat::chess::chessmove::Move::new_with_flags(
            square_to_viri(mv.src()),
            square_to_viri(mv.dst()),
            MoveFlags::EnPassant,
        )
    } else {
        viriformat::chess::chessmove::Move::new(square_to_viri(mv.src()), square_to_viri(mv.dst()))
    }
}

fn square_to_viri(sq: Square) -> viriformat::chess::types::Square {
    viriformat::chess::types::Square::new(sq.idx()).expect("Should be a valid square")
}

fn piece_to_viri(piece: PromotionPieceKind) -> viriformat::chess::piece::PieceType {
    use viriformat::chess::piece::PieceType::*;

    match piece {
        PromotionPieceKind::Knight => Knight,
        PromotionPieceKind::Bishop => Bishop,
        PromotionPieceKind::Rook => Rook,
        PromotionPieceKind::Queen => Queen,
    }
}

fn play_game(
    rand: &mut impl Rng,
    config: &DatagenConfig,
    states: &mut PlayerStates,
) -> (
    viriformat::dataformat::Game,
    viriformat::chess::board::GameOutcome,
) {
    states.reset();
    let mut game = acceptable_starting_position(rand, states);
    let mut virigame = game_to_viri(&game);

    let mut adjudication_stats = AdjudicationStats::new();

    let time_control = match config.mode {
        DatagenMode::Depth(d) => TimeControl::Depth(d),
    };

    states.reset();

    let outcome = loop {
        if let Some(outcome) = game_result(&game, config) {
            virigame.set_outcome(outcome);
            break outcome;
        }

        let state = states.for_player(game.player);
        let (next_move, eval) = search_position(&game, &time_control, state);
        let white_eval = eval.to_white_eval(game.player);

        virigame.add_move(
            move_to_viri(next_move),
            i16::try_from(white_eval.0).unwrap(),
        );
        game.make_move(next_move);

        if let Some(outcome) = adjudicate_forced_mate(white_eval) {
            break outcome;
        }

        if let Some(outcome) = adjudicate_result(white_eval, &mut adjudication_stats) {
            break outcome;
        }
    };

    (virigame, outcome)
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
