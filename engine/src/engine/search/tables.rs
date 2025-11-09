pub mod lmr_table;

pub fn init() {
    lmr_table::init();
}

use std::cmp::min;

use crate::{
    chess::{
        game::Game,
        moves::{Move, MoveList},
        piece::{Piece, PieceKind},
        player::Player,
        square::Square,
    },
    engine::{search::MAX_SEARCH_DEPTH_SIZE, util::mem::alloc_boxed},
};

pub struct Tables {
    pub quiet_history: Box<HistoryTable>,
    pub capture_history: Box<CaptureHistoryTable>,
    pub killer_moves: KillersTable,
    pub conthist: Box<ContHistTable>,
}

impl Tables {
    pub fn new() -> Self {
        Self {
            quiet_history: HistoryTable::new(),
            capture_history: CaptureHistoryTable::new(),
            killer_moves: KillersTable::new(),
            conthist: ContHistTable::new(),
        }
    }

    pub fn new_search(&mut self) {
        self.killer_moves = KillersTable::new();
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

pub struct HistoryTable([[[[[i16; 2]; 2]; Square::N]; Square::N]; Player::N]);

impl HistoryTable {
    const MAX: i32 = 8192;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    pub fn get(&self, game: &Game, mv: Move) -> i32 {
        let from_threatened = usize::from(game.threats.contains(mv.src()));
        let to_threatened = usize::from(game.threats.contains(mv.dst()));

        i32::from(self.0[game.player][mv.src()][mv.dst()][from_threatened][to_threatened])
    }

    fn update_for_move(&mut self, game: &Game, mv: Move, bonus: i16) {
        let from_threatened = usize::from(game.threats.contains(mv.src()));
        let to_threatened = usize::from(game.threats.contains(mv.dst()));

        let old = &mut self.0[game.player][mv.src()][mv.dst()][from_threatened][to_threatened];
        *old = taper_bonus(bonus, *old, Self::MAX);
    }

    pub fn update(&mut self, game: &Game, mv: Move, depth: u8, other_quiets_tried: &MoveList) {
        let bonus = history_bonus(depth);

        self.update_for_move(game, mv, bonus);

        for other_quiet in other_quiets_tried {
            self.update_for_move(game, *other_quiet, -bonus);
        }
    }
}

pub struct CaptureHistoryTable([[[[i16; PieceKind::N]; Square::N]; PieceKind::N]; Player::N]);

impl CaptureHistoryTable {
    pub const MAX: i32 = 8192;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    pub fn get(
        &self,
        player: Player,
        capturing_piece: PieceKind,
        capture_square: Square,
        captured_piece: PieceKind,
    ) -> i32 {
        i32::from(self.0[player][capturing_piece][capture_square][captured_piece])
    }

    fn update_for_move(&mut self, mv: Move, game: &Game, bonus: i16) {
        let capturing_piece = game.board.piece_guaranteed_at(mv.src()).kind;
        let capture_square = mv.dst();
        let captured_piece = if mv.is_en_passant() {
            PieceKind::Pawn
        } else {
            game.board.piece_guaranteed_at(mv.dst()).kind
        };

        let old = &mut self.0[game.player][capturing_piece][capture_square][captured_piece];
        *old = taper_bonus(bonus, *old, Self::MAX);
    }

    pub fn update(&mut self, mv: Move, game: &Game, depth: u8, other_captures_tried: &MoveList) {
        let bonus = history_bonus(depth);

        if mv.is_capture() {
            self.update_for_move(mv, game, bonus);
        }

        for other_capture in other_captures_tried {
            self.update_for_move(*other_capture, game, -bonus);
        }
    }
}

pub struct ContHistTable([[[[i16; Square::N]; Piece::N]; Square::N]; Piece::N]);

impl ContHistTable {
    const MAX: i32 = 16384;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    pub fn get(
        &self,
        game: &Game,
        previous_piece_moved: Piece,
        previous_moved_to: Square,
        mv: Move,
    ) -> i32 {
        let moved = game.board.piece_guaranteed_at(mv.src());
        i32::from(self.0[previous_piece_moved][previous_moved_to][moved][mv.dst()])
    }

    fn update_for_move(
        &mut self,
        previous_piece_moved: Piece,
        previous_moved_to: Square,
        moved: Piece,
        moved_to: Square,
        bonus: i16,
    ) {
        let old = &mut self.0[previous_piece_moved][previous_moved_to][moved][moved_to];
        *old = taper_bonus(bonus, *old, Self::MAX);
    }

    pub fn update(
        &mut self,
        game: &Game,
        previous_piece_moved: Piece,
        previous_move: Move,
        mv: Move,
        depth: u8,
        quiets_tried: &MoveList,
    ) {
        let bonus = history_bonus(depth);

        let moved = game.board.piece_guaranteed_at(mv.src());
        self.update_for_move(previous_piece_moved, previous_move.dst(), moved, mv.dst(), bonus);

        for quiet_tried in quiets_tried {
            let try_moved = game.board.piece_guaranteed_at(quiet_tried.src());
            self.update_for_move(
                previous_piece_moved,
                previous_move.dst(),
                try_moved,
                quiet_tried.dst(),
                -bonus,
            );
        }
    }
}
