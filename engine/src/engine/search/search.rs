use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use crate::{
    chess::{Game, Move, Piece, Player},
    engine::{
        eval::{Eval, nnue::NetworkStack},
        options::EngineOptions,
        search::{
            iterative_deepening,
            principal_variation::PrincipalVariation,
            tables::{KillersTable, Tables},
            time_control::{StopControl, TimeStrategy},
            types::Depth,
        },
        tablebases::{Tablebase, Wdl},
        transposition_table::TranspositionTable,
        util,
        util::buffered_atomic_counter::BufferedAtomicU64,
    },
};

pub const MAX_SEARCH_DEPTH: u8 = u8::MAX - 1;

// Size used for arrays that are indexed by ply. Add one extra above MAX_PLIES_SIZE for safety
// when we perform operations on 'the next ply'.
pub const MAX_PLIES_ARRAY_SIZE: usize = MAX_SEARCH_DEPTH as usize + 1;

pub struct PersistentState {
    pub tt: TranspositionTable,
    pub tablebase: Tablebase,

    pub node_counter: AtomicU64,
    pub tbhits_counter: AtomicU64,
}

impl PersistentState {
    pub fn new(tt_size_mb: usize) -> Self {
        Self::with_tablebase(tt_size_mb, &Tablebase::new())
    }

    pub fn with_tablebase(tt_size_mb: usize, tb: &Tablebase) -> Self {
        Self {
            tt: TranspositionTable::new(tt_size_mb),
            tablebase: tb.clone(),

            node_counter: AtomicU64::default(),
            tbhits_counter: AtomicU64::default(),
        }
    }

    pub fn new_search(&self) {
        self.tt.new_generation();
        self.reset_counters();
    }

    pub fn reset(&self) {
        self.tt.reset();
        self.reset_counters();
    }

    fn reset_counters(&self) {
        self.node_counter.store(0, Ordering::Relaxed);
        self.tbhits_counter.store(0, Ordering::Relaxed);
    }
}

pub struct ThreadData {
    pub id: usize,
    pub tables: Tables,
    pub nnue: NetworkStack,
    pub stack: SearchStack,
}

impl ThreadData {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            tables: Tables::new(),
            nnue: NetworkStack::new(),
            stack: SearchStack::new(),
        }
    }

    pub fn new_search(&mut self, game: &Game) {
        self.tables.killer_moves = KillersTable::new();
        self.nnue.setup(&game.board);
        self.stack = SearchStack::new();
    }

    pub fn reset(&mut self) {
        self.tables = Tables::new();
        self.nnue = NetworkStack::new();
        self.stack = SearchStack::new();
    }

    pub fn mut_refs(&mut self) -> (&mut Tables, &mut SearchStack, &mut NetworkStack) {
        (&mut self.tables, &mut self.stack, &mut self.nnue)
    }
}

#[repr(align(64))]
pub struct SearchContext<'s> {
    pub tt: &'s TranspositionTable,
    pub tablebase: &'s Tablebase,
    pub options: &'s EngineOptions,

    pub tables: &'s mut Tables,
    pub nnue: &'s mut NetworkStack,
    pub stack: &'s mut SearchStack,

    pub time_control: TimeStrategy,

    pub max_depth_reached: u8,
    pub completed_depth: Depth,
    pub root_depth: Depth,
    pub was_hard_stopped: bool,
    pub nodes: BufferedAtomicU64<'s>,
    pub tbhits: BufferedAtomicU64<'s>,
}

impl<'s> SearchContext<'s> {
    pub fn new(
        game: &Game,
        tt: &'s TranspositionTable,
        tablebase: &'s Tablebase,
        node_counter: &'s AtomicU64,
        tbhits_counter: &'s AtomicU64,
        tables: &'s mut Tables,
        search_stack: &'s mut SearchStack,
        nnue: &'s mut NetworkStack,
        time_control: TimeControl,
        stop_control: StopControl,
        options: &'s EngineOptions,
    ) -> Self {
        Self {
            tt,
            tablebase,
            options,

            tables,
            stack: search_stack,
            nnue,
            time_control: TimeStrategy::new(game, time_control, stop_control, options),

            max_depth_reached: 0,
            completed_depth: Depth::ZERO,
            root_depth: Depth::ZERO,
            was_hard_stopped: false,
            nodes: BufferedAtomicU64::new(node_counter),
            tbhits: BufferedAtomicU64::new(tbhits_counter),
        }
    }
}

#[repr(align(64))]
pub struct SearchStack([SearchStackEntry; MAX_PLIES_ARRAY_SIZE]);

impl SearchStack {
    pub const fn new() -> Self {
        Self([const { SearchStackEntry::new() }; MAX_PLIES_ARRAY_SIZE])
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

pub struct SearchStackEntry {
    pub mv: Option<(Move, Piece)>,
    pub eval: Eval,

    pub excluded_mv: Option<Move>,
    pub double_extensions: u8,
    pub fail_highs: u8,
}

impl SearchStackEntry {
    pub const fn new() -> Self {
        Self {
            mv: None,
            eval: Eval::MIN,

            excluded_mv: None,
            double_extensions: 0,
            fail_highs: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimeControl {
    Clocks {
        clocks: Clocks,
        start_time: Instant,
    },
    ExactTime {
        time: Duration,
        start_time: Instant,
    },
    Depth(Depth),
    Nodes {
        soft: Option<u64>,
        hard: Option<u64>,
    },
    Infinite,
}

#[derive(Debug, Clone)]
pub struct Clocks {
    pub clocks: [Option<Duration>; Player::N],
    pub increments: [Option<Duration>; Player::N],
    pub moves_to_go: Option<u32>,
}

impl Clocks {
    pub fn for_player(&self, player: Player) -> (Duration, Duration) {
        (self.clocks[player].unwrap_or_default(), self.increments[player].unwrap_or_default())
    }
}

pub struct SearchResult {
    pub best_move: Move,
    pub eval: Eval,
    pub pv: PrincipalVariation,
}

pub struct SearchInfo<'s> {
    pub game: &'s Game,
    pub eval: Eval,
    pub pv: PrincipalVariation,
    pub stats: SearchStats,
}

pub struct SearchStats {
    pub time: Duration,
    pub depth: u8,
    pub seldepth: u8,
    pub nodes: u64,
    pub nodes_per_second: u64,
    pub tbhits: u64,
    pub hashfull: u64,
}

impl SearchStats {
    pub fn from_ctx(ctx: &SearchContext<'_>) -> Self {
        Self {
            time: ctx.time_control.elapsed(),
            depth: ctx.completed_depth.as_u8(),
            seldepth: ctx.max_depth_reached,
            nodes: ctx.nodes.get_global(),
            nodes_per_second: util::metrics::nodes_per_second(
                ctx.nodes.get_global(),
                ctx.time_control.elapsed(),
            ),
            tbhits: ctx.tbhits.get_global(),
            hashfull: ctx.tt.occupancy(),
        }
    }
}

pub trait Reporter {
    fn generic_report(&self, s: &str);

    fn report_search_progress(&self, progress: SearchInfo<'_>);

    fn best_move(&self, game: &Game, mv: Move);
}

pub struct NullReporter;

impl Reporter for NullReporter {
    fn generic_report(&self, _: &str) {}

    fn report_search_progress(&self, _: SearchInfo<'_>) {}

    fn best_move(&self, _: &Game, _: Move) {}
}

pub fn search(
    game: &Game,
    persistent_state: &PersistentState,
    thread_data: &mut ThreadData,
    time_control: TimeControl,
    stop_control: &StopControl,
    options: &EngineOptions,
    reporter: &dyn Reporter,
) -> (Move, Eval) {
    let is_main_thread = thread_data.id == 0;

    let (tables, stack, nnue) = thread_data.mut_refs();
    let mut ctx = SearchContext::new(
        game,
        &persistent_state.tt,
        &persistent_state.tablebase,
        &persistent_state.node_counter,
        &persistent_state.tbhits_counter,
        tables,
        stack,
        nnue,
        time_control,
        stop_control.clone(),
        options,
    );

    let result = iterative_deepening::search(
        // Give the search its own copy of the game so we don't get one returned in a dirty state
        // when the search aborts.
        &mut game.clone(),
        &mut ctx,
        reporter,
    );

    if is_main_thread {
        // If we're the main thread, signal for all the other threads to stop and then wait until
        // they do.
        stop_control.stop();
        stop_control.wait_until_last();
    }

    stop_control.stopped();

    // Always send a final info line before reporting the best move so we have useful information
    // such as the exact number of nodes searched and the exact time used. This could be useful for
    // debugging time issues or reproducing a bug by playing exact nodes.
    // See https://github.com/AndyGrant/Ethereal/issues/214
    if ctx.was_hard_stopped || ctx.options.minimal {
        reporter.report_search_progress(SearchInfo {
            game,
            pv: result.pv.clone(),
            eval: result.eval,
            stats: SearchStats::from_ctx(&ctx),
        });
    }

    reporter.best_move(game, result.best_move);

    (result.best_move, result.eval)
}

// Simple single-threaded search used by utilities like bench, tests and datagen
pub fn st_search(
    game: &Game,
    persistent_state: &PersistentState,
    time_control: TimeControl,
    reporter: &dyn Reporter,
) -> (Move, Eval) {
    search(
        game,
        persistent_state,
        // Non-main thread ID so that we don't wait to finish
        &mut ThreadData::new(1),
        time_control,
        &StopControl::new(0),
        &EngineOptions::DEFAULT,
        reporter,
    )
}

pub fn probe_tb_at_root(
    game: &Game,
    tb: &Tablebase,
    time_control: &TimeControl,
) -> Option<(Move, Eval, PrincipalVariation, Duration, u8)> {
    let mut game = game.clone();
    let best_move = tb.best_move(&game)?;

    let start_time = match time_control {
        TimeControl::Clocks { start_time, .. } | TimeControl::ExactTime { start_time, .. } => {
            *start_time
        }
        _ => Instant::now(),
    };

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
        let king_in_check = game.in_check();

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

    let elapsed = start_time.elapsed();
    let depth = pv.len();
    let eval = eval.unwrap_or_else(|| match tb_score {
        Wdl::Win => Eval::tb_mate_in(MAX_SEARCH_DEPTH),
        Wdl::Draw => Eval::DRAW,
        Wdl::Loss => Eval::tb_mated_in(MAX_SEARCH_DEPTH),
    });

    Some((best_move, eval, pv, elapsed, depth))
}
