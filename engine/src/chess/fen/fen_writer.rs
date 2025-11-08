use crate::chess::{
    board::Board,
    game::Game,
    piece::Piece,
    square::{FILES, RANKS, Square},
};

const fn format_piece(piece: Piece) -> char {
    match piece {
        Piece::WHITE_ROOK => 'R',
        Piece::WHITE_KNIGHT => 'N',
        Piece::WHITE_BISHOP => 'B',
        Piece::WHITE_QUEEN => 'Q',
        Piece::WHITE_KING => 'K',
        Piece::WHITE_PAWN => 'P',
        Piece::BLACK_ROOK => 'r',
        Piece::BLACK_KNIGHT => 'n',
        Piece::BLACK_BISHOP => 'b',
        Piece::BLACK_QUEEN => 'q',
        Piece::BLACK_KING => 'k',
        Piece::BLACK_PAWN => 'p',
    }
}

fn format_rank(rank: &[Option<Piece>]) -> String {
    let state = rank.iter().fold(
        (String::new(), 0),
        |(str_so_far, prev_empty_squares), piece| {
            if let Some(p) = piece {
                let new_string = format!(
                    "{}{}{}",
                    str_so_far,
                    if prev_empty_squares > 0 {
                        prev_empty_squares.to_string()
                    } else {
                        String::new()
                    },
                    format_piece(*p)
                );

                (new_string, 0)
            } else {
                (str_so_far, prev_empty_squares + 1)
            }
        },
    );

    let (str_so_far, prev_empty_squares) = state;
    format!(
        "{}{}",
        str_so_far,
        if prev_empty_squares > 0 {
            prev_empty_squares.to_string()
        } else {
            String::new()
        }
    )
}

fn format_board(board: &Board) -> String {
    RANKS
        .into_iter()
        .rev()
        .map(|r| {
            FILES
                .into_iter()
                .map(|f| board.piece_at(Square::from_file_and_rank(f, r)))
                .collect::<Vec<_>>()
        })
        .map(|r| format_rank(&r))
        .collect::<Vec<String>>()
        .join("/")
}

fn format_current_player(game: &Game) -> String {
    match game.player {
        crate::chess::player::Player::White => "w".to_string(),
        crate::chess::player::Player::Black => "b".to_string(),
    }
}

fn format_castle_rights(game: &Game) -> String {
    let [white_castle_rights, black_castle_rights] = game.castle_rights;

    match (
        white_castle_rights.king_side,
        white_castle_rights.queen_side,
        black_castle_rights.king_side,
        black_castle_rights.queen_side,
    ) {
        (false, false, false, false) => "-".to_string(),
        (white_king, white_queen, black_king, black_queen) => format!(
            "{}{}{}{}",
            if white_king { "K" } else { "" },
            if white_queen { "Q" } else { "" },
            if black_king { "k" } else { "" },
            if black_queen { "q" } else { "" }
        ),
    }
}

fn format_en_passant_target(game: &Game) -> String {
    match game.en_passant_target {
        Some(sq) => sq.notation(),
        None => "-".to_string(),
    }
}

fn format_halfmove_clock(game: &Game) -> String {
    game.halfmove_clock.to_string()
}

fn format_fullmove_number(game: &Game) -> String {
    game.turn().to_string()
}

pub fn write(game: &Game) -> String {
    format!(
        "{} {} {} {} {} {}",
        format_board(&game.board),
        format_current_player(game),
        format_castle_rights(game),
        format_en_passant_target(game),
        format_halfmove_clock(game),
        format_fullmove_number(game),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_startpos() {
        crate::init();

        assert_eq!(
            Game::new().to_fen(),
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
        );
    }
}
