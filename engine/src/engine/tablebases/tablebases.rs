use crate::chess::{Game, Move};
#[cfg(feature = "syzygy")]
use crate::engine::tablebases::bindings;

pub enum Wdl {
    Win,
    Draw,
    Loss,
}

#[derive(Clone)]
pub struct Tablebase {
    pub is_enabled: bool,
}

#[cfg(feature = "syzygy")]
impl Tablebase {
    pub fn new() -> Self {
        Self { is_enabled: false }
    }

    pub fn can_probe(&self, game: &Game) -> bool {
        if !self.is_enabled {
            return false;
        }

        if game.board.occupancy().count() > self.n_men() {
            return false;
        }

        true
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "n_men will be at most 7 as these are the largest syzygy tablebases"
    )]
    pub fn n_men(&self) -> u8 {
        if !self.is_enabled {
            return 0;
        }

        unsafe { bindings::TB_LARGEST as u8 }
    }

    pub fn set_paths(&mut self, path: &str) {
        let path = std::ffi::CString::new(path).unwrap();
        let was_set = unsafe { bindings::tb_init(path.as_ptr()) };
        let n_men = unsafe { bindings::TB_LARGEST as usize };

        assert!(
            was_set && n_men != 0,
            "Invalid tablebase path: {}",
            path.to_str().unwrap_or_default()
        );

        self.is_enabled = true;
    }

    pub fn wdl(&self, game: &Game) -> Option<Wdl> {
        if !self.is_enabled {
            return None;
        }

        unsafe {
            let wdl = bindings::tb_probe_wdl(
                game.board
                    .occupancy_for(crate::chess::Player::White)
                    .as_u64(),
                game.board
                    .occupancy_for(crate::chess::Player::Black)
                    .as_u64(),
                game.board.all_kings().as_u64(),
                game.board.all_queens().as_u64(),
                game.board.all_rooks().as_u64(),
                game.board.all_bishops().as_u64(),
                game.board.all_knights().as_u64(),
                game.board.all_pawns().as_u64(),
                0,
                0,
                0,
                game.player == crate::chess::Player::White,
            );

            Self::to_wdl(wdl)
        }
    }

    #[rustfmt::skip]
    pub fn best_move(&self, game: &Game) -> Option<Move> {
        use crate::chess::moves::MoveListExt;

        if !self.is_enabled {
            return None;
        }

        unsafe {
            let result = bindings::tb_probe_root(
                game.board.occupancy_for(crate::chess::Player::White).as_u64(),
                game.board.occupancy_for(crate::chess::Player::Black).as_u64(),
                game.board.all_kings().as_u64(),
                game.board.all_queens().as_u64(),
                game.board.all_rooks().as_u64(),
                game.board.all_bishops().as_u64(),
                game.board.all_knights().as_u64(),
                game.board.all_pawns().as_u64(),
                game.halfmove_clock,
                0,
                0,
                game.player == crate::chess::Player::White,
                std::ptr::null_mut(),
            );

            if result == bindings::TB_RESULT_FAILED {
                return None;
            }

            // let wdl_bits = result & bindings::TB_RESULT_WDL_MASK >> bindings::TB_RESULT_WDL_SHIFT;
            // let dtz_bits = (result & bindings::TB_RESULT_DTZ_MASK) >> bindings::TB_RESULT_DTZ_SHIFT;
            let from_bits =(result & bindings::TB_RESULT_FROM_MASK) >> bindings::TB_RESULT_FROM_SHIFT;
            let to_bits = (result & bindings::TB_RESULT_TO_MASK) >> bindings::TB_RESULT_TO_SHIFT;
            let promotion_bits = (result & bindings::TB_RESULT_PROMOTES_MASK) >> bindings::TB_RESULT_PROMOTES_SHIFT;

            let from = crate::chess::Square::from_index(from_bits as u8);
            let to = crate::chess::Square::from_index(to_bits as u8);

            let promotion = match promotion_bits {
                bindings::TB_PROMOTES_QUEEN => Some(crate::chess::PromotionPieceKind::Queen),
                bindings::TB_PROMOTES_ROOK => Some(crate::chess::PromotionPieceKind::Rook),
                bindings::TB_PROMOTES_BISHOP => Some(crate::chess::PromotionPieceKind::Bishop),
                bindings::TB_PROMOTES_KNIGHT => Some(crate::chess::PromotionPieceKind::Knight),
                _ => None,
            };

            // Note that _technically_ castling could be the best move in a 7-man position.
            // However, the syzygy tablebases discount this because it's effectively impossible in a
            // normal game, so the fact that expect_matching won't deal correctly with castling moves
            // where the destination square isn't 'captures rook' doesn't matter here.
            let matching_move = game.moves().expect_matching(from, to, promotion);

            Some(matching_move)
        }
    }

    fn to_wdl(outcome: std::ffi::c_uint) -> Option<Wdl> {
        use Wdl::*;

        match outcome {
            bindings::TB_WIN => Some(Win),
            bindings::TB_LOSS => Some(Loss),
            bindings::TB_DRAW | bindings::TB_CURSED_WIN | bindings::TB_BLESSED_LOSS => Some(Draw),
            bindings::TB_RESULT_FAILED => None,
            _ => unreachable!(),
        }
    }
}

#[cfg(not(feature = "syzygy"))]
impl Tablebase {
    pub fn new() -> Self {
        Self { is_enabled: false }
    }

    pub fn can_probe(&self, _game: &Game) -> bool {
        false
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "n_men will be at most 7 as these are the largest syzygy tablebases"
    )]
    pub fn n_men(&self) -> u8 {
        0
    }

    pub fn set_paths(&mut self, _path: &str) {}

    pub fn wdl(&self, _game: &Game) -> Option<Wdl> {
        None
    }

    pub fn best_move(&self, _game: &Game) -> Option<Move> {
        None
    }
}
