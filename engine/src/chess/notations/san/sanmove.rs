use crate::chess::{CastleSide, notations::san::constants, prelude::*};

#[derive(Debug)]
pub enum SanAmbiguityResolution {
    File(File),
    Rank(Rank),
    Square(Square),
}

#[derive(Debug)]
pub enum SanFrom {
    Pawn(File),
    NonPawn(PieceKind),
}

#[derive(Debug)]
pub enum SanStatus {
    Check,
    Checkmate,
}

#[derive(Debug)]
pub enum SanMove {
    Move {
        from: SanFrom,
        ambiguity_resolution: Option<SanAmbiguityResolution>,
        to: Square,
        promotion: Option<PromotionPieceKind>,

        is_capture: bool,
        status: Option<SanStatus>,
    },
    Castle(CastleSide),
}

impl std::fmt::Display for SanMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Move {
                from,
                ambiguity_resolution,
                to,
                promotion,
                is_capture,
                status,
            } => {
                match from {
                    SanFrom::Pawn(file) => {
                        if *is_capture {
                            write!(f, "{}", file.notation())?;
                        }
                    }
                    SanFrom::NonPawn(kind) => write!(
                        f,
                        "{}",
                        match kind {
                            Knight => "N",
                            Bishop => "B",
                            Rook => "R",
                            Queen => "Q",
                            King => "K",
                            _ => unreachable!(),
                        }
                    )?,
                }

                if let Some(ambiguity_resolution) = ambiguity_resolution {
                    match ambiguity_resolution {
                        SanAmbiguityResolution::File(file) => {
                            write!(f, "{}", file.notation())?;
                        }
                        SanAmbiguityResolution::Rank(rank) => {
                            write!(f, "{}", rank.notation())?;
                        }
                        SanAmbiguityResolution::Square(square) => {
                            write!(f, "{}", square.notation())?;
                        }
                    }
                }

                if *is_capture {
                    write!(f, "x")?;
                }

                write!(f, "{}", to.notation())?;

                if let Some(promotion) = promotion {
                    write!(
                        f,
                        "={}",
                        match promotion {
                            PromotionPieceKind::Knight => "N",
                            PromotionPieceKind::Bishop => "B",
                            PromotionPieceKind::Rook => "R",
                            PromotionPieceKind::Queen => "Q",
                        }
                    )?;
                }

                if let Some(status) = status {
                    write!(
                        f,
                        "{}",
                        match status {
                            SanStatus::Check => "+",
                            SanStatus::Checkmate => "#",
                        }
                    )?;
                }
            }
            Self::Castle(side) => match side {
                CastleSide::Kingside => write!(f, "{}", constants::KINGSIDE_CASTLE)?,
                CastleSide::Queenside => write!(f, "{}", constants::QUEENSIDE_CASTLE)?,
            },
        }

        Ok(())
    }
}
