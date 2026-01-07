use std::{
    sync::{Arc, Mutex, atomic::AtomicU64},
    thread,
    time::{Duration, Instant},
};

use crate::{
    chess::{game::Game, moves::Move, piece::Piece, player::Player},
    engine::{
        eval::{Eval, nnue::NetworkStack},
        options::EngineOptions,
        search::{
            iterative_deepening,
            principal_variation::PrincipalVariation,
            tables::{KillersTable, Tables},
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

pub struct PersistentState {
    pub tt: TranspositionTable,
    pub tablebase: Tablebase,

    thread_data: Vec<Arc<Mutex<ThreadData>>>,
}

impl PersistentState {
    pub fn new(tt_size_mb: usize) -> Self {
        Self::with_tablebase(tt_size_mb, &Tablebase::new())
    }

    pub fn with_tablebase(tt_size_mb: usize, tb: &Tablebase) -> Self {
        Self {
            tt: TranspositionTable::new(tt_size_mb),
            tablebase: tb.clone(),

            thread_data: vec![Arc::new(Mutex::new(ThreadData::new()))],
        }
    }

    pub fn reset(&mut self) {
        self.tt.reset();

        for thread_data in &mut self.thread_data {
            let mut thread_data = thread_data.lock().unwrap();
            thread_data.reset();
        }
    }

    pub fn get_thread_data(&self, thread_id: usize) -> Arc<Mutex<ThreadData>> {
        self.thread_data[thread_id].clone()
    }

    pub fn scale_threads(&mut self, threads: usize) {
        self.thread_data.clear();

        for _ in 0..threads {
            self.thread_data
                .push(Arc::new(Mutex::new(ThreadData::new())));
        }
    }
}

pub struct ThreadData {
    pub tables: Tables,
    pub nnue: NetworkStack,
    pub stack: SearchStack,
}

impl ThreadData {
    pub fn new() -> Self {
        Self {
            tables: Tables::new(),
            nnue: NetworkStack::new(),
            stack: SearchStack::new(),
        }
    }

    pub fn new_search(&mut self, game: &Game) {
        self.tables.killer_moves = KillersTable::new();
        // TODO: Do we need to reset the stack every time?
        self.stack = SearchStack::new();
        self.nnue.setup(&game.board);
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

pub struct SearchContext<'s> {
    pub tt: &'s TranspositionTable,
    pub tablebase: &'s Tablebase,

    pub tables: &'s mut Tables,
    pub nnue: &'s mut NetworkStack,
    pub stack: &'s mut SearchStack,

    pub time_control: TimeStrategy,

    pub completed_depth: u8,
    pub max_depth_reached: u8,
    pub root_depth: u8,
    pub nodes_visited: BufferedAtomicU64<'s>,
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

            tables,
            stack: search_stack,
            nnue,
            time_control: TimeStrategy::new(game, time_control, stop_control, options),

            completed_depth: 0,
            max_depth_reached: 0,
            root_depth: 0,
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

pub struct SearchStackEntry {
    pub mv: Option<(Move, Piece)>,
    pub eval: Eval,

    pub excluded_mv: Option<Move>,
    pub double_extensions: u8,
}

impl SearchStackEntry {
    pub const fn new() -> Self {
        Self {
            mv: None,
            eval: Eval::MIN,

            excluded_mv: None,
            double_extensions: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TimeControl {
    Clocks { clocks: Clocks, start_time: Instant },
    ExactTime { time: Duration, start_time: Instant },
    Depth(u8),
    Nodes { soft: u64, hard: u64 },
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

pub struct SearchInfo {
    pub game: Game,
    pub eval: Eval,
    pub stats: SearchStats,
    pub pv: PrincipalVariation,
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
            depth: ctx.completed_depth,
            seldepth: ctx.max_depth_reached,
            nodes: ctx.nodes_visited.get_global(),
            nodes_per_second: util::metrics::nodes_per_second(
                ctx.nodes_visited.get_global(),
                ctx.time_control.elapsed(),
            ),
            tbhits: ctx.tbhits.get_global(),
            hashfull: ctx.tt.occupancy(),
        }
    }
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

pub fn search(
    game: &Game,
    persistent_state: &mut PersistentState,
    time_control: TimeControl,
    stop_control: StopControl,
    options: &EngineOptions,
    reporter: &impl Reporter,
) -> (Move, Eval) {
    persistent_state.tt.new_generation();

    let tablebase_result = persistent_state.tablebase.best_move(game);
    if let Some(mv) = tablebase_result {
        let start_time = Instant::now();
        let (pv, eval) = get_tablebase_pv(game, &persistent_state.tablebase);
        let elapsed = start_time.elapsed();

        let depth = pv.len();

        reporter.report_search_progress(
            game,
            SearchInfo {
                game: game.clone(),
                eval,
                pv,
                stats: SearchStats {
                    time: elapsed,
                    depth,
                    seldepth: depth,
                    nodes: u64::from(depth),
                    nodes_per_second: util::metrics::nodes_per_second(u64::from(depth), elapsed),
                    tbhits: 1,
                    hashfull: 0,
                },
            },
        );

        reporter.best_move(game, mv);
        return (mv, eval);
    }

    let threads_stop_control = StopControl::new();
    let global_node_count = AtomicU64::new(0);
    let global_tbhits_count = AtomicU64::new(0);

    let result = thread::scope(|scope| {
        let mut threads = Vec::new();

        // If we want more than one thread, spawn our other threads, sharing the transposition
        // table but with their own copy of everything else.
        for thread_id in 1..options.threads {
            let thread_data = persistent_state.get_thread_data(thread_id);
            let tt = &persistent_state.tt;
            let tablebase = &persistent_state.tablebase;
            let global_node_count = &global_node_count;
            let global_tbhits_count = &global_tbhits_count;

            let this_thread_stop_control = threads_stop_control.clone();

            let thread = scope.spawn(move || {
                let mut thread_data_handle =
                    thread_data.lock().expect("Unable to lock thread data");

                thread_data_handle.new_search(game);

                let (tables, stack, nnue) = thread_data_handle.mut_refs();

                let mut ctx = SearchContext::new(
                    game,
                    tt,
                    tablebase,
                    global_node_count,
                    global_tbhits_count,
                    tables,
                    stack,
                    nnue,
                    TimeControl::Infinite,
                    this_thread_stop_control,
                    options,
                );

                iterative_deepening::search(&mut game.clone(), &mut ctx, &NullReporter);
            });

            threads.push(thread);
        }

        let main_thread_data = persistent_state.get_thread_data(0);
        let mut main_thread_data = main_thread_data
            .lock()
            .expect("Unable to lock main thread data");

        main_thread_data.new_search(game);

        let (tables, stack, nnue) = main_thread_data.mut_refs();

        let mut ctx = SearchContext::new(
            game,
            &persistent_state.tt,
            &persistent_state.tablebase,
            &global_node_count,
            &global_tbhits_count,
            tables,
            stack,
            nnue,
            time_control,
            stop_control,
            options,
        );

        let result = iterative_deepening::search(
            // Give the search its own copy of the game so we don't get one returned in a dirty state
            // when the search aborts.
            &mut game.clone(),
            &mut ctx,
            reporter,
        );

        threads_stop_control.stop();
        for thread in threads {
            thread.join().expect("Thread panicked");
        }

        let result = result.expect("Should always have a result");

        // Always send a final info line before reporting the best move so we have useful information
        // such as the exact number of nodes searched and the exact time used. This could be useful for
        // debugging time issues or reproducing a bug by playing exact nodes.
        // See https://github.com/AndyGrant/Ethereal/issues/214
        reporter.report_search_progress(
            game,
            SearchInfo {
                game: game.clone(),
                eval: result.eval,
                pv: result.pv.clone(),
                stats: SearchStats::from_ctx(&ctx),
            },
        );

        result
    });

    reporter.best_move(game, result.best_move);
    (result.best_move, result.eval)
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
