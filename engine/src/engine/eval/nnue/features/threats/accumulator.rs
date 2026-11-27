use crate::{
    chess::{
        MoveObserver,
        moves::Move,
        prelude::{Board, H8, Piece, Player, Square},
    },
    engine::eval::nnue::{Accumulator, network::L1},
    util::arrayvec::ArrayVec,
};

#[derive(Debug, Clone)]
pub struct ThreatChanges {
    pub adds: ArrayVec<(Piece, Square), 2>,
    pub subs: ArrayVec<(Piece, Square), 2>,
    pub mv: Move,
    pub moved_piece: Piece,
}

impl ThreatChanges {
    pub fn uninit() -> Self {
        Self {
            adds: ArrayVec::new(),
            subs: ArrayVec::new(),

            // Values chosen arbitrarily since they will always be overwritten
            mv: Move::quiet(H8, H8),
            moved_piece: Piece::WHITE_PAWN,
        }
    }

    pub fn clear(&mut self) {
        self.adds.clear();
        self.subs.clear();

        self.mv = Move::quiet(H8, H8);
        self.moved_piece = Piece::WHITE_PAWN;
    }
}

#[derive(Clone)]
pub struct ThreatsStackEntry {
    pub accumulators: [Accumulator; Player::N],
    pub changes: ThreatChanges,
    pub correct: [bool; Player::N],
}

impl Default for ThreatsStackEntry {
    fn default() -> Self {
        Self {
            accumulators: [Accumulator([0; L1]), Accumulator([0; L1])],
            changes: ThreatChanges::uninit(),
            correct: [false; Player::N],
        }
    }
}

impl MoveObserver for ThreatsStackEntry {
    fn init(&mut self, moved_piece: Piece, mv: Move) {
        self.changes.moved_piece = moved_piece;
        self.changes.mv = mv;
    }

    fn set(&mut self, sq: Square, piece: Piece) {
        self.changes.adds.push((piece, sq));
    }

    fn remove(&mut self, sq: Square, piece: Piece) {
        self.changes.subs.push((piece, sq));
    }
}

impl ThreatsStackEntry {
    pub fn clear(&mut self) {
        self.changes.clear();
        self.correct = [false; Player::N];
    }

    pub(in crate::engine::eval::nnue) fn refresh(&mut self, board: &Board, pov: Player) {
        todo!();
        //self.correct[pov] = true;
    }

    pub fn update(&mut self, prev: &Self, king: Square, pov: Player) {
        todo!();
        //self.correct[pov] = true;
    }
}
