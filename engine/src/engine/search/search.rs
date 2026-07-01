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
            types::{Depth, SearchResults},
        },
        tablebases::{Tablebase, Wdl},
        transposition_table::TranspositionTable,
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

    pub fn reset(&mut self, options: &EngineOptions) {
        self.tt.reset(options.threads);
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

    time_control: TimeStrategy,

    pub id: usize,
    pub max_depth_reached: u8,
    pub completed_depth: Depth,
    pub root_depth: Depth,
    pub was_hard_stopped: bool,
    pub nodes: BufferedAtomicU64<'s>,
    pub tbhits: BufferedAtomicU64<'s>,
}

impl<'s> SearchContext<'s> {
    pub fn new(
        id: usize,
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

            id,
            max_depth_reached: 0,
            completed_depth: Depth::ZERO,
            root_depth: Depth::ZERO,
            was_hard_stopped: false,
            nodes: BufferedAtomicU64::new(node_counter),
            tbhits: BufferedAtomicU64::new(tbhits_counter),
        }
    }
}

impl SearchContext<'_> {
    pub fn should_start_new_search(&mut self, depth: Depth) -> bool {
        self.time_control
            .should_start_new_search(self.nodes.get(), depth)
    }

    pub fn stopped(&mut self) -> bool {
        self.time_control.stopped(self.nodes.get(), self.root_depth)
    }

    pub fn update_nodes_used(&mut self, mv: Move, nodes: u64) {
        self.time_control.update_nodes_used(mv, nodes);
    }

    pub fn update_after_search(&mut self, best_move: Move, depth: Depth, nodes: u64) {
        self.completed_depth = depth;
        self.time_control
            .update_after_search(best_move, depth, nodes);
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

#[derive(Clone)]
pub struct SearchResult {
    pub id: usize,
    pub mv: Move,
    pub score: Eval,
    pub pv: PrincipalVariation,
    pub stats: SearchStats,
}

#[derive(Clone)]
pub struct SearchStats {
    pub time: Duration,
    pub depth: u8,
    pub seldepth: u8,
    pub nodes: u64,
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
            tbhits: ctx.tbhits.get_global(),
            hashfull: ctx.tt.occupancy(),
        }
    }
}

pub trait Reporter {
    fn generic_report(&self, s: &str);

    fn report_search_progress(&self, game: &Game, result: &SearchResult);

    fn best_move(&self, game: &Game, mv: Move);
}

pub struct NullReporter;

impl Reporter for NullReporter {
    fn generic_report(&self, _: &str) {}

    fn report_search_progress(&self, _: &Game, _: &SearchResult) {}

    fn best_move(&self, _: &Game, _: Move) {}
}

pub fn search(
    game: &Game,
    persistent_state: &PersistentState,
    thread_data: &mut ThreadData,
    results: &SearchResults,
    time_control: TimeControl,
    stop_control: &StopControl,
    options: &EngineOptions,
    reporter: &dyn Reporter,
) -> SearchResult {
    let thread_id = thread_data.id;
    let is_main_thread = thread_id == 0;

    let (tables, stack, nnue) = thread_data.mut_refs();
    let mut ctx = SearchContext::new(
        thread_id,
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

    let thread_result = iterative_deepening::search(
        // Give the search its own copy of the game so we don't get one returned in a dirty state
        // when the search aborts.
        &mut game.clone(),
        &mut ctx,
        reporter,
    );

    results.set(thread_id, &thread_result);

    if is_main_thread {
        // If we're the main thread, signal for all the other threads to stop and then wait until
        // they do.
        stop_control.stop();
        stop_control.wait_until_last();

        let result = best_result(results);

        let send_final_info =
             // We picked a different thread, so we want to report *that* thread's info
             result.id != thread_id
             // We did more searching since last reporting, always send a final info line
             // before reporting the best move so we have useful information such as the exact number of
             // nodes searched and the exact time used. This could be useful for debugging time issues or
             // reproducing a bug by playing exact nodes.
             // See https://github.com/AndyGrant/Ethereal/issues/214
             || ctx.was_hard_stopped
             // This will be the only info we send
             || ctx.options.minimal;

        stop_control.stopped();

        if send_final_info {
            reporter.report_search_progress(game, &result);
        }

        reporter.best_move(game, result.mv);
    } else {
        stop_control.stopped();
    }

    thread_result
}

fn best_result(results: &SearchResults) -> SearchResult {
    let results = results.get();

    // For now, we assume that the main thread produced the best result
    let result = &results[0];

    result.clone()
}

// Simple single-threaded search used by utilities like bench, tests and datagen
pub fn st_search(
    game: &Game,
    persistent_state: &PersistentState,
    time_control: TimeControl,
    reporter: &dyn Reporter,
) -> SearchResult {
    search(
        game,
        persistent_state,
        &mut ThreadData::new(0),
        &SearchResults::new(1),
        time_control,
        &StopControl::new(1),
        &EngineOptions::DEFAULT,
        reporter,
    )
}

pub fn probe_tb_at_root(
    game: &Game,
    tb: &Tablebase,
    time_control: &TimeControl,
) -> Option<SearchResult> {
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
    let score = eval.unwrap_or_else(|| match tb_score {
        Wdl::Win => Eval::tb_mate_in(MAX_SEARCH_DEPTH),
        Wdl::Draw => Eval::DRAW,
        Wdl::Loss => Eval::tb_mated_in(MAX_SEARCH_DEPTH),
    });

    Some(SearchResult {
        id: 0,
        mv: best_move,
        pv,
        score,
        stats: SearchStats {
            time: elapsed,
            depth,
            seldepth: depth,
            nodes: u64::from(depth),
            tbhits: u64::from(depth),
            hashfull: 0,
        },
    })
}
