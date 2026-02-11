use crate::{
    chess::{
        game::Game,
        moves::Move,
        square::{Square, squares::all::*},
    },
    engine::{
        eval::Eval,
        options::EngineOptions,
        search::{
            NullReporter, PersistentState, TimeControl, search, time_control::StopControl,
            types::Depth,
        },
    },
};

fn test_expected_move(fen: &str, depth: Depth, mv: (Square, Square)) -> (Move, Eval) {
    crate::init();
    let game = Game::from_fen(fen).unwrap();
    let mut persistent_state = PersistentState::new(16);

    let (best_move, eval) = search(
        &game,
        &mut persistent_state,
        TimeControl::Depth(depth),
        StopControl::new(),
        &EngineOptions::DEFAULT,
        &NullReporter,
    );

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
