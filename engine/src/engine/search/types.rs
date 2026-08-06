use std::{
    cmp::Ordering,
    sync::{OnceLock, atomic::AtomicU64},
    time::{Duration, Instant},
};

use crate::{
    chess::prelude::*,
    engine::{
        eval::{Eval, nnue::NetworkStack},
        options::EngineOptions,
        search::{
            MAX_PLIES_ARRAY_SIZE,
            principal_variation::PrincipalVariation,
            tables::Tables,
            time_control::{StopControl, TimeStrategy},
        },
        tablebases::Tablebase,
        transposition_table::TranspositionTable,
        util::buffered_atomic_counter::BufferedAtomicU64,
    },
};

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
        self.node_counter
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.tbhits_counter
            .store(0, std::sync::atomic::Ordering::Relaxed);
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
    pub root_depth: Depth,
    pub was_hard_stopped: bool,
    pub nodes: BufferedAtomicU64<'s>,
    pub tbhits: BufferedAtomicU64<'s>,

    pub min_nmp_ply: u8,
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
            root_depth: Depth::ZERO,
            was_hard_stopped: false,
            nodes: BufferedAtomicU64::new(node_counter),
            tbhits: BufferedAtomicU64::new(tbhits_counter),

            min_nmp_ply: 0,
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

    pub fn update_after_search(&mut self, best_move: Move, depth: Depth) {
        self.time_control
            .update_after_search(best_move, depth, self.nodes.get());
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
    pub reduction: i32,
}

impl SearchStackEntry {
    pub const fn new() -> Self {
        Self {
            mv: None,
            eval: Eval::MIN,

            excluded_mv: None,
            double_extensions: 0,
            fail_highs: 0,
            reduction: 0,
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
    pub depth: u8,
    pub seldepth: u8,
    pub pv: PrincipalVariation,
    pub stats: SearchStats,
}

#[derive(Clone)]
pub struct SearchStats {
    pub time: Duration,
    pub nodes: u64,
    pub tbhits: u64,
    pub hashfull: u64,
}

impl SearchStats {
    pub fn from_ctx(ctx: &SearchContext<'_>) -> Self {
        Self {
            time: ctx.time_control.elapsed(),
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ScoreWindow {
    pub alpha: Eval,
    pub beta: Eval,
}

impl ScoreWindow {
    pub fn new(alpha: Eval, beta: Eval) -> Self {
        Self { alpha, beta }
    }

    pub fn zero_window_around_alpha(&self) -> Self {
        Self {
            alpha: self.alpha,
            beta: self.alpha + 1,
        }
    }

    pub fn zero_window_around_beta(&self) -> Self {
        Self {
            alpha: self.beta - 1,
            beta: self.beta,
        }
    }

    pub fn clamp_alpha(&mut self, max: Eval) {
        self.alpha = self.alpha.max(max);
    }

    pub fn clamp_beta(&mut self, min: Eval) {
        self.beta = self.beta.min(min);
    }

    pub fn is_zero_window(&self) -> bool {
        self.alpha == self.beta - 1
    }
}

impl std::ops::Neg for ScoreWindow {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            alpha: -self.beta,
            beta: -self.alpha,
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Depth(u8);

impl Depth {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn as_i32(self) -> i32 {
        self.0 as i32
    }

    pub const fn idx(self) -> usize {
        self.0 as usize
    }
}

impl PartialEq<u8> for Depth {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for Depth {
    fn partial_cmp(&self, other: &u8) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::ops::Add<Self> for Depth {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::Add<u8> for Depth {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl std::ops::Add<i8> for Depth {
    type Output = Self;

    fn add(self, rhs: i8) -> Self::Output {
        Self(self.0.saturating_add_signed(rhs))
    }
}

impl std::ops::AddAssign<u8> for Depth {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl std::ops::Sub<Self> for Depth {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl std::ops::Sub<u8> for Depth {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl std::ops::SubAssign<u8> for Depth {
    fn sub_assign(&mut self, rhs: u8) {
        self.0 = self.0.saturating_sub(rhs);
    }
}

impl std::ops::Mul<i32> for Depth {
    type Output = i32;

    fn mul(self, rhs: i32) -> Self::Output {
        i32::from(self.0) * rhs
    }
}

impl std::ops::Mul<Self> for Depth {
    type Output = i32;

    fn mul(self, rhs: Self) -> Self::Output {
        i32::from(self.0) * i32::from(rhs.0)
    }
}

impl std::ops::Div<u8> for Depth {
    type Output = Self;

    fn div(self, rhs: u8) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl std::ops::Div<i32> for Depth {
    type Output = i32;

    fn div(self, rhs: i32) -> Self::Output {
        i32::from(self.0) / rhs
    }
}

impl std::fmt::Display for Depth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct SearchResults(Vec<OnceLock<SearchResult>>);

impl SearchResults {
    pub fn new(n: usize) -> Self {
        Self(
            (0..n)
                .map(|_| OnceLock::new())
                .collect::<Vec<OnceLock<SearchResult>>>(),
        )
    }

    pub fn set(&self, n: usize, result: &SearchResult) {
        _ = self
            .0
            .get(n)
            .expect("Should have unique access to set result for our thread")
            .set(result.clone());
    }

    pub fn get(&self) -> Vec<SearchResult> {
        self.0
            .iter()
            .map(|l| l.get().expect("Every thread should've reported a result"))
            .cloned()
            .collect::<Vec<_>>()
    }
}
