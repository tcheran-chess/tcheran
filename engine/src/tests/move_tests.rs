use crate::{
    chess::prelude::*,
    engine::{
        eval::Eval,
        search::{
            NullReporter, PersistentState, SearchResult, TimeControl, st_search, types::Depth,
        },
    },
};

fn test_expected_move(fen: &str, depth: Depth, mv: (Square, Square)) -> SearchResult {
    crate::init();
    let game = Game::from_valid_fen(fen);

    let result =
        st_search(&game, &PersistentState::new(16), TimeControl::Depth(depth), &NullReporter);

    assert_eq!((result.mv.from(), result.mv.to()), mv);
    result
}

#[test]
fn test_mate_on_100th_halfmove_detected() {
    let result = test_expected_move(
        "4Q3/8/1p4pk/1PbB1p1p/7P/p3P1PK/P3qP2/8 w - - 99 88",
        Depth::new(5),
        (E8, H8),
    );

    assert_eq!(result.score, Eval::mate_in(1));
}
