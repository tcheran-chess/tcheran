use shakmaty::{KnownOutcome, Position};

use crate::Wdl;

pub trait SyzygyChess {
    type Game: SyzygyChessGame<Self::Move>;

    type Move: SyzygyChessMove;

    type PieceType: SyzygyChessPieceType;
    type Color: SyzygyChessColor;
}

pub trait SyzygyChessGame<Move: SyzygyChessMove> {
    fn outcome(&self) -> Option<Wdl>;
    fn play_unchecked(&mut self, mv: Move);
    fn board(&self);
}

pub trait SyzygyChessMove {}
pub trait SyzygyChessPieceType {
    const Pawn: Self;
}
pub trait SyzygyChessColor {}

pub struct ShakmatyChess;

impl SyzygyChess for ShakmatyChess {
    type Game = shakmaty::Chess;

    type Move = shakmaty::Move;

    type PieceType = shakmaty::Role;
    type Color = shakmaty::Color;
}

impl SyzygyChessGame for shakmaty::Chess {
    fn outcome(&self) -> Option<Wdl> {
        let outcome = self.variant_outcome().known();
        let Some(outcome) = outcome else {
            return None;
        };

        Some(match outcome {
            KnownOutcome::Draw => Wdl::Draw,
            KnownOutcome::Decisive { winner } if winner == self.turn() => Wdl::Win,
            KnownOutcome::Decisive { .. } => Wdl::Loss,
        })
    }

    fn play_unchecked(&mut self) {
        todo!()
    }

    fn board(&self) {
        todo!()
    }
}

impl SyzygyChessMove for shakmaty::Move {}
impl SyzygyChessPieceType for shakmaty::Role {
    const Pawn: Self = Self::Pawn;
}
impl SyzygyChessColor for shakmaty::Color {}
