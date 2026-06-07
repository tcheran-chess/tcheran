use crate::chess::{Game, Player, PromotionPieceKind, Square, moves::Move, squares};

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct UciMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PromotionPieceKind>,
}

impl UciMove {
    pub fn notation(self) -> String {
        format!(
            "{}{}{}",
            self.from.notation(),
            self.to.notation(),
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
                if self.to == squares::kingside_king_castle_end(Player::White)
                    && mv.to() == squares::WHITE_STANDARD_KINGSIDE_ROOK_START
                {
                    return mv;
                }

                if self.to == squares::queenside_king_castle_end(Player::White)
                    && mv.to() == squares::WHITE_STANDARD_QUEENSIDE_ROOK_START
                {
                    return mv;
                }

                if self.to == squares::kingside_king_castle_end(Player::Black)
                    && mv.to() == squares::BLACK_STANDARD_KINGSIDE_ROOK_START
                {
                    return mv;
                }

                if self.to == squares::queenside_king_castle_end(Player::Black)
                    && mv.to() == squares::BLACK_STANDARD_QUEENSIDE_ROOK_START
                {
                    return mv;
                }
            }

            if mv.from() == self.from && mv.to() == self.to && mv.promotion() == self.promotion {
                return mv;
            }
        }

        panic!("Illegal move")
    }

    pub fn from_move(mv: Move, is_frc: bool) -> Self {
        let mut to = mv.to();

        if !is_frc && mv.is_castling() {
            to = match mv.to() {
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
                _ => to,
            };
        }

        Self {
            from: mv.from(),
            to,
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
