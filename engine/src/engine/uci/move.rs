use crate::chess::{
    game::Game,
    moves::Move,
    piece::PromotionPieceKind,
    player::Player,
    square::{Square, squares},
};

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct UciMove {
    pub src: Square,
    pub dst: Square,
    pub promotion: Option<PromotionPieceKind>,
}

impl UciMove {
    pub fn notation(self) -> String {
        format!(
            "{}{}{}",
            self.src.notation(),
            self.dst.notation(),
            match self.promotion {
                Some(piece) => match piece {
                    PromotionPieceKind::Knight => "n",
                    PromotionPieceKind::Bishop => "b",
                    PromotionPieceKind::Rook => "r",
                    PromotionPieceKind::Queen => "q",
                },
                None => "",
            }
        )
    }

    pub fn find_in_game(&self, game: &Game) -> Move {
        let is_frc = game.is_frc;

        for &mv in &game.moves() {
            // If the move is castling, it's represented internally as 'captures rook'
            // in standard chess, so account for that.
            if !is_frc && mv.is_castling() {
                if self.dst == squares::kingside_king_castle_end(Player::White)
                    && mv.dst() == squares::WHITE_STANDARD_KINGSIDE_ROOK_START
                {
                    return mv;
                }

                if self.dst == squares::queenside_king_castle_end(Player::White)
                    && mv.dst() == squares::WHITE_STANDARD_QUEENSIDE_ROOK_START
                {
                    return mv;
                }

                if self.dst == squares::kingside_king_castle_end(Player::Black)
                    && mv.dst() == squares::BLACK_STANDARD_KINGSIDE_ROOK_START
                {
                    return mv;
                }

                if self.dst == squares::queenside_king_castle_end(Player::Black)
                    && mv.dst() == squares::BLACK_STANDARD_QUEENSIDE_ROOK_START
                {
                    return mv;
                }
            }

            if mv.src() == self.src && mv.dst() == self.dst && mv.promotion() == self.promotion {
                return mv;
            }
        }

        panic!("Illegal move")
    }

    pub fn from_move(mv: Move, is_frc: bool) -> Self {
        let mut dst = mv.dst();

        if !is_frc && mv.is_castling() {
            dst = match mv.dst() {
                squares::WHITE_STANDARD_KINGSIDE_ROOK_START => {
                    squares::kingside_king_castle_end(Player::White)
                }
                squares::WHITE_STANDARD_QUEENSIDE_ROOK_START => {
                    squares::queenside_king_castle_end(Player::White)
                }
                squares::BLACK_STANDARD_KINGSIDE_ROOK_START => {
                    squares::kingside_king_castle_end(Player::Black)
                }
                squares::BLACK_STANDARD_QUEENSIDE_ROOK_START => {
                    squares::queenside_king_castle_end(Player::Black)
                }
                _ => dst,
            };
        }

        Self {
            src: mv.src(),
            dst,
            promotion: mv.promotion(),
        }
    }
}

impl std::fmt::Debug for UciMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl std::fmt::Display for UciMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}
