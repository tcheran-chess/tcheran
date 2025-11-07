use crate::chess::{
    bitboard::Bitboard,
    movegen,
    piece::{Piece, PieceKind},
    player::{ByPlayer, Player},
    square::Square,
};

#[derive(Clone)]
pub struct Board {
    pieces: [Bitboard; PieceKind::N],
    colors: ByPlayer<Bitboard>,
    squares: [Option<Piece>; Square::N],
}

impl Board {
    #[inline(always)]
    pub fn occupancy(&self) -> Bitboard {
        self.occupancy_for(Player::White) | self.occupancy_for(Player::Black)
    }

    #[inline(always)]
    pub fn occupancy_for(&self, player: Player) -> Bitboard {
        *self.colors.for_player(player)
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

    #[inline(always)]
    pub fn remove_at(&mut self, square: Square) {
        let piece = self.piece_guaranteed_at(square);
        self.pieces[piece.kind] ^= square.bb();
        self.colors
            .for_player_mut(piece.player)
            .unset_inplace(square);
        self.squares[square] = None;
    }

    #[inline(always)]
    pub fn set_at(&mut self, square: Square, piece: Piece) {
        self.pieces[piece.kind] |= square.bb();
        self.colors.for_player_mut(piece.player).set_inplace(square);
        self.squares[square] = Some(piece);
    }

    pub fn king_in_check(&self, player: Player) -> bool {
        let king = self.king(player).single();
        let enemy_attackers = movegen::generate_attackers_of(self, player, king);
        enemy_attackers.any()
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
                            Some(Piece { player, kind }) => match kind {
                                PieceKind::Pawn => match player {
                                    Player::White => "♟",
                                    Player::Black => "♙",
                                },
                                PieceKind::Knight => match player {
                                    Player::White => "♞",
                                    Player::Black => "♘",
                                },
                                PieceKind::Bishop => match player {
                                    Player::White => "♝",
                                    Player::Black => "♗",
                                },
                                PieceKind::Rook => match player {
                                    Player::White => "♜",
                                    Player::Black => "♖",
                                },
                                PieceKind::Queen => match player {
                                    Player::White => "♛",
                                    Player::Black => "♕",
                                },
                                PieceKind::King => match player {
                                    Player::White => "♚",
                                    Player::Black => "♔",
                                },
                            },
                            None => ".",
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
        let mut white_pawns = Bitboard::EMPTY;
        let mut white_knights = Bitboard::EMPTY;
        let mut white_bishops = Bitboard::EMPTY;
        let mut white_rooks = Bitboard::EMPTY;
        let mut white_queens = Bitboard::EMPTY;
        let mut white_king = Bitboard::EMPTY;

        let mut black_pawns = Bitboard::EMPTY;
        let mut black_knights = Bitboard::EMPTY;
        let mut black_bishops = Bitboard::EMPTY;
        let mut black_rooks = Bitboard::EMPTY;
        let mut black_queens = Bitboard::EMPTY;
        let mut black_king = Bitboard::EMPTY;

        for (i, maybe_piece) in squares.iter().enumerate() {
            if let Some(p) = maybe_piece {
                let square = Square::from_index(i.try_into().unwrap()).bb();

                match *p {
                    Piece::WHITE_PAWN => white_pawns |= square,
                    Piece::WHITE_KNIGHT => white_knights |= square,
                    Piece::WHITE_BISHOP => white_bishops |= square,
                    Piece::WHITE_ROOK => white_rooks |= square,
                    Piece::WHITE_QUEEN => white_queens |= square,
                    Piece::WHITE_KING => white_king |= square,

                    Piece::BLACK_PAWN => black_pawns |= square,
                    Piece::BLACK_KNIGHT => black_knights |= square,
                    Piece::BLACK_BISHOP => black_bishops |= square,
                    Piece::BLACK_ROOK => black_rooks |= square,
                    Piece::BLACK_QUEEN => black_queens |= square,
                    Piece::BLACK_KING => black_king |= square,
                }
            }
        }

        let pawns = white_pawns | black_pawns;
        let knights = white_knights | black_knights;
        let bishops = white_bishops | black_bishops;
        let rooks = white_rooks | black_rooks;
        let queens = white_queens | black_queens;
        let kings = white_king | black_king;

        let white_occupancy =
            white_pawns | white_knights | white_bishops | white_rooks | white_queens | white_king;

        let black_occupancy =
            black_pawns | black_knights | black_bishops | black_rooks | black_queens | black_king;

        Ok(Self {
            pieces: [pawns, knights, bishops, rooks, queens, kings],
            colors: ByPlayer::new(white_occupancy, black_occupancy),
            squares,
        })
    }
}
