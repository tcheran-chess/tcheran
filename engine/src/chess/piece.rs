use crate::chess::prelude::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    pub const N: usize = 6;

    pub const ALL: [Self; Self::N] = [Pawn, Knight, Bishop, Rook, Queen, King];
}

impl<T> std::ops::Index<PieceKind> for [T; PieceKind::N] {
    type Output = T;

    fn index(&self, index: PieceKind) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> std::ops::IndexMut<PieceKind> for [T; PieceKind::N] {
    fn index_mut(&mut self, index: PieceKind) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PromotionPieceKind {
    Knight,
    Bishop,
    Rook,
    Queen,
}

impl PromotionPieceKind {
    pub const fn piece(self) -> PieceKind {
        match self {
            Self::Knight => Knight,
            Self::Bishop => Bishop,
            Self::Rook => Rook,
            Self::Queen => Queen,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Piece {
    pub kind: PieceKind,
    pub player: Player,
}

impl Piece {
    pub const N: usize = 12;

    pub const WHITE_PAWN: Self = Self::white(Pawn);
    pub const WHITE_KNIGHT: Self = Self::white(Knight);
    pub const WHITE_BISHOP: Self = Self::white(Bishop);
    pub const WHITE_ROOK: Self = Self::white(Rook);
    pub const WHITE_QUEEN: Self = Self::white(Queen);
    pub const WHITE_KING: Self = Self::white(King);

    pub const BLACK_PAWN: Self = Self::black(Pawn);
    pub const BLACK_KNIGHT: Self = Self::black(Knight);
    pub const BLACK_BISHOP: Self = Self::black(Bishop);
    pub const BLACK_ROOK: Self = Self::black(Rook);
    pub const BLACK_QUEEN: Self = Self::black(Queen);
    pub const BLACK_KING: Self = Self::black(King);

    pub const fn new(player: Player, kind: PieceKind) -> Self {
        Self { kind, player }
    }

    const fn white(kind: PieceKind) -> Self {
        Self::new(White, kind)
    }

    const fn black(kind: PieceKind) -> Self {
        Self::new(Black, kind)
    }

    const fn idx(self) -> usize {
        self.kind as usize + PieceKind::N * self.player as usize
    }

    pub fn char(&self) -> char {
        match self.kind {
            Pawn => match self.player {
                White => '♟',
                Black => '♙',
            },
            Knight => match self.player {
                White => '♞',
                Black => '♘',
            },
            Bishop => match self.player {
                White => '♝',
                Black => '♗',
            },
            Rook => match self.player {
                White => '♜',
                Black => '♖',
            },
            Queen => match self.player {
                White => '♛',
                Black => '♕',
            },
            King => match self.player {
                White => '♚',
                Black => '♔',
            },
        }
    }
}

impl<T> std::ops::Index<Piece> for [T; Piece::N] {
    type Output = T;

    fn index(&self, piece: Piece) -> &Self::Output {
        unsafe { self.get_unchecked(piece.idx()) }
    }
}

impl<T> std::ops::IndexMut<Piece> for [T; Piece::N] {
    fn index_mut(&mut self, piece: Piece) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(piece.idx()) }
    }
}
