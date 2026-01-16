use crate::chess::{
    bitboard::Bitboard,
    moves::Move,
    piece::{Piece, PieceKind},
    player::Player,
    square::Square,
};

#[derive(Clone)]
pub struct Board {
    pieces: [Bitboard; PieceKind::N],
    colors: [Bitboard; Player::N],
    squares: [Option<Piece>; Square::N],
}

impl Board {
    #[inline(always)]
    pub fn occupancy(&self) -> Bitboard {
        self.occupancy_for(Player::White) | self.occupancy_for(Player::Black)
    }

    #[inline(always)]
    pub fn occupancy_for(&self, player: Player) -> Bitboard {
        self.colors[player]
    }

    #[inline(always)]
    pub fn pieces_of_kind(&self, kind: PieceKind, player: Player) -> Bitboard {
        self.pieces[kind] & self.occupancy_for(player)
    }

    pub fn pawns(&self, player: Player) -> Bitboard {
        self.all_pawns() & self.occupancy_for(player)
    }

    pub fn all_pawns(&self) -> Bitboard {
        self.pieces[PieceKind::Pawn]
    }

    pub fn knights(&self, player: Player) -> Bitboard {
        self.all_knights() & self.occupancy_for(player)
    }

    pub fn all_knights(&self) -> Bitboard {
        self.pieces[PieceKind::Knight]
    }

    pub fn bishops(&self, player: Player) -> Bitboard {
        self.all_bishops() & self.occupancy_for(player)
    }

    pub fn all_bishops(&self) -> Bitboard {
        self.pieces[PieceKind::Bishop]
    }

    pub fn rooks(&self, player: Player) -> Bitboard {
        self.all_rooks() & self.occupancy_for(player)
    }

    pub fn all_rooks(&self) -> Bitboard {
        self.pieces[PieceKind::Rook]
    }

    pub fn queens(&self, player: Player) -> Bitboard {
        self.all_queens() & self.occupancy_for(player)
    }

    pub fn all_queens(&self) -> Bitboard {
        self.pieces[PieceKind::Queen]
    }

    pub fn king(&self, player: Player) -> Bitboard {
        self.all_kings() & self.occupancy_for(player)
    }

    pub fn king_square(&self, player: Player) -> Square {
        self.king(player).single()
    }

    pub fn all_kings(&self) -> Bitboard {
        self.pieces[PieceKind::King]
    }

    pub fn diagonal_sliders(&self, player: Player) -> Bitboard {
        self.bishops(player) | self.queens(player)
    }

    pub fn all_diagonal_sliders(&self) -> Bitboard {
        self.all_bishops() | self.all_queens()
    }

    pub fn orthogonal_sliders(&self, player: Player) -> Bitboard {
        self.rooks(player) | self.queens(player)
    }

    pub fn all_orthogonal_sliders(&self) -> Bitboard {
        self.all_rooks() | self.all_queens()
    }

    #[inline(always)]
    pub fn piece_at(&self, square: Square) -> Option<Piece> {
        self.squares[square]
    }

    #[inline(always)]
    pub fn piece_guaranteed_at(&self, square: Square) -> Piece {
        self.piece_at(square).unwrap()
    }

    pub fn captured_piece(&self, mv: Move) -> Option<PieceKind> {
        if !mv.is_capture() {
            return None;
        }

        if mv.is_en_passant() {
            return Some(PieceKind::Pawn);
        }

        Some(self.piece_guaranteed_at(mv.dst()).kind)
    }

    #[inline(always)]
    pub fn remove_at(&mut self, square: Square) {
        let piece = self.piece_guaranteed_at(square);
        self.pieces[piece.kind].unset(square);
        self.colors[piece.player].unset(square);
        self.squares[square] = None;
    }

    #[inline(always)]
    pub fn set_at(&mut self, square: Square, piece: Piece) {
        self.pieces[piece.kind].set(square);
        self.colors[piece.player].set(square);
        self.squares[square] = Some(piece);
    }
}

impl std::fmt::Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{}\n",
            (0..8)
                .rev()
                .map(|rank| {
                    (0..8)
                        .map(|file| match self.piece_at(Square::from_idxs(file, rank)) {
                            Some(piece) => piece.char().to_string(),
                            None => ".".to_string(),
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

impl TryFrom<[Option<Piece>; Square::N]> for Board {
    type Error = ();

    fn try_from(squares: [Option<Piece>; Square::N]) -> Result<Self, ()> {
        let mut board = Self {
            pieces: [Bitboard::EMPTY; PieceKind::N],
            colors: [Bitboard::EMPTY; Player::N],
            squares: [None; Square::N],
        };

        for (i, maybe_piece) in squares.into_iter().enumerate() {
            let Some(piece) = maybe_piece else {
                continue;
            };

            let square = Square::from_index(i.try_into().unwrap());
            board.set_at(square, piece);
        }

        Ok(board)
    }
}
