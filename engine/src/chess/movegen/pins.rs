use crate::chess::{
    bitboard::Bitboard,
    board::Board,
    movegen::{
        tables,
        tables::{bishop_attacks, rook_attacks},
    },
    player::Player,
    square::Square,
};

pub fn get_pins(board: &Board, player: Player, king_square: Square) -> (Bitboard, Bitboard) {
    let all_pieces = board.occupancy();
    let our_pieces = board.occupancy_for(player);
    let them = player.other();

    let mut orthogonal_pins = Bitboard::EMPTY;
    let potential_orthogonal_pinned = rook_attacks(king_square, all_pieces) & our_pieces;
    let without_potential_orthogonal_pinned_pieces = all_pieces & !potential_orthogonal_pinned;
    let orthogonal_pinners = rook_attacks(king_square, without_potential_orthogonal_pinned_pieces)
        & board.orthogonal_sliders(them);

    let mut diagonal_pins = Bitboard::EMPTY;
    let potential_diagonal_pinned = bishop_attacks(king_square, all_pieces) & our_pieces;
    let without_potential_diagonal_pinned_pieces = all_pieces & !potential_diagonal_pinned;
    let diagonal_pinners = bishop_attacks(king_square, without_potential_diagonal_pinned_pieces)
        & board.diagonal_sliders(them);

    for pinner in orthogonal_pinners {
        orthogonal_pins.set(pinner);
        orthogonal_pins |= tables::between(king_square, pinner);
    }

    for pinner in diagonal_pinners {
        diagonal_pins.set(pinner);
        diagonal_pins |= tables::between(king_square, pinner);
    }

    (orthogonal_pins, diagonal_pins)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::{bitboard::bitboards::*, game::Game};

    fn pin_test(
        fen: &'static str,
        expected_orthogonal_pins: Bitboard,
        expected_diagonal_pins: Bitboard,
    ) {
        crate::init();
        let game = Game::from_fen(fen).unwrap();

        let king_square = game.board.king(game.player).single();
        let (orthogonal_pins, diagonal_pins) = get_pins(&game.board, game.player, king_square);

        assert_eq!(orthogonal_pins, expected_orthogonal_pins);
        assert_eq!(diagonal_pins, expected_diagonal_pins);
    }

    #[test]
    fn test_pin_in_gist_8_depth_3() {
        pin_test(
            "rnbq1k1r/pp1P1ppp/2p5/8/1bB5/8/PPPNNnPP/R1BQK2R w KQ - 3 9",
            Bitboard::EMPTY,
            B4_BB | C3_BB | D2_BB,
        );
    }
}
