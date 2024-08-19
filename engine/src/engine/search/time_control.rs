
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::chess::game::Game;
use crate::chess::moves::Move;
use crate::chess::player::Player;
use crate::engine::options::EngineOptions;
use crate::engine::search::{TimeControl, params};

pub struct TimeStrategy {
    time_control: TimeControl,
    started_at: Instant,

    soft_stop: Duration,
    hard_stop: Duration,

    last_best_move: Option<Move>,
    best_move_stability: usize,

    next_check_at: u64,

    force_stop: Arc<AtomicBool>,
}

pub struct Control {
    force_stop: Arc<AtomicBool>,
}

impl Control {
    pub fn stop(&self) {
        self.force_stop.store(true, Ordering::Relaxed);
    }
}

impl TimeStrategy {
    pub fn new(
        game: &Game,
        time_control: &TimeControl,
        options: &EngineOptions,
    ) -> (Self, Control) {
        let now = Instant::now();
        let move_overhead = Duration::from_millis(options.move_overhead as u64);

        let mut soft_stop = Duration::default();
        let mut hard_stop = Duration::default();

        match time_control {
            TimeControl::Infinite => {}
            TimeControl::ExactTime(move_time) => {
                soft_stop = *move_time;
                hard_stop = *move_time;
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

                println!("Have {:?} left", time_remaining);
                println!("Will use max {:?}", max_time_per_move);
                println!("Will stop new search depths at {:?}", soft_stop);
                println!("Will abort at {:?}", hard_stop);
            }
        }

        let force_stop = Arc::new(AtomicBool::new(false));

        let control = Control {
            force_stop: force_stop.clone(),
        };

        let time_strategy = Self {
            time_control: time_control.clone(),
            started_at: now,

            soft_stop,
            hard_stop,

            last_best_move: None,
            best_move_stability: 0,

            next_check_at: params::CHECK_TERMINATION_NODE_FREQUENCY,

            force_stop,
        };

        (time_strategy, control)
    }

    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    pub fn should_start_new_search(&self, depth: u8) -> bool {
        if depth == 1 {
            return true;
        }

        if self.is_force_stopped() {
            return false;
        }

        match self.time_control {
            TimeControl::Clocks(_) => {
                let soft_stop = if depth > params::BEST_MOVE_STABILITY_INITIAL_DEPTH {
                    self.soft_stop.mul_f32(
                        params::BEST_MOVE_STABILITY_TIME_MULTIPLIERS[self.best_move_stability],
                    )
                } else {
                    self.soft_stop
                };

                println!(
                    "stability is now {} so soft stop is {:?}",
                    self.best_move_stability, soft_stop
                );
                self.elapsed() < soft_stop
            }
            TimeControl::ExactTime(time) => self.elapsed() < time,
            TimeControl::Infinite => true,
        }
    }

    pub fn should_stop(&mut self, nodes_visited: u64) -> bool {
        if nodes_visited < self.next_check_at {
            return false;
        }

        if self.is_force_stopped() {
            return true;
        }

        self.next_check_at = nodes_visited + params::CHECK_TERMINATION_NODE_FREQUENCY;

        match self.time_control {
            TimeControl::Clocks(_) => self.elapsed() > self.hard_stop,
            TimeControl::ExactTime(time) => self.elapsed() > time,
            TimeControl::Infinite => false,
        }
    }

    pub fn update(&mut self, best_move: Move, depth: u8) {
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
        self.force_stop.load(Ordering::Relaxed)
    }
}
