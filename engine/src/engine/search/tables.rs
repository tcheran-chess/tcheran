pub mod lmr_table;

pub fn init() {
    lmr_table::init();
}

use std::cmp::min;

use crate::{
    chess::{moves::Move, player::Player, square::Square},
    engine::search::MAX_SEARCH_DEPTH_SIZE,
};

pub struct Tables<'s> {
    pub history_table: &'s mut HistoryTable,

    pub killer_moves: KillersTable,
    pub countermove_table: CountermoveTable,
}

impl<'s> Tables<'s> {
    pub fn new(history_table: &'s mut HistoryTable) -> Self {
        Self {
            history_table,
            killer_moves: KillersTable::new(),
            countermove_table: CountermoveTable::new(),
        }
    }
}

pub struct KillersTable([Option<Move>; MAX_SEARCH_DEPTH_SIZE]);

impl KillersTable {
    pub const fn new() -> Self {
        Self([None; MAX_SEARCH_DEPTH_SIZE])
    }

    pub fn get(&self, plies: u8) -> Option<Move> {
        self.0[plies as usize]
    }

    pub fn set(&mut self, plies: u8, mv: Move) {
        self.0[plies as usize] = Some(mv);
    }
}

pub const HISTORY_MAX_BONUS: i16 = 1600;
pub const HISTORY_FACTOR: i16 = 350;
pub const HISTORY_OFFSET: i16 = 350;

pub fn history_bonus(depth: u8) -> i16 {
    min(HISTORY_FACTOR * i16::from(depth) - HISTORY_OFFSET, HISTORY_MAX_BONUS)
}

#[expect(clippy::cast_possible_truncation, reason = "Dipped into i32 to avoid overflows")]
const fn taper_bonus(bonus: i16, old: i16, max: i32) -> i16 {
    let old = old as i32;
    let bonus = bonus as i32;

    (old + bonus - (old * bonus.abs()) / max) as i16
}

#[derive(Clone)]
pub struct HistoryTable([[[i16; Square::N]; Square::N]; Player::N]);

impl HistoryTable {
    const MAX: i32 = 8192;

    pub const fn new() -> Self {
        Self([[[0; Square::N]; Square::N]; Player::N])
    }

    pub fn get(&self, player: Player, mv: Move) -> i32 {
        i32::from(self.0[player][mv.src()][mv.dst()])
    }

    fn update(&mut self, player: Player, mv: Move, bonus: i16) {
        let old = &mut self.0[player][mv.src()][mv.dst()];
        *old = taper_bonus(bonus, *old, Self::MAX);
    }

    pub fn add_bonus_for(&mut self, player: Player, mv: Move, depth: u8) {
        let bonus = history_bonus(depth);
        self.update(player, mv, bonus);
    }

    pub fn add_malus_for(&mut self, player: Player, mv: Move, depth: u8) {
        let malus = -history_bonus(depth);
        self.update(player, mv, malus);
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
