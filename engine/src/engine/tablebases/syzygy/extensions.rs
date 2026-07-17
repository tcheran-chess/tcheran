use crate::chess::{File, Game, Move, PieceKind, Player, Rank, Square};

pub trait FlipDiagonalExt {
    type Output;

    fn flip_diagonal(&self) -> Self::Output;
}

impl FlipDiagonalExt for File {
    type Output = Rank;

    fn flip_diagonal(&self) -> Rank {
        Rank::from_idx(self.idx())
    }
}

impl FlipDiagonalExt for Rank {
    type Output = File;

    fn flip_diagonal(&self) -> File {
        File::from_idx(self.idx())
    }
}

impl FlipDiagonalExt for Square {
    type Output = Square;

    fn flip_diagonal(&self) -> Square {
        Square::from_array_index(((self.idx() as u32).wrapping_mul(0x2080_0000) >> 26) as usize)
    }
}

pub trait PlayerExt {
    fn fold_wb<T>(&self, white: T, black: T) -> T;
}

impl PlayerExt for Player {
    fn fold_wb<T>(&self, white: T, black: T) -> T {
        if *self == Player::White { white } else { black }
    }
}

pub trait MoveExt {
    fn is_zeroing(self, game: &Game) -> bool;
}

impl MoveExt for Move {
    fn is_zeroing(self, game: &Game) -> bool {
        let moved_piece = game.board.piece_guaranteed_at(self.from());
        moved_piece.kind == PieceKind::Pawn || self.is_capture()
    }
}

pub(crate) enum Outcome {
    Decisive(Player),
    Draw,
}

pub trait GameExt {
    fn outcome(&self) -> Option<Outcome>;
    fn is_checkmate(&self) -> bool;
}

impl GameExt for Game {
    fn outcome(&self) -> Option<Outcome> {
        if self.moves().is_empty() {
            Some(if self.in_check() {
                Outcome::Decisive(!self.player)
            } else {
                Outcome::Draw // Stalemate
            })
        } else if self.is_stalemate_by_insufficient_material() {
            Some(Outcome::Draw)
        } else {
            None
        }
    }

    fn is_checkmate(&self) -> bool {
        self.in_check() && self.moves().is_empty()
    }
}

impl std::ops::BitXor<bool> for Player {
    type Output = Player;

    fn bitxor(self, rhs: bool) -> Self::Output {
        Self::from_idx((self.idx() ^ rhs as usize) as u8)
    }
}
