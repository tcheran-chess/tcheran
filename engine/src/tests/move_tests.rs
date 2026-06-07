use crate::{
    chess::{Game, Move, Square, squares::all::*},
    engine::{
        eval::Eval,
        search::{NullReporter, PersistentState, TimeControl, st_search, types::Depth},
    },
};

fn test_expected_move(fen: &str, depth: Depth, mv: (Square, Square)) -> (Move, Eval) {
    crate::init();
    let game = Game::from_fen(fen).unwrap();

    let (best_move, eval) =
        st_search(&game, &PersistentState::new(16), TimeControl::Depth(depth), &NullReporter);

    assert_eq!((best_move.from(), best_move.to()), mv);
    (best_move, eval)
}

#[test]
fn test_mate_on_100th_halfmove_detected() {
    let (_, eval) = test_expected_move(
        "4Q3/8/1p4pk/1PbB1p1p/7P/p3P1PK/P3qP2/8 w - - 99 88",
        Depth::new(5),
        (E8, H8),
    );

    assert_eq!(eval, Eval::mate_in(1));
}
