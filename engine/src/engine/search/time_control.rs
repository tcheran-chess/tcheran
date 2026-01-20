use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{
    chess::{game::Game, moves::Move, square::Square},
    engine::{
        options::EngineOptions,
        params::*,
        search::{SearchContext, TimeControl, types::Depth},
    },
};

pub struct TimeStrategy {
    time_control: TimeControl,
    started_at: Instant,
    stopped: bool,

    soft_stop: Duration,
    hard_stop: Duration,

    last_best_move: Option<Move>,
    best_move_stability: usize,
    nodes_used: [[u64; Square::N]; Square::N],
    scale: f32,

    next_check_at: u64,

    control: StopControl,
}

#[derive(Clone)]
pub struct StopControl {
    force_stop: Arc<AtomicBool>,
}

impl StopControl {
    pub fn new() -> Self {
        Self {
            force_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn stop(&self) {
        self.force_stop.store(true, Ordering::Relaxed);
    }

    pub fn should_stop(&self) -> bool {
        self.force_stop.load(Ordering::Relaxed)
    }
}

const CHECK_TERMINATION_NODE_FREQUENCY: u64 = 2048;
const BEST_MOVE_STABILITY_TIME_MULTIPLIERS: [f32; 5] = [2.50, 1.20, 1.00, 0.80, 0.75];

impl TimeStrategy {
    pub fn new(
        game: &Game,
        time_control: TimeControl,
        control: StopControl,
        options: &EngineOptions,
    ) -> Self {
        let mut started_at = None;
        let mut soft_stop = Duration::default();
        let mut hard_stop = Duration::default();
        let mut next_check_at = CHECK_TERMINATION_NODE_FREQUENCY;

        match time_control {
            TimeControl::ExactTime {
                time: move_time,
                start_time,
            } => {
                started_at = Some(start_time);
                soft_stop = move_time;
                hard_stop = move_time;
            }
            TimeControl::Clocks {
                ref clocks,
                start_time,
            } => {
                started_at = Some(start_time);
                let (mut time_remaining, increment) = clocks.for_player(game.player);

                time_remaining = time_remaining
                    .saturating_sub(options.move_overhead)
                    .max(options.move_overhead);

                let absolute_max = time_remaining.mul_f32(max_time_per_move());
                let moves_to_go = clocks.moves_to_go.unwrap_or(default_moves_to_go());

                let base_time = absolute_max / moves_to_go + increment.mul_f32(increment_to_use());

                hard_stop = absolute_max.mul_f32(hard_time_multiplier());
                soft_stop = std::cmp::min(base_time.mul_f32(soft_time_multiplier()), hard_stop);
            }
            TimeControl::Nodes { hard, .. } => {
                if let Some(hard_limit) = hard {
                    next_check_at = hard_limit;
                }
            }
            TimeControl::Infinite | TimeControl::Depth(_) => {}
        }

        Self {
            time_control,
            started_at: started_at.unwrap_or_else(Instant::now),
            stopped: false,

            soft_stop,
            hard_stop,

            last_best_move: None,
            best_move_stability: 0,
            nodes_used: [[0; Square::N]; Square::N],
            scale: 1.0,

            next_check_at,

            control,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn should_start_new_search(&self, depth: Depth, ctx: &SearchContext<'_>) -> bool {
        if depth == 1 {
            return true;
        }

        if self.is_force_stopped() {
            return false;
        }

        match self.time_control {
            TimeControl::Infinite => true,
            TimeControl::Clocks { .. } => self.elapsed() < self.soft_stop.mul_f32(self.scale),
            TimeControl::ExactTime { time, .. } => self.elapsed() < time,
            TimeControl::Depth(d) => d >= depth,
            TimeControl::Nodes { soft, .. } => {
                soft.is_none() || soft.is_some_and(|s| ctx.nodes.get() <= s)
            }
        }
    }

    #[inline]
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    #[inline]
    fn stop(&mut self) {
        self.stopped = true;
    }

    pub fn update(&mut self, nodes_visited: u64, root_depth: Depth) {
        // If we're on our first iterative deepening iteration we don't have a best move
        // yet, so don't force-stop the search under any circumstances.
        if root_depth == 1 {
            return;
        }

        if nodes_visited < self.next_check_at || self.stopped {
            return;
        }

        if self.is_force_stopped() {
            self.stop();
            return;
        }

        self.next_check_at = nodes_visited + CHECK_TERMINATION_NODE_FREQUENCY;

        match self.time_control {
            TimeControl::Clocks { .. } => {
                if self.elapsed() > self.hard_stop {
                    self.stop();
                }
            }
            TimeControl::ExactTime { time, .. } => {
                if self.elapsed() > time {
                    self.stop();
                }
            }
            TimeControl::Nodes { hard, .. } => {
                // If we had a hard node limit, we will have waited to check until we hit
                // that limit so we can just stop immediately here.
                if hard.is_some() {
                    self.stop();
                }
            }
            TimeControl::Infinite | TimeControl::Depth(_) => {}
        }
    }

    pub fn update_nodes_used(&mut self, mv: Move, nodes_used: u64) {
        self.nodes_used[mv.src()][mv.dst()] += nodes_used;
    }

    #[expect(clippy::cast_precision_loss, reason = "Time management calculations can be approx")]
    pub fn update_after_search(&mut self, best_move: Move, depth: Depth, nodes_visited: u64) {
        let mut scale = 1.0;

        if depth >= best_move_stability_initial_depth() {
            self.best_move_stability = if Some(best_move) == self.last_best_move {
                std::cmp::min(4, self.best_move_stability + 1)
            } else {
                0
            };

            scale *= BEST_MOVE_STABILITY_TIME_MULTIPLIERS[self.best_move_stability];
        }

        let nodes_for_best_move = self.nodes_used[best_move.src()][best_move.dst()];
        let fraction_used_for_best_move = nodes_for_best_move as f32 / nodes_visited as f32;
        let node_scale_adjustment = fraction_used_for_best_move
            .mul_add(-node_tm_multiplier(), node_tm_base())
            .max(node_tm_min());

        scale *= node_scale_adjustment;

        self.scale = scale;
        self.last_best_move = Some(best_move);
    }

    fn is_force_stopped(&self) -> bool {
        self.control.should_stop()
    }
}
