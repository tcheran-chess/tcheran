use crate::chess::{prelude::*, squares};

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

    pub fn find_in_game(&self, game: &Game) -> Option<Move> {
        game.moves()
            .into_iter()
            .find(|&&mv| *self == Self::from_move(mv, game.is_frc))
            .copied()
    }

    pub fn from_move(mv: Move, is_frc: bool) -> Self {
        let mut to = mv.to();

        if !is_frc && mv.is_castling() {
            // If the move is castling, it's represented internally as 'captures rook'
            // in standard chess, so account for that.
            to = match mv.to() {
                squares::WHITE_STANDARD_KINGSIDE_ROOK_START => {
                    squares::kingside_king_castle_end(White)
                }
                squares::WHITE_STANDARD_QUEENSIDE_ROOK_START => {
                    squares::queenside_king_castle_end(White)
                }
                squares::BLACK_STANDARD_KINGSIDE_ROOK_START => {
                    squares::kingside_king_castle_end(Black)
                }
                squares::BLACK_STANDARD_QUEENSIDE_ROOK_START => {
                    squares::queenside_king_castle_end(Black)
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
