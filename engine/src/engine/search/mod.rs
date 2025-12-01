mod aspiration;
mod iterative_deepening;
pub mod move_picker;
mod negamax;
mod principal_variation;
mod quiescence;
mod tables;
pub mod time_control;

use std::{cell::RefCell, time::Duration};

use crate::{
    chess::{game::Game, moves::Move},
    engine::{
        eval::{Eval, nnue::NetworkStack},
        options::EngineOptions,
        search::{
            move_picker::MovePicker,
            principal_variation::PrincipalVariation,
            tables::{CountermoveTable, HistoryTable, KillersTable},
            time_control::{StopControl, TimeStrategy},
        },
        tablebases::{Tablebase, Wdl},
        transposition_table::TranspositionTable,
        util,
    },
};

pub const MAX_SEARCH_DEPTH: u8 = u8::MAX;
pub const MAX_SEARCH_DEPTH_SIZE: usize = MAX_SEARCH_DEPTH as usize;

mod params {
    use crate::engine::eval::Eval;

    pub const CHECK_TERMINATION_NODE_FREQUENCY: u64 = 10000;

    pub const ASPIRATION_MIN_DEPTH: u8 = 5;
    pub const ASPIRATION_WINDOW_SIZE: Eval = Eval::new(25);

    pub const NULL_MOVE_PRUNING_BASE_REDUCTION: u8 = 4;
    pub const NULL_MOVE_PRUNING_REDUCTION_FACTOR: u8 = 4;

    pub const FUTILITY_PRUNE_DEPTH: u8 = 1;
    pub const FUTILITY_PRUNE_MAX_MOVE_VALUE: Eval = Eval::new(135);

    pub const SEE_PRUNE_DEPTH: u8 = 10;
    pub const SEE_QUIET_MARGIN: Eval = Eval::new(-30);
    pub const SEE_CAPTURE_MARGIN: Eval = Eval::new(-100);

    pub const REVERSE_FUTILITY_PRUNE_DEPTH: u8 = 4;
    pub const REVERSE_FUTILITY_PRUNE_MARGIN_PER_PLY: Eval = Eval::new(150);

    pub const LMR_DEPTH: u8 = 3;
    pub const LMR_MOVE_THRESHOLD: usize = 3;

    pub const MAX_TIME_PER_MOVE: f32 = 0.5;
    pub const INCREMENT_TO_USE: f32 = 0.5;
    pub const BASE_TIME_PER_MOVE: f32 = 0.033;

    pub const SOFT_TIME_MULTIPLIER: f32 = 0.75;
    pub const HARD_TIME_MULTIPLIER: f32 = 3.00;

    pub const BEST_MOVE_STABILITY_INITIAL_DEPTH: u8 = 5;
    pub const BEST_MOVE_STABILITY_TIME_MULTIPLIERS: [f32; 5] = [2.50, 1.20, 1.00, 0.80, 0.75];
}

pub struct PersistentState {
    pub tt: TranspositionTable,
    pub history_table: HistoryTable,
    pub tablebase: Tablebase,
}

impl PersistentState {
    pub fn new(tt_size_mb: usize) -> Self {
        Self {
            tt: TranspositionTable::new(tt_size_mb),
            history_table: HistoryTable::new(),
            tablebase: Tablebase::new(),
        }
    }

    pub fn with_tablebase(tt_size_mb: usize, tb: &Tablebase) -> Self {
        Self {
            tt: TranspositionTable::new(tt_size_mb),
            history_table: HistoryTable::new(),
            tablebase: tb.clone(),
        }
    }

    pub fn reset(&mut self) {
        self.tt.reset();
        self.history_table = HistoryTable::new();
    }
}

pub(crate) struct SearchContext<'s> {
    pub tt: &'s mut TranspositionTable,
    pub tablebase: &'s mut Tablebase,

    pub history_table: &'s mut HistoryTable,

    pub nnue: &'s mut NetworkStack,

    pub time_control: &'s mut TimeStrategy,

    #[expect(unused, reason = "Not used yet")]
    pub options: &'s EngineOptions,

    pub killer_moves: KillersTable,
    pub countermove_table: CountermoveTable,

    nodes_visited: u64,
    max_depth_reached: u8,
    tbhits: u64,
}

impl<'s> SearchContext<'s> {
    pub(crate) const fn new(
        persistent_state: &'s mut PersistentState,
        nnue: &'s mut NetworkStack,
        time_strategy: &'s mut TimeStrategy,
        options: &'s EngineOptions,
    ) -> Self {
        Self {
            tt: &mut persistent_state.tt,
            tablebase: &mut persistent_state.tablebase,

            history_table: &mut persistent_state.history_table,

            nnue,

            time_control: time_strategy,

            options,

            killer_moves: KillersTable::new(),
            countermove_table: CountermoveTable::new(),

            max_depth_reached: 0,
            nodes_visited: 0,
            tbhits: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimeControl {
    Clocks(Clocks),
    ExactTime(Duration),
    Depth(u8),
    Nodes(u32),
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
    pub eval: Eval,
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

    fn report_search_progress(&self, game: &Game, progress: SearchInfo);

    fn best_move(&self, game: &Game, mv: Move);
}

pub struct NullReporter;

impl Reporter for NullReporter {
    fn generic_report(&self, _: &str) {}

    fn report_search_progress(&self, _: &Game, _: SearchInfo) {}

    fn best_move(&self, _: &Game, _: Move) {}
}

pub struct CapturingReporter {
    eval: RefCell<Option<Eval>>,
    nodes: RefCell<u64>,
}

impl CapturingReporter {
    pub fn new() -> Self {
        Self {
            eval: RefCell::new(None),
            nodes: RefCell::new(0),
        }
    }

    pub fn eval(&self) -> Eval {
        self.eval.borrow().unwrap()
    }

    pub fn nodes(&self) -> u64 {
        *self.nodes.borrow()
    }
}

impl Reporter for CapturingReporter {
    fn generic_report(&self, _: &str) {}

    fn report_search_progress(&self, _: &Game, stats: SearchInfo) {
        *self.eval.borrow_mut() = Some(stats.eval);
        *self.nodes.borrow_mut() = stats.stats.nodes;
    }

    fn best_move(&self, _: &Game, _: Move) {}
}

pub fn search(
    game: &Game,
    persistent_state: &mut PersistentState,
    time_control: &TimeControl,
    stop_control: Option<StopControl>,
    options: &EngineOptions,
    reporter: &impl Reporter,
) -> Move {
    let mut time_strategy = TimeStrategy::new(game, time_control, stop_control, options);
    let mut nnue = NetworkStack::from_board(&game.board);

    let mut ctx = SearchContext::new(persistent_state, &mut nnue, &mut time_strategy, options);

    ctx.tt.new_generation();

    let mut pv = PrincipalVariation::new();

    let tablebase_result = ctx.tablebase.best_move(game);
    if let Some(mv) = tablebase_result {
        let (pv, eval) = get_tablebase_pv(game, &ctx);

        let depth = pv.len();

        reporter.report_search_progress(
            game,
            SearchInfo {
                depth,
                seldepth: depth,
                eval,
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

    move_picker
        .next(game, ctx, 0)
        .unwrap_or_else(|| panic!("No valid moves in position {}", game.to_fen()))
}

fn get_tablebase_pv(game: &Game, ctx: &SearchContext<'_>) -> (PrincipalVariation, Eval) {
    let mut game = game.clone();
    let player = game.player;

    let mut pv = PrincipalVariation::new();

    let tb_score = ctx
        .tablebase
        .wdl(&game)
        .expect("In tablebase position, but unable to get tablebase score");

    let mut eval = None;

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
            eval = Some(if king_in_check {
                let plies = pv.len();

                if game.player == player {
                    Eval::mated_in(plies)
                } else {
                    Eval::mate_in(plies)
                }
            } else {
                Eval::DRAW
            });

            break;
        }
    }

    (
        pv,
        eval.unwrap_or_else(|| match tb_score {
            Wdl::Win => Eval::mate_in(1),
            Wdl::Draw => Eval::DRAW,
            Wdl::Loss => Eval::mated_in(1),
        }),
    )
}
