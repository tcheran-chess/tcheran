use std::{cmp::Reverse, collections::HashSet, mem::MaybeUninit};

#[cfg(feature = "syzygy")]
use crate::engine::tablebases::bindings;
use crate::{
    chess::{MAX_LEGAL_MOVES, moves::MoveListExt, prelude::*},
    engine::{
        eval::Eval,
        search::{RootMove, RootTbInfo},
        tablebases::bindings::{TbRootMove, TbRootMoves},
    },
    util::arrayvec::ArrayVec,
};

#[derive(Clone)]
pub struct Tablebase {
    pub is_enabled: bool,
    pub n_men: u8,
}

#[cfg(feature = "syzygy")]
impl Tablebase {
    pub fn new() -> Self {
        Self {
            is_enabled: false,
            n_men: 0,
        }
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
        self.n_men = n_men as u8;
    }

    pub fn can_probe(&self, game: &Game) -> bool {
        if !self.is_enabled {
            return false;
        }

        if game.board.occupancy().count() > self.n_men {
            return false;
        }

        true
    }

    pub fn n_men(&self) -> u8 {
        self.n_men
    }

    pub fn wdl(&self, game: &Game) -> Option<Outcome> {
        debug_assert!(self.is_enabled);

        if game.castle_rights[White].any() || game.castle_rights[Black].any() {
            return None;
        }

        if game.halfmove_clock != 0 {
            return None;
        }

        let ep_square = game.en_passant_target.map_or(0, |ep| u32::from(ep.idx()));

        unsafe {
            let wdl = bindings::tb_probe_wdl(
                game.board.occupancy_for(White).as_u64(),
                game.board.occupancy_for(Black).as_u64(),
                game.board.all_kings().as_u64(),
                game.board.all_queens().as_u64(),
                game.board.all_rooks().as_u64(),
                game.board.all_bishops().as_u64(),
                game.board.all_knights().as_u64(),
                game.board.all_pawns().as_u64(),
                0,
                0,
                ep_square,
                game.player == White,
            );

            Self::to_outcome(wdl)
        }
    }

    pub fn root_wdl_dtz(&self, game: &Game) -> Option<RootTbInfo> {
        debug_assert!(self.is_enabled);

        if game.castle_rights[White].any() || game.castle_rights[Black].any() {
            return None;
        }

        let mut c_moves: MaybeUninit<TbRootMoves> = MaybeUninit::uninit();

        let ep_square = game.en_passant_target.map_or(0, |ep| u32::from(ep.idx()));
        let has_repeated_position = Self::has_repeated_position(game);

        let dtz_found = unsafe {
            bindings::tb_probe_root_dtz(
                game.board.occupancy_for(White).as_u64(),
                game.board.occupancy_for(Black).as_u64(),
                game.board.all_kings().as_u64(),
                game.board.all_queens().as_u64(),
                game.board.all_rooks().as_u64(),
                game.board.all_bishops().as_u64(),
                game.board.all_knights().as_u64(),
                game.board.all_pawns().as_u64(),
                game.halfmove_clock,
                0,
                ep_square,
                game.player == White,
                has_repeated_position,
                true,
                c_moves.as_mut_ptr(),
            )
        } != 0;

        let wdl_found = if !dtz_found {
            unsafe {
                bindings::tb_probe_root_wdl(
                    game.board.occupancy_for(White).as_u64(),
                    game.board.occupancy_for(Black).as_u64(),
                    game.board.all_kings().as_u64(),
                    game.board.all_queens().as_u64(),
                    game.board.all_rooks().as_u64(),
                    game.board.all_bishops().as_u64(),
                    game.board.all_knights().as_u64(),
                    game.board.all_pawns().as_u64(),
                    game.halfmove_clock,
                    0,
                    ep_square,
                    game.player == White,
                    true,
                    c_moves.as_mut_ptr(),
                ) != 0
            }
        } else {
            false
        };

        if !dtz_found && !wdl_found {
            return None;
        }

        let tb_moves = unsafe { c_moves.assume_init() };

        assert_ne!(tb_moves.size, 0, "Unexpected 0 tb moves in position {}", game.to_fen());

        let mut root_moves_vec: Vec<_> = (0..tb_moves.size as usize)
            .map(|i| tb_moves.moves[i])
            .map(|m| Self::to_root_move(game, &m))
            .collect();

        root_moves_vec.sort_by_key(|m| Reverse(m.tb_rank));

        let best_rank = root_moves_vec[0].tb_rank;

        let mut root_moves = ArrayVec::<RootMove, MAX_LEGAL_MOVES>::new();
        for mv in root_moves_vec {
            if mv.tb_rank != best_rank {
                break;
            }

            root_moves.push(mv);
        }

        let probe_wdl = !dtz_found && root_moves.get(0).tb_score.is_win();

        Some(RootTbInfo {
            root_moves,
            probe_wdl,
        })
    }

    fn to_root_move(game: &Game, tb: &TbRootMove) -> RootMove {
        let tb_mv = u32::from(tb.move_);

        let to_bits = tb_mv & 0x3f;
        let from_bits = (tb_mv >> 6) & 0x3f;
        let promotion_bits = (tb_mv >> 12) & 0x7;

        let from = Square::from_index(from_bits as u8);
        let to = Square::from_index(to_bits as u8);

        let promotion = match promotion_bits {
            bindings::TB_PROMOTES_QUEEN => Some(PromotionPieceKind::Queen),
            bindings::TB_PROMOTES_ROOK => Some(PromotionPieceKind::Rook),
            bindings::TB_PROMOTES_BISHOP => Some(PromotionPieceKind::Bishop),
            bindings::TB_PROMOTES_KNIGHT => Some(PromotionPieceKind::Knight),
            _ => None,
        };

        RootMove {
            mv: game.moves().expect_matching(from, to, promotion),

            tb_score: Self::to_eval(tb.tbScore),
            tb_rank: tb.tbRank,
        }
    }

    fn has_repeated_position(game: &Game) -> bool {
        let mut seen = HashSet::new();
        seen.insert(game.hash);

        game.history
            .iter()
            .rev()
            .take(game.halfmove_clock as usize)
            .any(|position| !seen.insert(position.hash))
    }

    fn to_outcome(outcome: std::ffi::c_uint) -> Option<Outcome> {
        use Outcome::*;

        match outcome {
            bindings::TB_WIN => Some(Win),
            bindings::TB_LOSS => Some(Loss),
            bindings::TB_DRAW | bindings::TB_CURSED_WIN | bindings::TB_BLESSED_LOSS => Some(Draw),
            bindings::TB_RESULT_FAILED => None,
            _ => unreachable!(),
        }
    }

    fn to_eval(tb_score: i32) -> Eval {
        if tb_score > 30000 {
            return Eval::tb_mate_in(0);
        }

        if tb_score < -30000 {
            return Eval::tb_mated_in(0);
        }

        Eval::DRAW
    }
}

#[cfg(not(feature = "syzygy"))]
impl Tablebase {
    pub fn new() -> Self {
        Self {
            is_enabled: false,
            n_men: 0,
        }
    }

    pub fn set_paths(&mut self, _path: &str) {}

    pub fn can_probe(&self, _game: &Game) -> bool {
        false
    }

    pub fn n_men(&self) -> u8 {
        self.n_men
    }

    pub fn wdl(&self, _game: &Game) -> Option<Outcome> {
        None
    }

    pub fn best_move(&self, _game: &Game) -> Option<Move> {
        None
    }
}
