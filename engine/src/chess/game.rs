use crate::chess::{
    bitboard::{Bitboard, bitboards},
    board::Board,
    fen,
    movegen::{
        generate_legal_moves, tables,
        tables::{bishop_attacks, king_attacks, knight_attacks, pawn_attacks, rook_attacks},
    },
    moves::{Move, MoveList},
    piece::{Piece, PieceKind},
    player::Player,
    square::{Square, squares},
    zobrist,
    zobrist::ZobristHash,
};

#[derive(Debug, Copy, Clone)]
pub enum CastleRightsSide {
    Kingside,
    Queenside,
}

impl CastleRightsSide {
    pub const N: usize = 2;
}

#[derive(Copy, Clone, Debug)]
pub struct CastleRights {
    pub king_side: bool,
    pub queen_side: bool,
}

impl CastleRights {
    pub const fn none() -> Self {
        Self {
            king_side: false,
            queen_side: false,
        }
    }

    pub fn can_castle_to_side(self, side: CastleRightsSide) -> bool {
        match side {
            CastleRightsSide::Kingside => self.king_side,
            CastleRightsSide::Queenside => self.queen_side,
        }
    }

    pub fn remove_rights(&mut self, side: CastleRightsSide) {
        match side {
            CastleRightsSide::Kingside => self.king_side = false,
            CastleRightsSide::Queenside => self.queen_side = false,
        }
    }
}

impl Default for CastleRights {
    fn default() -> Self {
        Self {
            king_side: true,
            queen_side: true,
        }
    }
}

impl<T> std::ops::Index<CastleRightsSide> for [T; CastleRightsSide::N] {
    type Output = T;

    fn index(&self, index: CastleRightsSide) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> std::ops::IndexMut<CastleRightsSide> for [T; CastleRightsSide::N] {
    fn index_mut(&mut self, index: CastleRightsSide) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

#[derive(Debug, Clone)]
pub struct History {
    pub mv: Option<Move>,
    pub captured: Option<Piece>,
    pub castle_rights: [CastleRights; Player::N],
    pub en_passant_target: Option<Square>,
    pub halfmove_clock: u32,

    pub hash: ZobristHash,
    pub pawn_hash: ZobristHash,
    pub major_piece_hash: ZobristHash,
    pub minor_piece_hash: ZobristHash,
    pub non_pawn_hash: [ZobristHash; Player::N],

    pub checkers: Bitboard,
    pub orthogonal_pins: Bitboard,
    pub diagonal_pins: Bitboard,
    pub threats: Bitboard,
}

#[inline]
fn is_major_piece(piece: Piece) -> bool {
    [PieceKind::Rook, PieceKind::Queen, PieceKind::King].contains(&piece.kind)
}

#[inline]
fn is_minor_piece(piece: Piece) -> bool {
    [PieceKind::Knight, PieceKind::Bishop, PieceKind::King].contains(&piece.kind)
}

#[derive(Debug, Clone)]
pub struct Game {
    pub player: Player,
    pub board: Board,
    pub castle_rights: [CastleRights; Player::N],
    pub en_passant_target: Option<Square>,
    pub halfmove_clock: u32,
    pub plies: u32,

    pub hash: ZobristHash,
    pub pawn_hash: ZobristHash,
    pub major_piece_hash: ZobristHash,
    pub minor_piece_hash: ZobristHash,
    pub non_pawn_hash: [ZobristHash; Player::N],

    pub history: Vec<History>,

    pub checkers: Bitboard,
    pub orthogonal_pins: Bitboard,
    pub diagonal_pins: Bitboard,
    pub threats: Bitboard,
}

impl Game {
    pub fn new() -> Self {
        Self::from_fen(fen::START_POS).unwrap()
    }

    pub fn from_state(
        board: Board,
        player: Player,
        castle_rights: [CastleRights; Player::N],
        en_passant_target: Option<Square>,
        halfmove_clock: u32,
        plies: u32,
    ) -> Self {
        let mut game = Self {
            board,
            player,
            castle_rights,
            en_passant_target,
            halfmove_clock,
            plies,

            checkers: Bitboard::EMPTY,
            orthogonal_pins: Bitboard::EMPTY,
            diagonal_pins: Bitboard::EMPTY,
            threats: Bitboard::EMPTY,

            hash: ZobristHash::uninit(),
            pawn_hash: ZobristHash::uninit(),
            major_piece_hash: ZobristHash::uninit(),
            minor_piece_hash: ZobristHash::uninit(),
            non_pawn_hash: [ZobristHash::uninit(); Player::N],

            history: Vec::new(),
        };

        game.hash = zobrist::hash(&game);
        game.pawn_hash = zobrist::hash_pieces(&game, |p| p.kind == PieceKind::Pawn);
        game.major_piece_hash = zobrist::hash_pieces(&game, is_major_piece);
        game.minor_piece_hash = zobrist::hash_pieces(&game, is_minor_piece);
        game.non_pawn_hash = [
            zobrist::hash_pieces(&game, |p| p.player == Player::White),
            zobrist::hash_pieces(&game, |p| p.player == Player::Black),
        ];

        game.update_threats();
        game.update_checks_and_pins();

        game
    }

    pub fn from_fen(fen: &str) -> Result<Self, fen::ParseError> {
        fen::parse(fen)
    }

    pub fn to_fen(&self) -> String {
        fen::write(self)
    }

    pub fn turn(&self) -> u32 {
        self.plies / 2 + 1
    }

    #[inline]
    pub fn is_draw(&self, plies: u8) -> bool {
        self.is_repeated_position(plies)
            || self.is_stalemate_by_fifty_move_rule()
            || self.is_stalemate_by_insufficient_material()
    }

    pub fn is_stalemate_by_fifty_move_rule(&self) -> bool {
        if self.halfmove_clock >= 100 {
            let mut movelist = MoveList::new();
            generate_legal_moves(self, |m| movelist.push(m));
            return !movelist.is_empty();
        }

        false
    }

    pub fn is_repeated_position(&self, plies: u8) -> bool {
        let plies = plies as usize;

        let mut seen = false;
        for (plies_back, position) in self
            .history
            .iter()
            .rev()
            .enumerate()
            .take(self.halfmove_clock as usize)
            .skip(3)
            .step_by(2)
        {
            if position.hash == self.hash {
                // If the move happened before during search, use two-fold.
                if plies_back < plies {
                    return true;
                }

                // If we're before the start of the search, use three-fold.
                if seen {
                    return true;
                }

                seen = true;
            }
        }

        false
    }

    pub fn is_stalemate_by_insufficient_material(&self) -> bool {
        let all_pieces = self.board.occupancy();

        match all_pieces.count() {
            // King vs king is always a draw
            2 => true,

            // If the sole remaining non-king piece on the board is a knight or bishop,
            // it's a draw
            3 => (self.board.all_knights() | self.board.all_bishops()).any(),

            4 => {
                let player_pieces = self.board.occupancy_for(self.player);
                let knights = self.board.all_knights();
                let bishops = self.board.all_bishops();
                let kings = self.board.all_kings();

                let one_piece_each = player_pieces.count() == 2;

                let knight_count = knights.count();
                let bishop_count = bishops.count();
                let king_in_corner = (kings & bitboards::CORNERS).any();
                let king_on_edge = (kings & bitboards::EDGES).any();

                // This logic is from Carp
                (knight_count == 2 && !king_on_edge)
                    || (bishop_count == 2
                        && ((bishops & bitboards::LIGHT_SQUARES).count() != 1
                            || (one_piece_each && !king_in_corner)))
                    || (knight_count == 1 && bishop_count == 1 && one_piece_each && !king_in_corner)
            }
            _ => false,
        }
    }

    // Zugzwang is more likely to occur in King and Pawn endgames
    pub fn zugzwang_likely(&self) -> bool {
        let player = self.player;

        self.board.occupancy_for(player)
            == self.board.king_square(player).bb() | self.board.pawns(player)
    }

    #[inline(always)]
    pub fn is_king_in_check(&self) -> bool {
        self.checkers.any()
    }

    fn set_at(&mut self, sq: Square, piece: Piece) {
        self.board.set_at(sq, piece);
        self.toggle_piece_in_hashes(sq, piece);
    }

    fn remove_at(&mut self, sq: Square) -> Piece {
        let removed_piece = self.board.piece_guaranteed_at(sq);
        self.board.remove_at(sq);
        self.toggle_piece_in_hashes(sq, removed_piece);

        removed_piece
    }

    fn toggle_piece_in_hashes(&mut self, sq: Square, piece: Piece) {
        self.hash.toggle_piece_on_square(sq, piece);

        if piece.kind == PieceKind::Pawn {
            self.pawn_hash.toggle_piece_on_square(sq, piece);
        }

        if is_minor_piece(piece) {
            self.minor_piece_hash.toggle_piece_on_square(sq, piece);
        }

        if is_major_piece(piece) {
            self.major_piece_hash.toggle_piece_on_square(sq, piece);
        }

        for player in Player::ALL {
            if piece.player == player && piece.kind != PieceKind::Pawn {
                self.non_pawn_hash[player].toggle_piece_on_square(sq, piece);
            }
        }
    }

    fn try_remove_castle_rights(&mut self, player: Player, castle_rights_side: CastleRightsSide) {
        let castle_rights = &mut self.castle_rights[player];

        // We don't want to modify anything if the castle rights on this side were already lost
        if !castle_rights.can_castle_to_side(castle_rights_side) {
            return;
        }

        castle_rights.remove_rights(castle_rights_side);

        self.hash.toggle_castle_rights(player, castle_rights_side);
    }

    // Convenience method to prevent tests from having to construct their own
    // movelist and allow them to iterate easily over the resulting list of moves
    pub fn moves(&self) -> MoveList {
        let mut movelist = MoveList::new();
        generate_legal_moves(self, |m| movelist.push(m));
        movelist
    }

    fn update_threats(&mut self) {
        let mut threats = Bitboard::EMPTY;

        let blockers = self.board.occupancy();
        let them = self.player.other();

        for pawn in self.board.pawns(them) {
            threats |= pawn_attacks(pawn, them);
        }

        for knight in self.board.knights(them) {
            threats |= knight_attacks(knight);
        }

        for diagonal_slider in self.board.diagonal_sliders(them) {
            threats |= bishop_attacks(diagonal_slider, blockers);
        }

        for orthogonal_slider in self.board.orthogonal_sliders(them) {
            threats |= rook_attacks(orthogonal_slider, blockers);
        }

        threats |= king_attacks(self.board.king_square(them));

        self.threats = threats;
    }

    fn update_checks_and_pins(&mut self) {
        self.checkers = Bitboard::EMPTY;
        self.orthogonal_pins = Bitboard::EMPTY;
        self.diagonal_pins = Bitboard::EMPTY;

        let our_king = self.board.king_square(self.player);
        let them = self.player.other();

        let our_pieces = self.board.occupancy_for(self.player);
        let their_pieces = self.board.occupancy_for(them);

        self.checkers |= pawn_attacks(our_king, self.player) & self.board.pawns(them);
        self.checkers |= knight_attacks(our_king) & self.board.knights(them);

        let their_orthogonal_sliders = self.board.orthogonal_sliders(them);
        let their_diagonal_sliders = self.board.diagonal_sliders(them);

        let potential_orthogonal_pinners =
            rook_attacks(our_king, their_pieces) & their_orthogonal_sliders;
        let potential_diagonal_pinners =
            bishop_attacks(our_king, their_pieces) & their_diagonal_sliders;

        for pinner in potential_orthogonal_pinners {
            let between_ray = tables::between(our_king, pinner);
            let blockers = between_ray & our_pieces;

            match blockers.count() {
                0 => self.checkers.set(pinner),
                1 => self.orthogonal_pins |= pinner.bb() | between_ray,
                _ => {}
            }
        }

        for pinner in potential_diagonal_pinners {
            let between_ray = tables::between(our_king, pinner);
            let blockers = between_ray & our_pieces;

            match blockers.count() {
                0 => self.checkers.set(pinner),
                1 => self.diagonal_pins |= pinner.bb() | between_ray,
                _ => {}
            }
        }
    }

    pub fn make_move(&mut self, mv: Move) {
        let from = mv.src();
        let to = mv.dst();
        let player = self.player;
        let other_player = player.other();

        let maybe_captured_piece = self.board.piece_at(to);

        // Capture the irreversible aspects of the position so that they can be restored
        // if we undo this move.
        let history = History {
            mv: Some(mv),
            captured: maybe_captured_piece,
            castle_rights: self.castle_rights,
            en_passant_target: self.en_passant_target,
            halfmove_clock: self.halfmove_clock,

            hash: self.hash,
            pawn_hash: self.pawn_hash,
            major_piece_hash: self.major_piece_hash,
            minor_piece_hash: self.minor_piece_hash,
            non_pawn_hash: self.non_pawn_hash,

            checkers: self.checkers,
            orthogonal_pins: self.orthogonal_pins,
            diagonal_pins: self.diagonal_pins,
            threats: self.threats,
        };

        self.history.push(history);

        let moved_piece = self.remove_at(from);

        if maybe_captured_piece.is_some() {
            self.remove_at(to);
        }

        if let Some(promoted_to) = mv.promotion() {
            let promoted_piece = Piece::new(player, promoted_to.piece());
            self.set_at(to, promoted_piece);
        } else {
            self.set_at(to, moved_piece);
        }

        // If we moved a pawn to the en passant target, this was an en passant capture, so we
        // remove the captured pawn from the board.
        if mv.is_en_passant() {
            // Remove the piece behind the square the pawn just moved to
            let capture_square = to.backward(player);
            self.remove_at(capture_square);
        }

        let new_en_passant_target = if mv.is_double_push() {
            let to_bb = to.bb();
            let en_passant_attacker_squares = to_bb.west() | to_bb.east();
            let enemy_pawns = self.board.pawns(other_player);
            let en_passant_can_happen = (en_passant_attacker_squares & enemy_pawns).any();

            if en_passant_can_happen {
                Some(from.forward(player))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(previous_en_passant_target) = self.en_passant_target {
            self.hash.toggle_en_passant(previous_en_passant_target);
        }

        if let Some(new_en_passant_target) = new_en_passant_target {
            self.hash.toggle_en_passant(new_en_passant_target);
        }

        self.en_passant_target = new_en_passant_target;

        if mv.is_castling()
            && let Some((rook_from, rook_to)) = squares::castle_squares(player, to)
        {
            let rook = self.remove_at(rook_from);
            self.set_at(rook_to, rook);
        }

        // Check if we lost castle rights.
        // If we moved the king, we lose all rights to castle.
        // If we moved one of our rooks, we lose rights to castle on that side.
        if moved_piece.kind == PieceKind::King && from == squares::king_start(player) {
            self.try_remove_castle_rights(player, CastleRightsSide::Kingside);
            self.try_remove_castle_rights(player, CastleRightsSide::Queenside);
        } else if moved_piece.kind == PieceKind::Rook {
            if from == squares::kingside_rook_start(player) {
                self.try_remove_castle_rights(player, CastleRightsSide::Kingside);
            } else if from == squares::queenside_rook_start(player) {
                self.try_remove_castle_rights(player, CastleRightsSide::Queenside);
            }
        }

        // Check if we removed our enemy's ability to castle, i.e. if we took one of their rooks
        if maybe_captured_piece.is_some() {
            if to == squares::kingside_rook_start(other_player) {
                self.try_remove_castle_rights(other_player, CastleRightsSide::Kingside);
            } else if to == squares::queenside_rook_start(other_player) {
                self.try_remove_castle_rights(other_player, CastleRightsSide::Queenside);
            }
        }

        let should_reset_halfmove_clock =
            maybe_captured_piece.is_some() || moved_piece.kind == PieceKind::Pawn;

        if should_reset_halfmove_clock {
            self.halfmove_clock = 0;
        } else {
            self.halfmove_clock += 1;
        }

        self.plies += 1;

        self.player = other_player;
        self.hash.toggle_side_to_play();

        self.update_checks_and_pins();
        self.update_threats();
    }

    pub fn make_null_move(&mut self) {
        // Capture the irreversible aspects of the position so that they can be restored
        // if we undo this move.
        let history = History {
            mv: None,
            captured: None,
            castle_rights: self.castle_rights,
            en_passant_target: self.en_passant_target,
            halfmove_clock: self.halfmove_clock,

            hash: self.hash,
            pawn_hash: self.pawn_hash,
            major_piece_hash: self.major_piece_hash,
            minor_piece_hash: self.minor_piece_hash,
            non_pawn_hash: self.non_pawn_hash,

            checkers: self.checkers,
            orthogonal_pins: self.orthogonal_pins,
            diagonal_pins: self.diagonal_pins,
            threats: self.threats,
        };

        self.history.push(history);

        if let Some(previous_en_passant_target) = self.en_passant_target {
            self.hash.toggle_en_passant(previous_en_passant_target);
        }

        self.en_passant_target = None;

        self.plies += 1;

        self.player = self.player.other();
        self.hash.toggle_side_to_play();

        self.update_checks_and_pins();
        self.update_threats();
    }

    pub fn undo_move(&mut self) {
        let history = self.history.pop().unwrap();
        let mv = history.mv.unwrap();
        let from = mv.src();
        let to = mv.dst();

        // The player that made this move is the one whose turn it was before
        // we start undoing the move.
        let player = self.player.other();
        let other_player = self.player;

        self.plies -= 1;
        self.player = player;
        self.hash = history.hash;
        self.pawn_hash = history.pawn_hash;
        self.major_piece_hash = history.major_piece_hash;
        self.minor_piece_hash = history.minor_piece_hash;
        self.non_pawn_hash = history.non_pawn_hash;
        self.halfmove_clock = history.halfmove_clock;
        self.castle_rights = history.castle_rights;
        self.en_passant_target = history.en_passant_target;
        self.checkers = history.checkers;
        self.orthogonal_pins = history.orthogonal_pins;
        self.diagonal_pins = history.diagonal_pins;
        self.threats = history.threats;

        // Undo castling, if we castled
        if mv.is_castling()
            && let Some((rook_from, rook_to)) = squares::castle_squares(player, to)
        {
            self.board.remove_at(rook_to);
            self.board
                .set_at(rook_from, Piece::new(player, PieceKind::Rook));
        }

        // Replace the pawn taken by en-passant capture
        if mv.is_en_passant() {
            let capture_square = to.backward(player);

            self.board
                .set_at(capture_square, Piece::new(other_player, PieceKind::Pawn));
        }

        let moved_piece = self.board.piece_guaranteed_at(to);
        self.board.remove_at(to);

        if let Some(captured_piece) = history.captured {
            self.board.set_at(to, captured_piece);
        }

        if mv.promotion().is_some() {
            self.board.set_at(from, Piece::new(player, PieceKind::Pawn));
        } else {
            self.board.set_at(from, moved_piece);
        }
    }

    pub fn undo_null_move(&mut self) {
        let history = self.history.pop().unwrap();
        assert!(history.mv.is_none());

        self.plies -= 1;
        self.player = self.player.other();
        self.hash = history.hash;
        self.pawn_hash = history.pawn_hash;
        self.major_piece_hash = history.major_piece_hash;
        self.minor_piece_hash = history.minor_piece_hash;
        self.non_pawn_hash = history.non_pawn_hash;
        self.en_passant_target = history.en_passant_target;
        self.halfmove_clock = history.halfmove_clock;
        self.checkers = history.checkers;
        self.orthogonal_pins = history.orthogonal_pins;
        self.diagonal_pins = history.diagonal_pins;
        self.threats = history.threats;
    }

    pub fn approx_zobrist_after(&self, mv: Move) -> ZobristHash {
        let mut key = self.hash;

        let moved_piece = self.board.piece_guaranteed_at(mv.src());
        let captured_piece = self.board.piece_at(mv.dst());

        key.toggle_piece_on_square(mv.src(), moved_piece);

        if let Some(promotion) = mv.promotion() {
            key.toggle_piece_on_square(mv.dst(), Piece::new(moved_piece.player, promotion.piece()));
        } else {
            key.toggle_piece_on_square(mv.dst(), moved_piece);
        }

        if let Some(captured_piece) = captured_piece {
            key.toggle_piece_on_square(mv.dst(), captured_piece);
        }

        if let Some(ep_target) = self.en_passant_target {
            key.toggle_en_passant(ep_target);
        }

        if mv.is_double_push() {
            let ep_square = mv.dst().backward(self.player);
            key.toggle_en_passant(ep_square);
        }

        key.toggle_side_to_play();
        key
    }

    pub fn approx_zobrist_after_null_move(&self) -> ZobristHash {
        let mut key = self.hash;
        key.toggle_side_to_play();
        key
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::bitboard::bitboards::*;

    #[test]
    fn test_draw_by_insufficient_material() {
        crate::init();

        // Knight vs Bishop mate
        assert!(
            !Game::from_fen("5b1K/5k1N/8/8/8/8/8/8 b - - 1 1")
                .unwrap()
                .is_stalemate_by_insufficient_material()
        );

        // Bishop vs Knight - draw
        assert!(
            Game::from_fen("8/8/3k4/4n3/8/2KB4/8/8 w - - 0 1")
                .unwrap()
                .is_stalemate_by_insufficient_material()
        );

        // Rook vs Knight mate
        assert!(
            !Game::from_fen("8/8/4k3/4n3/8/2KR4/8/8 w - - 0 1")
                .unwrap()
                .is_stalemate_by_insufficient_material()
        );
    }

    #[test]
    fn test_pin_in_gist_8_depth_3() {
        crate::init();

        let game =
            Game::from_fen("rnbq1k1r/pp1P1ppp/2p5/8/1bB5/8/PPPNNnPP/R1BQK2R w KQ - 3 9").unwrap();

        assert_eq!(game.orthogonal_pins, Bitboard::EMPTY);
        assert_eq!(game.diagonal_pins, B4_BB | C3_BB | D2_BB);
    }
}
