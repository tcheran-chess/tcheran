pub mod lmr_table;

pub fn init() {
    lmr_table::init();
}

use crate::{
    chess::{moves::Move, player::Player, square::Square},
    engine::search::{MAX_SEARCH_DEPTH_SIZE, move_ordering},
};

pub struct KillersTable([[Option<Move>; 2]; MAX_SEARCH_DEPTH_SIZE]);

impl KillersTable {
    pub const fn new() -> Self {
        Self([[None; 2]; MAX_SEARCH_DEPTH_SIZE])
    }

    pub fn get_0(&self, plies: u8) -> Option<Move> {
        let plies = plies as usize;
        self.0[plies][0]
    }

    pub fn get_1(&self, plies: u8) -> Option<Move> {
        let plies = plies as usize;
        self.0[plies][1]
    }

    pub fn try_push(&mut self, plies: u8, mv: Move) {
        let killer_0 = self.get_0(plies);

        // If the first killer (which would become the second) is the same as the move we're trying
        // to add, we'd end up with duplicate moves.
        if Some(mv) == killer_0 {
            return;
        }

        let plies = plies as usize;
        self.0[plies][1] = killer_0;
        self.0[plies][0] = Some(mv);
    }
}

pub struct HistoryTable([[[i32; Square::N]; Square::N]; Player::N]);

impl HistoryTable {
    pub const fn new() -> Self {
        Self([[[0; Square::N]; Square::N]; Player::N])
    }

    pub fn reset(&mut self) {
        for from_square in 0..Square::N {
            for to_square in 0..Square::N {
                for player in 0..Player::N {
                    self.0[player][from_square][to_square] = 0;
                }
            }
        }
    }

    fn bonus(depth: u8) -> i32 {
        let depthi32 = i32::from(depth);

        depthi32 * depthi32
    }

    pub fn get(&self, player: Player, mv: Move) -> i32 {
        self.0[player][mv.src()][mv.dst()]
    }

    pub fn add_bonus_for(&mut self, player: Player, mv: Move, depth: u8) {
        let bonus = Self::bonus(depth);
        let existing_score = self.get(player, mv);
        let new_score = std::cmp::min(existing_score + bonus, move_ordering::HISTORY_MAX_SCORE);

        self.0[player][mv.src()][mv.dst()] = new_score;
    }

    pub fn decay(&mut self, decay_factor: i32) {
        for from_square in 0..Square::N {
            for to_square in 0..Square::N {
                for player in 0..Player::N {
                    self.0[player][from_square][to_square] /= decay_factor;
                }
            }
        }
    }
}

pub struct CountermoveTable([[[Option<Move>; Square::N]; Square::N]; Player::N]);

impl CountermoveTable {
    pub const fn new() -> Self {
        Self([[[None; Square::N]; Square::N]; Player::N])
    }

    pub fn set(&mut self, player: Player, previous_move: Move, counter_move: Move) {
        self.0[player][previous_move.src()][previous_move.dst()] = Some(counter_move);
    }

    pub fn get(&self, player: Player, previous_move: Move) -> Option<Move> {
        self.0[player][previous_move.src()][previous_move.dst()]
    }
}
