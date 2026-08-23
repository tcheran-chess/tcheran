use crate::chess::prelude::*;

pub fn perft(depth: u8, game: &mut Game) -> usize {
    if depth == 1 {
        return game.moves().len();
    }

    game.moves()
        .iter()
        .map(|m| {
            game.make_move(*m);
            let result = perft(depth - 1, game);
            game.undo_move();
            result
        })
        .sum()
}

pub fn perft_div(depth: u8, game: &mut Game) -> Vec<(Move, usize)> {
    let root_moves = game.moves();

    let mut perft_for_moves: Vec<(Move, usize)> = vec![];

    if depth == 1 {
        for &mv in &root_moves {
            perft_for_moves.push((mv, 1));
        }

        return perft_for_moves;
    }

    for &mv in &root_moves {
        game.make_move(mv);
        let number_for_mv = perft(depth - 1, game);
        game.undo_move();
        perft_for_moves.push((mv, number_for_mv));
    }

    perft_for_moves
}
