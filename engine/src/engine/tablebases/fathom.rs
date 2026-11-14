use std::mem::MaybeUninit;

use crate::{
    chess::{moves::MoveListExt, prelude::*},
    engine::{eval::Eval, search::RootMove, tablebases::bindings},
};

pub fn tb_init(path: &str) -> bool {
    let path = std::ffi::CString::new(path).unwrap();
    unsafe { bindings::tb_init(path.as_ptr()) }
}

pub fn n_men() -> usize {
    unsafe { bindings::TB_LARGEST as usize }
}

pub fn tb_probe_root_dtz(game: &Game, has_repeated_position: bool) -> Option<Vec<RootMove>> {
    let mut c_moves: MaybeUninit<bindings::TbRootMoves> = MaybeUninit::uninit();

    let dtz_found = unsafe {
        bindings::tb_probe_root_dtz(
            game.board.occupancy_for(White).raw(),
            game.board.occupancy_for(Black).raw(),
            game.board.all_kings().raw(),
            game.board.all_queens().raw(),
            game.board.all_rooks().raw(),
            game.board.all_bishops().raw(),
            game.board.all_knights().raw(),
            game.board.all_pawns().raw(),
            game.halfmove_clock,
            0,
            ep_square(game),
            game.player == White,
            has_repeated_position,
            true,
            c_moves.as_mut_ptr(),
        )
    } != 0;

    if !dtz_found {
        return None;
    }

    let tb_moves = unsafe { c_moves.assume_init() };
    assert_ne!(tb_moves.size, 0, "Unexpected 0 tb moves in position {}", game.to_fen());

    Some(to_root_moves(game, &tb_moves))
}

pub fn tb_probe_root_wdl(game: &Game) -> Option<Vec<RootMove>> {
    let mut c_moves: MaybeUninit<bindings::TbRootMoves> = MaybeUninit::uninit();

    let wdl_found = unsafe {
        bindings::tb_probe_root_wdl(
            game.board.occupancy_for(White).raw(),
            game.board.occupancy_for(Black).raw(),
            game.board.all_kings().raw(),
            game.board.all_queens().raw(),
            game.board.all_rooks().raw(),
            game.board.all_bishops().raw(),
            game.board.all_knights().raw(),
            game.board.all_pawns().raw(),
            game.halfmove_clock,
            0,
            ep_square(game),
            game.player == White,
            true,
            c_moves.as_mut_ptr(),
        ) != 0
    };

    if !wdl_found {
        return None;
    }

    let tb_moves = unsafe { c_moves.assume_init() };
    assert_ne!(tb_moves.size, 0, "Unexpected 0 tb moves in position {}", game.to_fen());

    Some(to_root_moves(game, &tb_moves))
}

pub fn tb_probe_wdl(game: &Game) -> Option<Outcome> {
    unsafe {
        let wdl = bindings::tb_probe_wdl(
            game.board.occupancy_for(White).raw(),
            game.board.occupancy_for(Black).raw(),
            game.board.all_kings().raw(),
            game.board.all_queens().raw(),
            game.board.all_rooks().raw(),
            game.board.all_bishops().raw(),
            game.board.all_knights().raw(),
            game.board.all_pawns().raw(),
            0,
            0,
            ep_square(game),
            game.player == White,
        );

        to_outcome(wdl)
    }
}

fn ep_square(game: &Game) -> u32 {
    game.en_passant_target.map_or(0, |ep| u32::from(ep.raw()))
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

fn to_root_moves(game: &Game, tb_moves: &bindings::TbRootMoves) -> Vec<RootMove> {
    (0..tb_moves.size as usize)
        .map(|i| tb_moves.moves[i])
        .map(|m| to_root_move(game, &m))
        .collect()
}

fn to_root_move(game: &Game, tb: &bindings::TbRootMove) -> RootMove {
    let tb_mv = u32::from(tb.move_);

    let to_bits = tb_mv & 0x3f;
    let from_bits = (tb_mv >> 6) & 0x3f;
    let promotion_bits = (tb_mv >> 12) & 0x7;

    let from = Square::from_raw(from_bits as u8);
    let to = Square::from_raw(to_bits as u8);

    let promotion = match promotion_bits {
        bindings::TB_PROMOTES_QUEEN => Some(PromotionPieceKind::Queen),
        bindings::TB_PROMOTES_ROOK => Some(PromotionPieceKind::Rook),
        bindings::TB_PROMOTES_BISHOP => Some(PromotionPieceKind::Bishop),
        bindings::TB_PROMOTES_KNIGHT => Some(PromotionPieceKind::Knight),
        _ => None,
    };

    RootMove {
        mv: game
            .moves()
            .expect_matching_with_promotion(from, to, promotion),

        tb_score: to_eval(tb.tbScore),
        tb_rank: tb.tbRank,
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
