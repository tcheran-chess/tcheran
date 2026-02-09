use engine::chess::{moves::Move, piece::PromotionPieceKind, square::Square};
use viriformat::chess::{
    chessmove::Move as ViriMove, piece::PieceType as ViriPieceType, types::Square as ViriSquare,
};

pub trait ToViriExt {
    type Output;

    fn to_viri(self) -> Self::Output;
}

impl ToViriExt for Move {
    type Output = ViriMove;

    fn to_viri(self) -> Self::Output {
        use viriformat::chess::chessmove::MoveFlags;

        let from = self.src().to_viri();
        let to = self.dst().to_viri();

        if let Some(promo_piece) = self.promotion() {
            ViriMove::new_with_promo(from, to, promo_piece.to_viri())
        } else if self.is_castling() {
            ViriMove::new_with_flags(from, to, MoveFlags::Castle)
        } else if self.is_en_passant() {
            ViriMove::new_with_flags(from, to, MoveFlags::EnPassant)
        } else {
            ViriMove::new(from, to)
        }
    }
}

impl ToViriExt for Square {
    type Output = ViriSquare;

    fn to_viri(self) -> Self::Output {
        ViriSquare::new(self.idx()).expect("Should be a valid square")
    }
}

impl ToViriExt for PromotionPieceKind {
    type Output = ViriPieceType;

    fn to_viri(self) -> Self::Output {
        match self {
            Self::Knight => ViriPieceType::Knight,
            Self::Bishop => ViriPieceType::Bishop,
            Self::Rook => ViriPieceType::Rook,
            Self::Queen => ViriPieceType::Queen,
        }
    }
}
