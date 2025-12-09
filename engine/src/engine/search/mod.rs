mod aspiration;
mod iterative_deepening;
pub mod move_picker;
mod negamax;
mod principal_variation;
mod quiescence;
pub mod tables;
pub mod time_control;

use std::{
    cell::RefCell,
    sync::atomic::AtomicU64,
    thread,
    time::{Duration, Instant},
};

use crate::{
    chess::{game::Game, moves::Move},
    engine::{
        eval::{Eval, nnue::NetworkStack},
        options::EngineOptions,
        search::{
            move_picker::MovePicker,
            principal_variation::PrincipalVariation,
            tables::Tables,
            time_control::{StopControl, TimeStrategy},
        },
        tablebases::{Tablebase, Wdl},
        transposition_table::TranspositionTable,
        util,
        util::buffered_atomic_counter::BufferedAtomicU64,
    },
};

pub const MAX_SEARCH_DEPTH: u8 = u8::MAX;
pub const MAX_SEARCH_DEPTH_SIZE: usize = MAX_SEARCH_DEPTH as usize;

mod params {
    use crate::engine::eval::Eval;

    pub const ASPIRATION_MIN_DEPTH: u8 = 5;
    pub const ASPIRATION_WINDOW_SIZE: Eval = Eval::new(25);

    pub const NULL_MOVE_PRUNING_BASE_REDUCTION: u8 = 4;
    pub const NULL_MOVE_PRUNING_REDUCTION_FACTOR: u8 = 4;

    pub const FUTILITY_PRUNE_DEPTH: u8 = 1;
    pub const FUTILITY_PRUNE_MAX_MOVE_VALUE: Eval = Eval::new(201);

    pub const SEE_PRUNE_DEPTH: u8 = 10;
    pub const SEE_QUIET_MARGIN: Eval = Eval::new(-133);
    pub const SEE_CAPTURE_MARGIN: Eval = Eval::new(-111);

    pub const REVERSE_FUTILITY_PRUNE_DEPTH: u8 = 4;
    pub const REVERSE_FUTILITY_PRUNE_MARGIN_PER_PLY: Eval = Eval::new(40);

    pub const LMR_BASE: f32 = 0.75;
    pub const LMR_FACTOR: f32 = 2.25;
    pub const LMR_DEPTH: u8 = 3;
    pub const LMR_MOVE_THRESHOLD: usize = 3;

    pub const LMP_DEPTH: u8 = 2;
    pub const LMP_MOVE_THRESHOLD: u8 = 5;

    pub const IIR_DEPTH: u8 = 4;

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
    pub tables: Tables,
    pub tablebase: Tablebase,
}

impl PersistentState {
    pub fn new(tt_size_mb: usize) -> Self {
        Self {
            tt: TranspositionTable::new(tt_size_mb),
            tables: Tables::new(),
            tablebase: Tablebase::new(),
        }
    }

    pub fn with_tablebase(tt_size_mb: usize, tb: &Tablebase) -> Self {
        Self {
            tt: TranspositionTable::new(tt_size_mb),
            tables: Tables::new(),
            tablebase: tb.clone(),
        }
    }

    pub fn reset(&mut self) {
        self.tt.reset();
        self.tables = Tables::new();
    }
}

pub(crate) struct SearchContext<'s> {
    pub tt: &'s TranspositionTable,
    pub tables: &'s mut Tables,
    pub tablebase: &'s Tablebase,

    pub stack: SearchStack,
    pub nnue: NetworkStack,

    pub time_control: TimeStrategy,

    #[expect(unused, reason = "Not used yet")]
    pub options: &'s EngineOptions,

    max_depth_reached: u8,
    nodes_visited: BufferedAtomicU64<'s>,
    tbhits: BufferedAtomicU64<'s>,
}

impl<'s> SearchContext<'s> {
    pub fn new(
        game: &Game,
        tt: &'s TranspositionTable,
        tables: &'s mut Tables,
        tablebase: &'s Tablebase,
        node_counter: &'s AtomicU64,
        tbhits_counter: &'s AtomicU64,
        time_control: TimeControl,
        stop_control: StopControl,
        options: &'s EngineOptions,
    ) -> Self {
        Self {
            tt,
            tables,
            tablebase,

            stack: SearchStack::new(),
            nnue: NetworkStack::from_board(&game.board),

            time_control: TimeStrategy::new(game, time_control, stop_control, options),

            options,

            max_depth_reached: 0,
            nodes_visited: BufferedAtomicU64::new(node_counter),
            tbhits: BufferedAtomicU64::new(tbhits_counter),
        }
    }
}

pub struct SearchStack([SearchStackEntry; MAX_SEARCH_DEPTH_SIZE]);

impl SearchStack {
    pub const fn new() -> Self {
        Self([const { SearchStackEntry::new() }; MAX_SEARCH_DEPTH_SIZE])
    }

    pub fn get(&mut self, plies: u8) -> &mut SearchStackEntry {
        &mut self.0[plies as usize]
    }

    pub fn last(&self, plies: u8) -> Option<&SearchStackEntry> {
        self.get_prev(plies, 1)
    }

    pub fn get_prev(&self, plies: u8, i: usize) -> Option<&SearchStackEntry> {
        let plies = plies as usize;

        if i > plies {
            return None;
        }

        Some(&self.0[plies - i])
    }
}

#[derive(Clone)]
pub struct SearchStackEntry {
    mv: Option<Move>,
}

impl SearchStackEntry {
    pub const fn new() -> Self {
        Self { mv: None }
    }
}

#[derive(Debug, Clone)]
pub enum TimeControl {
    Clocks(Clocks),
    ExactTime(Duration),
    Depth(u8),
    Nodes { soft: u64, hard: u64 },
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
    time_control: TimeControl,
    stop_control: StopControl,
    options: &EngineOptions,
    reporter: &impl Reporter,
) -> Move {
    persistent_state.tt.new_generation();
    persistent_state.tables.new_search();

    let tablebase_result = persistent_state.tablebase.best_move(game);
    if let Some(mv) = tablebase_result {
        let start_time = Instant::now();
        let (pv, eval) = get_tablebase_pv(game, &persistent_state.tablebase);
        let elapsed = start_time.elapsed();

        let depth = pv.len();

        reporter.report_search_progress(
            game,
            SearchInfo {
                depth,
                seldepth: depth,
                eval,
                pv,
                hashfull: 0,
                stats: SearchStats {
                    time: elapsed,
                    nodes: u64::from(depth),
                    nodes_per_second: util::metrics::nodes_per_second(u64::from(depth), elapsed),
                    tbhits: 1,
                },
            },
        );

        return mv;
    }

    let threads_stop_control = StopControl::new();
    let global_node_count = AtomicU64::new(0);
    let global_tbhits_count = AtomicU64::new(0);

    thread::scope(|scope| {
        let mut threads = Vec::new();

        // If we want more than one thread, spawn our other threads, sharing the transposition
        // table but with their own copy of everything else.
        for _ in 1..options.threads {
            let tt = &persistent_state.tt;
            let tablebase = &persistent_state.tablebase;
            let this_thread_stop_control = threads_stop_control.clone();
            let global_node_count = &global_node_count;
            let global_tbhits_count = &global_tbhits_count;
            let mut thread_tables = persistent_state.tables.clone();

            let thread = scope.spawn(move || {
                let mut ctx = SearchContext::new(
                    game,
                    tt,
                    &mut thread_tables,
                    tablebase,
                    global_node_count,
                    global_tbhits_count,
                    TimeControl::Infinite,
                    this_thread_stop_control,
                    options,
                );

                iterative_deepening::search(
                    &mut game.clone(),
                    &mut ctx,
                    &mut PrincipalVariation::new(),
                    &NullReporter,
                );
            });

            threads.push(thread);
        }

        let mut ctx = SearchContext::new(
            game,
            &persistent_state.tt,
            &mut persistent_state.tables,
            &persistent_state.tablebase,
            &global_node_count,
            &global_tbhits_count,
            time_control,
            stop_control,
            options,
        );

        let mut pv = PrincipalVariation::new();

        iterative_deepening::search(
            // Give the search its own copy of the game so we don't get one returned in a dirty state
            // when the search aborts.
            &mut game.clone(),
            &mut ctx,
            &mut pv,
            reporter,
        );

        let best_move = pv.first().copied();

        threads_stop_control.stop();
        for thread in threads {
            thread.join().expect("Thread panicked");
        }

        best_move.unwrap_or_else(|| panic_move(game, &ctx))
    })
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
        .next(game, ctx.tables, 0)
        .unwrap_or_else(|| panic!("No valid moves in position {}", game.to_fen()))
}

fn get_tablebase_pv(game: &Game, tb: &Tablebase) -> (PrincipalVariation, Eval) {
    let mut game = game.clone();
    let player = game.player;

    let mut pv = PrincipalVariation::new();

    let tb_score = tb
        .wdl(&game)
        .expect("In tablebase position, but unable to get tablebase score");

    let mut eval = None;

    for _ in 0..MAX_SEARCH_DEPTH {
        let tablebase_move = tb
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
