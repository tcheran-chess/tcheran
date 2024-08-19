
use crate::chess::game::Game;
use crate::chess::moves::Move;
use crate::engine::eval::Eval;
use crate::engine::options::EngineOptions;
use crate::engine::search::move_picker::MovePicker;
use crate::engine::search::principal_variation::PrincipalVariation;
use crate::engine::search::tables::{CountermoveTable, HistoryTable, KillersTable};
use crate::engine::search::time_control::TimeStrategy;
use crate::engine::search::transposition::SearchTranspositionTable;
use crate::engine::tablebases::{Tablebase, Wdl};
use crate::engine::util;
use std::time::Duration;

mod aspiration;
mod iterative_deepening;
mod move_ordering;
pub mod move_picker;
mod negamax;
mod principal_variation;
mod quiescence;
mod tables;
pub mod time_control;
pub mod transposition;

const MAX_SEARCH_DEPTH: u8 = u8::MAX;
const MAX_SEARCH_DEPTH_SIZE: usize = MAX_SEARCH_DEPTH as usize;

mod params {
    use crate::engine::eval::Eval;

    pub const CHECK_TERMINATION_NODE_FREQUENCY: u64 = 10000;

    pub const ASPIRATION_MIN_DEPTH: u8 = 5;
    pub const ASPIRATION_WINDOW_SIZE: Eval = Eval::new(25);

    pub const NULL_MOVE_PRUNING_DEPTH_LIMIT: u8 = 3;
    pub const NULL_MOVE_PRUNING_DEPTH_REDUCTION: u8 = 2;

    pub const FUTILITY_PRUNE_DEPTH: u8 = 1;
    pub const FUTILITY_PRUNE_MAX_MOVE_VALUE: Eval = Eval::new(135);

    pub const REVERSE_FUTILITY_PRUNE_DEPTH: u8 = 4;
    pub const REVERSE_FUTILITY_PRUNE_MARGIN_PER_PLY: Eval = Eval::new(150);

    pub const LMR_DEPTH: u8 = 3;
    pub const LMR_MOVE_THRESHOLD: usize = 3;

    pub const HISTORY_DECAY_FACTOR: i32 = 8;

    pub const MAX_TIME_PER_MOVE: f32 = 0.5;
    pub const INCREMENT_TO_USE: f32 = 0.5;
    pub const BASE_TIME_PER_MOVE: f32 = 0.033;

    pub const SOFT_TIME_MULTIPLIER: f32 = 0.75;
    pub const HARD_TIME_MULTIPLIER: f32 = 3.00;

    pub const BEST_MOVE_STABILITY_INITIAL_DEPTH: u8 = 5;
    pub const BEST_MOVE_STABILITY_TIME_MULTIPLIERS: [f32; 5] = [2.50, 1.20, 1.00, 0.80, 0.75];
}

pub struct PersistentState {
    pub tt: SearchTranspositionTable,
    pub history_table: HistoryTable,
    pub tablebase: Tablebase,
}

impl PersistentState {
    pub fn new(tt_size_mb: usize) -> Self {
        Self {
            tt: SearchTranspositionTable::new(tt_size_mb),
            history_table: HistoryTable::new(),
            tablebase: Tablebase::new(),
        }
    }

    pub fn reset(&mut self) {
        self.tt.reset();
        self.history_table.reset();
    }
}

pub struct SearchContext<'s> {
    pub tt: &'s mut SearchTranspositionTable,
    pub tablebase: &'s mut Tablebase,

    pub history_table: &'s mut HistoryTable,

    pub time_control: &'s mut TimeStrategy,

    pub options: &'s EngineOptions,
    pub search_restrictions: &'s SearchRestrictions,

    pub killer_moves: KillersTable,
    pub countermove_table: CountermoveTable,

    nodes_visited: u64,
    max_depth_reached: u8,
    tbhits: u64,
}

impl<'s> SearchContext<'s> {
    pub const fn new(
        persistent_state: &'s mut PersistentState,
        time_strategy: &'s mut TimeStrategy,
        options: &'s EngineOptions,
        search_restrictions: &'s SearchRestrictions,
    ) -> Self {
        Self {
            tt: &mut persistent_state.tt,
            tablebase: &mut persistent_state.tablebase,

            history_table: &mut persistent_state.history_table,

            time_control: time_strategy,

            options,
            search_restrictions,

            killer_moves: KillersTable::new(),
            countermove_table: CountermoveTable::new(),

            max_depth_reached: 0,
            nodes_visited: 0,
            tbhits: 0,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SearchScore {
    Centipawns(i16),
    Mate(i16),
}

#[derive(Default)]
pub struct SearchRestrictions {
    pub depth: Option<u8>,
}

#[derive(Debug, Clone)]
pub enum TimeControl {
    Clocks(Clocks),
    ExactTime(Duration),
    Infinite,
}

#[derive(Debug, Clone)]
pub struct Clocks {
    pub white_clock: Option<Duration>,
    pub black_clock: Option<Duration>,
    pub white_increment: Option<Duration>,
    pub black_increment: Option<Duration>,
    pub moves_to_go: Option<u32>,
}

pub struct SearchInfo {
    pub depth: u8,
    pub seldepth: u8,
    pub score: SearchScore,
    pub stats: SearchStats,
    pub pv: PrincipalVariation,
    pub hashfull: usize,
}

pub struct SearchStats {
    pub time: Duration,
    pub nodes: u64,
    pub nodes_per_second: u64,
    pub tbhits: u64,
}

pub trait Reporter {
    fn generic_report(&self, s: &str);

    fn report_search_progress(&mut self, game: &Game, progress: SearchInfo);

    fn best_move(&self, game: &Game, mv: Move);
}

pub struct NullReporter;

impl Reporter for NullReporter {
    fn generic_report(&self, _: &str) {}

    fn report_search_progress(&mut self, _: &Game, _: SearchInfo) {}

    fn best_move(&self, _: &Game, _: Move) {}
}

pub struct CapturingReporter {
    pub score: Option<SearchScore>,
    pub nodes: u64,
}

impl CapturingReporter {
    pub fn new() -> Self {
        Self {
            score: None,
            nodes: 0,
        }
    }
}

impl Reporter for CapturingReporter {
    fn generic_report(&self, _: &str) {}

    fn report_search_progress(&mut self, _: &Game, stats: SearchInfo) {
        self.score = Some(stats.score);
        self.nodes = stats.stats.nodes;
    }

    fn best_move(&self, _: &Game, _: Move) {}
}

pub fn search(
    game: &Game,
    persistent_state: &mut PersistentState,
    time_strategy: &mut TimeStrategy,
    search_restrictions: &SearchRestrictions,
    options: &EngineOptions,
    reporter: &mut impl Reporter,
) -> Move {
    let mut ctx = SearchContext::new(
        persistent_state,
        time_strategy,
        options,
        search_restrictions,
    );

    ctx.tt.new_generation();
    ctx.history_table.decay(params::HISTORY_DECAY_FACTOR);

    let mut pv = PrincipalVariation::new();

    let tablebase_result = ctx.tablebase.best_move(game);
    if let Some(mv) = tablebase_result {
        let (pv, score) = get_tablebase_pv(game, &ctx);

        let depth = pv.len();

        reporter.report_search_progress(
            game,
            SearchInfo {
                depth,
                seldepth: depth,
                score,
                pv,
                hashfull: persistent_state.tt.occupancy(),
                stats: SearchStats {
                    time: time_strategy.elapsed(),
                    nodes: u64::from(depth),
                    nodes_per_second: util::metrics::nodes_per_second(
                        u64::from(depth),
                        time_strategy.elapsed(),
                    ),
                    tbhits: 1,
                },
            },
        );

        return mv;
    }

    iterative_deepening::search(
        // Give the search its own copy of the game so we don't get one returned in a dirty state
        // when the search aborts.
        &mut game.clone(),
        &mut ctx,
        &mut pv,
        reporter,
    );

    let best_move = pv.first().copied();

    best_move.unwrap_or_else(|| panic_move(game, &ctx))
}

pub fn init() {
    tables::init();
}

// If we have so little time to search that we couldn't determine a best move, we'll need to spend
// a bit of extra time so that we still make a move.
// Rather than returning a random move, we return the first move that is returned after move ordering
fn panic_move(game: &Game, ctx: &SearchContext<'_>) -> Move {
    let mut move_picker = MovePicker::new(None);

    move_picker.next(game, ctx, 0).unwrap()
}

fn get_tablebase_pv(game: &Game, ctx: &SearchContext<'_>) -> (PrincipalVariation, SearchScore) {
    let mut game = game.clone();
    let player = game.player;

    let mut pv = PrincipalVariation::new();

    let tb_score = ctx
        .tablebase
        .wdl(&game)
        .expect("In tablebase position, but unable to get tablebase score");

    let mut search_score = None;

    for _ in 0..MAX_SEARCH_DEPTH {
        let tablebase_move = ctx
            .tablebase
            .best_move(&game)
            .expect("In tablebase position, but unable to get tablebase move");

        pv.append(tablebase_move);

        game.make_move(tablebase_move);

        // Check if this move terminated the game, and return an appropriate score
        let legal_moves = game.moves();
        let king_in_check = game.is_king_in_check();

        if legal_moves.is_empty() {
            search_score = Some(if king_in_check {
                let plies = pv.len();

                let mate_score = if game.player == player {
                    Eval::mated_in(plies)
                } else {
                    Eval::mate_in(plies)
                };

                let mate_in_moves = mate_score.is_mate_in_moves().unwrap();

                SearchScore::Mate(mate_in_moves)
            } else {
                SearchScore::Centipawns(0)
            });

            break;
        }
    }

    (
        pv,
        search_score.unwrap_or(match tb_score {
            Wdl::Win => SearchScore::Mate(1),
            Wdl::Draw => SearchScore::Centipawns(0),
            Wdl::Loss => SearchScore::Mate(-1),
        }),
    )
}
