use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{
    chess::{Game, Move, Square},
    engine::{
        options::EngineOptions,
        params::*,
        search::{TimeControl, types::Depth},
    },
    util::monitor::Monitor,
};

pub struct TimeStrategy {
    time_control: TimeControl,
    started_at: Instant,

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
    running: Monitor<u32>,
}

impl StopControl {
    pub fn new(n: u32) -> Self {
        Self {
            force_stop: Arc::new(AtomicBool::new(false)),
            running: Monitor::new(n),
        }
    }

    pub fn start_search(&self, n: u32) {
        self.force_stop.store(false, Ordering::Relaxed);
        self.running.set(n);
    }

    pub fn stop(&self) {
        self.force_stop.store(true, Ordering::Relaxed);
    }

    pub fn stopped(&self) {
        self.running.modify(|n| *n -= 1, |n| n == 0 || n == 1);
    }

    pub fn reset(&self) {
        self.force_stop.store(false, Ordering::Relaxed);
    }

    pub fn is_busy(&self) -> bool {
        self.running.value() != 0
    }

    pub fn should_stop(&self) -> bool {
        self.force_stop.load(Ordering::Relaxed)
    }

    pub fn wait_until_last(&self) {
        self.running.wait_while(|v| *v > 1);
    }

    pub fn wait_until_finished(&self) {
        self.running.wait_while(|v| *v > 0);
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

                let absolute_max = time_remaining.mul_f32(max_time_per_move() as f32 / 100.0);
                let moves_to_go = clocks.moves_to_go.unwrap_or(default_moves_to_go());

                let base_time = absolute_max / moves_to_go
                    + increment.mul_f32(increment_to_use() as f32 / 100.0);

                hard_stop = absolute_max.mul_f32(hard_time_multiplier() as f32 / 100.0);
                soft_stop = std::cmp::min(
                    base_time.mul_f32(soft_time_multiplier() as f32 / 100.0),
                    hard_stop,
                );
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

    pub fn should_start_new_search(&self, nodes: u64, depth: Depth) -> bool {
        if depth == 1 {
            return true;
        }

        if self.control.should_stop() {
            return false;
        }

        match self.time_control {
            TimeControl::Infinite => true,
            TimeControl::Clocks { .. } => self.elapsed() < self.soft_stop.mul_f32(self.scale),
            TimeControl::ExactTime { time, .. } => self.elapsed() < time,
            TimeControl::Depth(d) => d >= depth,
            TimeControl::Nodes { soft, .. } => soft.is_none_or(|s| nodes <= s),
        }
    }

    #[inline]
    pub fn stopped(&mut self, nodes: u64, root_depth: Depth) -> bool {
        // If we're on our first iterative deepening iteration we don't have a best move
        // yet, so don't force-stop the search under any circumstances.
        if root_depth == 1 {
            return false;
        }

        // If either we, or another thread, has already asked to stop, then we're stopped
        if self.control.should_stop() {
            return true;
        }

        if nodes < self.next_check_at {
            return false;
        }

        self.next_check_at = nodes + CHECK_TERMINATION_NODE_FREQUENCY;

        let should_stop = match self.time_control {
            TimeControl::Clocks { .. } => self.elapsed() > self.hard_stop,
            TimeControl::ExactTime { time, .. } => self.elapsed() > time,
            TimeControl::Nodes { hard, .. } => {
                // If we had a hard node limit, we will have waited to check until we hit
                // that limit so we can just stop immediately here.
                hard.is_some()
            }
            TimeControl::Infinite | TimeControl::Depth(_) => false,
        };

        if should_stop {
            // This will only ever be called by the main thread, as the other threads are running
            // in TimeControl::Infinite
            self.control.stop();
        }

        should_stop
    }

    pub fn update_nodes_used(&mut self, mv: Move, nodes_used: u64) {
        self.nodes_used[mv.from()][mv.to()] += nodes_used;
    }

    pub fn update_after_search(&mut self, best_move: Move, depth: Depth, nodes_visited: u64) {
        // Update best move stability
        if Some(best_move) == self.last_best_move {
            self.best_move_stability += 1;
        } else {
            self.best_move_stability = 0;
        }

        self.last_best_move = Some(best_move);

        // Compute new scale
        let mut scale = 1.0;

        if depth >= best_move_stability_initial_depth() {
            scale *= BEST_MOVE_STABILITY_TIME_MULTIPLIERS[self.best_move_stability.min(4)];
        }

        let node_scale_adjustment = {
            let nodes_for_best_move = self.nodes_used[best_move.from()][best_move.to()];
            let fraction_used_for_best_move = nodes_for_best_move as f32 / nodes_visited as f32;

            fraction_used_for_best_move
                .mul_add(-(node_tm_multiplier() as f32 / 100.0), node_tm_base() as f32 / 100.0)
                .max(node_tm_min() as f32 / 100.0)
        };

        scale *= node_scale_adjustment;

        self.scale = scale;
    }
}
