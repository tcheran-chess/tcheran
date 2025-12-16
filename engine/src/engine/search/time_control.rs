use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{
    chess::{game::Game, moves::Move, player::Player},
    engine::{
        options::EngineOptions,
        search::{SearchContext, TimeControl, params},
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

impl TimeStrategy {
    pub fn new(
        game: &Game,
        time_control: TimeControl,
        control: StopControl,
        options: &EngineOptions,
    ) -> Self {
        let now = Instant::now();
        let move_overhead = Duration::from_millis(options.move_overhead as u64);

        let mut soft_stop = Duration::default();
        let mut hard_stop = Duration::default();

        match time_control {
            TimeControl::ExactTime(move_time) => {
                soft_stop = move_time;
                hard_stop = move_time;
            }
            TimeControl::Clocks(ref clocks) => {
                let (time_remaining, increment) = match game.player {
                    Player::White => (clocks.white_clock, clocks.white_increment),
                    Player::Black => (clocks.black_clock, clocks.black_increment),
                };
                let increment = increment.unwrap_or_default();

                let mut time_remaining = time_remaining.unwrap_or_default();

                time_remaining = time_remaining
                    .saturating_sub(move_overhead)
                    .max(move_overhead);

                let max_time_per_move = time_remaining.mul_f32(params::MAX_TIME_PER_MOVE);

                let base_time = if let Some(moves_to_go) = clocks.moves_to_go {
                    // Try to use a roughly even amount of time per move
                    time_remaining / moves_to_go
                } else {
                    time_remaining.mul_f32(params::BASE_TIME_PER_MOVE)
                } + increment.mul_f32(params::INCREMENT_TO_USE);

                soft_stop = std::cmp::min(
                    base_time.mul_f32(params::SOFT_TIME_MULTIPLIER),
                    max_time_per_move,
                );

                hard_stop = std::cmp::min(
                    base_time.mul_f32(params::HARD_TIME_MULTIPLIER),
                    max_time_per_move,
                );
            }
            TimeControl::Infinite | TimeControl::Depth(_) | TimeControl::Nodes { .. } => {}
        }

        Self {
            time_control,
            started_at: now,
            stopped: false,

            soft_stop,
            hard_stop,

            last_best_move: None,
            best_move_stability: 0,

            next_check_at: CHECK_TERMINATION_NODE_FREQUENCY,

            control,
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn should_start_new_search(&self, depth: u8, ctx: &SearchContext<'_>) -> bool {
        if depth == 1 {
            return true;
        }

        if self.is_force_stopped() {
            return false;
        }

        match self.time_control {
            TimeControl::Infinite => true,
            TimeControl::Clocks(_) => {
                let soft_stop = if depth > params::BEST_MOVE_STABILITY_INITIAL_DEPTH {
                    self.soft_stop.mul_f32(
                        params::BEST_MOVE_STABILITY_TIME_MULTIPLIERS[self.best_move_stability],
                    )
                } else {
                    self.soft_stop
                };

                self.elapsed() < soft_stop
            }
            TimeControl::ExactTime(time) => self.elapsed() < time,
            TimeControl::Depth(d) => d >= depth,
            TimeControl::Nodes { soft, .. } => soft == 0 || ctx.nodes_visited.get() <= soft,
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

    pub fn update(&mut self, nodes_visited: u64) {
        if nodes_visited < self.next_check_at || self.stopped {
            return;
        }

        if self.is_force_stopped() {
            self.stop();
            return;
        }

        self.next_check_at = nodes_visited + CHECK_TERMINATION_NODE_FREQUENCY;

        match self.time_control {
            TimeControl::Clocks(_) => {
                if self.elapsed() > self.hard_stop {
                    self.stop();
                }
            }
            TimeControl::ExactTime(time) => {
                if self.elapsed() > time {
                    self.stop();
                }
            }
            TimeControl::Nodes { hard, .. } => {
                if hard > 0 && nodes_visited > hard {
                    self.stop();
                }
            }
            TimeControl::Infinite | TimeControl::Depth(_) => {}
        }
    }

    pub fn update_after_search(&mut self, best_move: Move, depth: u8) {
        if depth >= params::BEST_MOVE_STABILITY_INITIAL_DEPTH {
            self.best_move_stability = if Some(best_move) == self.last_best_move {
                std::cmp::min(4, self.best_move_stability + 1)
            } else {
                0
            };
        }

        self.last_best_move = Some(best_move);
    }

    fn is_force_stopped(&self) -> bool {
        self.control.should_stop()
    }
}
