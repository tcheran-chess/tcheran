use crate::chess::{
    Bitboard, Board, Move, Piece, PieceKind, Player, PromotionPieceKind, Square, bitboards,
    moves::{
        MoveList, bishop_attacks, generate_legal_moves, king_attacks, knight_attacks, pawn_attacks,
        rook_attacks,
    },
    notations,
    ranks::{back_rank, pawn_back_rank, promotion_rank},
    rays::{ray_between, ray_intersecting},
    squares,
    zobrist::{self, ZobristHash},
};

pub trait MoveObserver {
    fn init(&mut self, moved_piece: Piece, mv: Move);
    fn set(&mut self, sq: Square, piece: Piece);
    fn remove(&mut self, sq: Square, piece: Piece);
}

struct NullObserver;

impl MoveObserver for NullObserver {
    fn init(&mut self, _moved_piece: Piece, _move: Move) {}
    fn set(&mut self, _sq: Square, _piece: Piece) {}
    fn remove(&mut self, _sq: Square, _piece: Piece) {}
}

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
    pub king_side: Option<Square>,
    pub queen_side: Option<Square>,
}

impl CastleRights {
    pub const fn none() -> Self {
        Self {
            king_side: None,
            queen_side: None,
        }
    }

    pub fn can_castle_to_side(self, side: CastleRightsSide) -> bool {
        match side {
            CastleRightsSide::Kingside => self.king_side.is_some(),
            CastleRightsSide::Queenside => self.queen_side.is_some(),
        }
    }

    pub fn castle_dst_squares(self, player: Player, to: Square) -> Option<(Square, Square)> {
        if let Some(king_side) = self.king_side
            && to == king_side
        {
            return Some((
                squares::kingside_king_castle_end(player),
                squares::kingside_rook_castle_end(player),
            ));
        }

        if let Some(queen_side) = self.queen_side
            && to == queen_side
        {
            return Some((
                squares::queenside_king_castle_end(player),
                squares::queenside_rook_castle_end(player),
            ));
        }

        None
    }

    pub fn remove_rights(&mut self, side: CastleRightsSide) {
        match side {
            CastleRightsSide::Kingside => self.king_side = None,
            CastleRightsSide::Queenside => self.queen_side = None,
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
    pub moved: Option<Piece>,
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
    pub check_zones: [Bitboard; 4],
    pub pinned: [Bitboard; Player::N],
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
    pub check_zones: [Bitboard; 4],
    pub pinned: [Bitboard; Player::N],
    pub threats: Bitboard,

    pub is_frc: bool,
}

impl Game {
    pub fn new() -> Self {
        Self::from_fen(notations::fen::START_POS).unwrap()
    }

    pub fn new_dfrc(white_idx: usize, black_idx: usize) -> Self {
        let (board, castle_rights) = notations::scharnagl::from_idxs(white_idx, black_idx);
        Self::from_state(board, Player::White, castle_rights, None, 0, 0, true)
    }

    pub fn from_state(
        board: Board,
        player: Player,
        castle_rights: [CastleRights; Player::N],
        en_passant_target: Option<Square>,
        halfmove_clock: u32,
        plies: u32,
        is_frc: bool,
    ) -> Self {
        let mut game = Self {
            board,
            player,
            castle_rights,
            en_passant_target,
            halfmove_clock,
            plies,

            checkers: Bitboard::EMPTY,
            check_zones: [Bitboard::EMPTY; 4],
            pinned: [Bitboard::EMPTY; Player::N],
            threats: Bitboard::EMPTY,

            hash: ZobristHash::uninit(),
            pawn_hash: ZobristHash::uninit(),
            major_piece_hash: ZobristHash::uninit(),
            minor_piece_hash: ZobristHash::uninit(),
            non_pawn_hash: [ZobristHash::uninit(); Player::N],

            history: Vec::new(),

            is_frc,
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
        game.check_en_passant_square_is_valid();

        game
    }

    pub fn from_fen(fen: &str) -> Result<Self, notations::fen::ParseError> {
        notations::fen::parse(fen, false)
    }

    pub fn from_frc_fen(fen: &str) -> Result<Self, notations::fen::ParseError> {
        notations::fen::parse(fen, true)
    }

    pub fn to_fen(&self) -> String {
        notations::fen::write(self)
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
    pub fn in_check(&self) -> bool {
        self.checkers.any()
    }

    #[inline(always)]
    pub fn is_direct_check(&self, mv: Move) -> bool {
        let moved_piece_kind = mv.promotion().map_or_else(
            || self.board.piece_guaranteed_at(mv.from()).kind,
            PromotionPieceKind::piece,
        );

        let zones = match moved_piece_kind {
            PieceKind::Pawn => self.check_zones[0],
            PieceKind::Knight => self.check_zones[1],
            PieceKind::Bishop => self.check_zones[2],
            PieceKind::Rook => self.check_zones[3],
            PieceKind::Queen => self.check_zones[2] | self.check_zones[3],
            PieceKind::King => return false,
        };

        zones.contains(mv.to())
    }

    fn set_at(&mut self, sq: Square, piece: Piece, observer: &mut impl MoveObserver) {
        self.board.set_at(sq, piece);
        self.toggle_piece_in_hashes(sq, piece);
        observer.set(sq, piece);
    }

    fn remove_at(&mut self, sq: Square, observer: &mut impl MoveObserver) -> Piece {
        let removed_piece = self.board.piece_guaranteed_at(sq);
        self.board.remove_at(sq);
        self.toggle_piece_in_hashes(sq, removed_piece);
        observer.remove(sq, removed_piece);

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

        let us = self.player;
        let them = !self.player;

        let our_king = self.board.king(us);
        let blockers = self.board.occupancy() & !our_king;

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
        self.pinned = [Bitboard::EMPTY; Player::N];

        let them = self.player.other();

        let our_king = self.board.king_square(self.player);
        let their_king = self.board.king_square(them);

        self.checkers |= pawn_attacks(our_king, self.player) & self.board.pawns(them);
        self.checkers |= knight_attacks(our_king) & self.board.knights(them);

        let all_diagonal_sliders = self.board.all_diagonal_sliders();
        let all_orthogonal_sliders = self.board.all_orthogonal_sliders();

        for player in Player::ALL {
            let our_king = self.board.king_square(player);
            let our_pieces = self.board.occupancy_for(player);
            let their_pieces = self.board.occupancy_for(!player);

            let potential_diagonal_pinners = all_diagonal_sliders
                & bishop_attacks(our_king, their_pieces)
                & self.board.diagonal_sliders(!player);

            let potential_orthogonal_pinners = all_orthogonal_sliders
                & rook_attacks(our_king, their_pieces)
                & self.board.orthogonal_sliders(!player);

            for pinner in potential_orthogonal_pinners | potential_diagonal_pinners {
                let between_ray = ray_between(our_king, pinner);
                let blockers = between_ray & our_pieces;

                match blockers.count() {
                    0 if player == self.player => self.checkers.set(pinner),
                    1 => self.pinned[player].set(blockers.single()),
                    _ => {}
                }
            }
        }

        let blockers = self.board.occupancy();

        self.check_zones = [
            pawn_attacks(their_king, them),
            knight_attacks(their_king),
            bishop_attacks(their_king, blockers),
            rook_attacks(their_king, blockers),
        ];
    }

    // Logic ported from Stormphrax
    pub fn is_legal(&self, mv: Move) -> bool {
        let us = self.player;
        let them = !self.player;

        let from = mv.from();
        let to = mv.to();
        let from_piece = self.board.piece_at(from);
        let to_piece = self.board.piece_at(to);
        let our_king = self.board.king_square(us);
        let occupancy = self.board.occupancy();

        // There has to have been a piece that we moved
        let Some(moved_piece) = from_piece else {
            return false;
        };

        // That piece has to have been ours
        if moved_piece.player != us {
            return false;
        }

        // Check some preconditions for special move types
        if mv.is_capture() && !mv.is_en_passant() && to_piece.is_none() {
            return false;
        }

        // If we're in check and moving anything except the king:
        if self.in_check() && moved_piece.kind != PieceKind::King {
            // If multiple pieces are checking, we had to evade with the king
            if self.checkers.count() > 1 {
                return false;
            }

            let checker = self.checkers.single();
            // en passant requires special handling
            if !mv.is_en_passant() {
                let check_ray = ray_between(our_king, checker);

                // We either block the check or capture the checker
                let valid_destinations = check_ray | checker.bb();

                if !valid_destinations.contains(to) {
                    return false;
                }
            }
        }

        // If the piece we're moving is pinned, it can only move along the pin ray
        if self.pinned[us].contains(from) {
            let move_ray = ray_intersecting(from, to);
            if !move_ray.contains(our_king) {
                return false;
            }
        }

        // If we're capturing:
        if let Some(captured_piece) = to_piece {
            if captured_piece.player == us {
                // We can only 'capture' our own piece if castling
                if !mv.is_castling() {
                    return false;
                }

                // When castling, we can only capture rooks
                if captured_piece.kind != PieceKind::Rook {
                    return false;
                }
            } else if !mv.is_capture() {
                return false;
            }

            // We can't capture kings
            if captured_piece.kind == PieceKind::King {
                return false;
            }
        }

        // If we're castling:
        if mv.is_castling() {
            // We can only castle a king
            if moved_piece.kind != PieceKind::King {
                return false;
            }

            // Can't castle in check!
            if self.in_check() {
                return false;
            }

            let our_back_rank = back_rank(us);

            // We have to stay on the back rank
            if from.rank() != our_back_rank || to.rank() != our_back_rank {
                return false;
            }

            // We have to have rights to castle
            let (king_dst, rook_dst) = if Some(to) == self.castle_rights[us].king_side {
                (squares::kingside_king_castle_end(us), squares::kingside_rook_castle_end(us))
            } else if Some(to) == self.castle_rights[us].queen_side {
                (squares::queenside_king_castle_end(us), squares::queenside_rook_castle_end(us))
            } else {
                return false;
            };

            return if self.is_frc {
                if self.pinned[us].contains(to) {
                    return false;
                }

                let required_safe_squares = ray_between(from, king_dst) | from.bb() | king_dst.bb();
                let required_empty_squares =
                    required_safe_squares | ray_between(from, to) | rook_dst.bb();

                let blockers = occupancy.without(from).without(to);

                (required_empty_squares & blockers).is_empty()
                    && (required_safe_squares & self.threats).is_empty()
            } else {
                let required_empty_squares = ray_between(from, to);
                let required_safe_squares = ray_between(from, king_dst) | king_dst.bb();

                (required_empty_squares & occupancy).is_empty()
                    && (required_safe_squares & self.threats).is_empty()
            };
        }

        // Lots of special handling for pawn moves
        if moved_piece.kind == PieceKind::Pawn {
            if mv.is_en_passant() {
                // Can't do en-passant without an en-passant target
                let Some(en_passant_target) = self.en_passant_target else {
                    return false;
                };

                // Must have moved to the en-passant target square
                if to != en_passant_target {
                    return false;
                }

                // Must have moved there from a valid square
                let valid_sources = pawn_attacks(en_passant_target, them);
                if !valid_sources.contains(from) {
                    return false;
                }

                let capture_square = en_passant_target.forward(them);
                let occupancy_after_ep = occupancy.without(from).with(to).without(capture_square);

                let orthogonal_checks = rook_attacks(our_king, occupancy_after_ep)
                    & self.board.orthogonal_sliders(them);

                let diagonal_checks = bishop_attacks(our_king, occupancy_after_ep)
                    & self.board.diagonal_sliders(them);

                return orthogonal_checks.is_empty() && diagonal_checks.is_empty();
            }

            let promotion_rank = promotion_rank(us);
            if from.rank() == promotion_rank && !mv.is_promotion()
                || mv.is_promotion() && from.rank() != promotion_rank
            {
                return false;
            }

            let their_pieces = self.board.occupancy_for(them);

            let valid_destinations = if mv.is_capture() {
                pawn_attacks(from, us) & their_pieces
            } else {
                let single_push = from.forward(us).bb();

                let pawn_back_rank = pawn_back_rank(us);

                if mv.is_double_push() && from.rank() != pawn_back_rank {
                    return false;
                }

                // Can't double push over one of their pieces - single pushing is handled above
                if from.rank() == pawn_back_rank && (single_push & their_pieces).is_empty() {
                    if mv.is_double_push() && (single_push & occupancy).is_empty() {
                        single_push.forward(us)
                    } else {
                        single_push
                    }
                } else {
                    single_push
                }
            };

            if !valid_destinations.contains(to) {
                return false;
            }
        } else {
            // Not valid for non-pawns
            if mv.is_promotion() || mv.is_en_passant() || mv.is_double_push() {
                return false;
            }

            let valid_destinations = match moved_piece.kind {
                PieceKind::Knight => knight_attacks(from),
                PieceKind::Bishop => bishop_attacks(from, occupancy),
                PieceKind::Rook => rook_attacks(from, occupancy),
                PieceKind::Queen => bishop_attacks(from, occupancy) | rook_attacks(from, occupancy),
                PieceKind::King => king_attacks(from) & !self.threats,

                // Handled above
                PieceKind::Pawn => unreachable!(),
            };

            if !valid_destinations.contains(to) {
                return false;
            }
        }

        true
    }

    pub fn make_move(&mut self, mv: Move) {
        self.make_move_observed(mv, &mut NullObserver);
    }

    pub fn make_move_observed(&mut self, mv: Move, observer: &mut impl MoveObserver) {
        let from = mv.from();
        let to = mv.to();
        let player = self.player;
        let other_player = player.other();

        let moved_piece = self.board.piece_guaranteed_at(from);
        let maybe_captured_piece = self.board.piece_at(to);

        observer.init(moved_piece, mv);

        // Capture the irreversible aspects of the position so that they can be restored
        // if we undo this move.
        let history = History {
            mv: Some(mv),
            moved: Some(moved_piece),
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
            check_zones: self.check_zones,
            pinned: self.pinned,
            threats: self.threats,
        };

        self.history.push(history);

        self.remove_at(from, observer);

        if maybe_captured_piece.is_some() {
            self.remove_at(to, observer);
        }

        // Move the piece to the destination, unless we're castling.
        // If we're castling, the destination square is the rook's square, which is not where the king
        // ends up. We'll move it to the right square later on.
        if let Some(promoted_to) = mv.promotion() {
            let promoted_piece = Piece::new(player, promoted_to.piece());
            self.set_at(to, promoted_piece, observer);
        } else if !mv.is_castling() {
            self.set_at(to, moved_piece, observer);
        }

        // If we moved a pawn to the en passant target, this was an en passant capture, so we
        // remove the captured pawn from the board.
        if mv.is_en_passant() {
            // Remove the piece behind the square the pawn just moved to
            let capture_square = to.backward(player);
            self.remove_at(capture_square, observer);
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
            && let Some((king_to, rook_to)) =
                self.castle_rights[player].castle_dst_squares(player, to)
        {
            self.set_at(king_to, moved_piece, observer);

            // The rook was 'captured'
            self.set_at(rook_to, maybe_captured_piece.unwrap(), observer);
        }

        // If we moved the king, we lose castle rights
        if moved_piece.kind == PieceKind::King {
            self.try_remove_castle_rights(player, CastleRightsSide::Kingside);
            self.try_remove_castle_rights(player, CastleRightsSide::Queenside);
        } else if moved_piece.kind == PieceKind::Rook {
            // If we moved one of our rooks, we lose rights to castle on that side.
            if Some(from) == self.castle_rights[player].king_side {
                self.try_remove_castle_rights(player, CastleRightsSide::Kingside);
            } else if Some(from) == self.castle_rights[player].queen_side {
                self.try_remove_castle_rights(player, CastleRightsSide::Queenside);
            }
        }

        // Check if we removed our enemy's ability to castle, i.e. if we took one of their rooks
        if let Some(captured_piece) = maybe_captured_piece
            && captured_piece.kind == PieceKind::Rook
        {
            if Some(to) == self.castle_rights[other_player].king_side {
                self.try_remove_castle_rights(other_player, CastleRightsSide::Kingside);
            } else if Some(to) == self.castle_rights[other_player].queen_side {
                self.try_remove_castle_rights(other_player, CastleRightsSide::Queenside);
            }
        }

        let should_reset_halfmove_clock = mv.is_capture() || moved_piece.kind == PieceKind::Pawn;

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
        self.check_en_passant_square_is_valid();
    }

    pub fn make_null_move(&mut self) {
        // Capture the irreversible aspects of the position so that they can be restored
        // if we undo this move.
        let history = History {
            mv: None,
            moved: None,
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
            check_zones: self.check_zones,
            pinned: self.pinned,
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
        let from = mv.from();
        let to = mv.to();

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
        self.check_zones = history.check_zones;
        self.pinned = history.pinned;
        self.threats = history.threats;

        // Undo castling, if we castled
        if mv.is_castling()
            && let Some((king_to, rook_to)) =
                self.castle_rights[player].castle_dst_squares(player, to)
        {
            self.board.remove_at(king_to);
            self.board.remove_at(rook_to);
        }

        // Replace the pawn taken by en-passant capture
        if mv.is_en_passant() {
            let capture_square = to.backward(player);

            self.board
                .set_at(capture_square, Piece::new(other_player, PieceKind::Pawn));
        }

        // If we castled, the piece we moved doesn't end up on the 'to' square.
        if !mv.is_castling() {
            self.board.remove_at(to);
        }

        if let Some(captured_piece) = history.captured {
            self.board.set_at(to, captured_piece);
        }

        self.board.set_at(from, history.moved.unwrap());
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
        self.check_zones = history.check_zones;
        self.pinned = history.pinned;
        self.threats = history.threats;
    }

    pub fn check_en_passant_square_is_valid(&mut self) {
        let Some(en_passant_target) = self.en_passant_target else {
            return;
        };

        let mut can_capture = false;

        let moved_pawn = en_passant_target.backward(self.player);
        let king = self.board.king_square(self.player);
        let them = !self.player;

        let potential_capturers =
            self.board.pawns(self.player) & pawn_attacks(en_passant_target, them);

        for pawn in potential_capturers {
            let blockers = self
                .board
                .occupancy()
                .without(moved_pawn)
                .with(en_passant_target)
                .without(pawn);

            let checkers = rook_attacks(king, blockers) & self.board.orthogonal_sliders(them)
                | bishop_attacks(king, blockers) & self.board.diagonal_sliders(them);

            if checkers.is_empty() {
                can_capture = true;
            }
        }

        // If all of our pawns were ruled out due to pins, there's no valid target
        if !can_capture {
            self.en_passant_target = None;
            self.hash.toggle_en_passant(en_passant_target);
        }
    }

    pub fn approx_zobrist_after(&self, mv: Move) -> ZobristHash {
        let mut key = self.hash;

        let moved_piece = self.board.piece_guaranteed_at(mv.from());
        let captured_piece = self.board.piece_at(mv.to());

        key.toggle_piece_on_square(mv.from(), moved_piece);

        if let Some(promotion) = mv.promotion() {
            key.toggle_piece_on_square(mv.to(), Piece::new(moved_piece.player, promotion.piece()));
        } else if !mv.is_castling() {
            key.toggle_piece_on_square(mv.to(), moved_piece);
        }

        if let Some(captured_piece) = captured_piece {
            key.toggle_piece_on_square(mv.to(), captured_piece);
        }

        if let Some(ep_target) = self.en_passant_target {
            key.toggle_en_passant(ep_target);
        }

        if mv.is_double_push() {
            let ep_square = mv.to().backward(self.player);
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
    use crate::chess::{moves::MoveListExt, square::squares::all::*};

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
    fn test_en_passant_target_not_set_if_not_legal() {
        crate::init();

        let mut game = Game::from_fen("k7/3p4/8/K3P2r/8/8/8/8 b - - 0 1").unwrap();
        game.make_move(game.moves().expect_matching(D7, D5, None));

        assert_eq!(game.en_passant_target, None);

        // Bishop pinning the capturing pawn diagonally
        let mut game =
            Game::from_fen("r3k2r/Pppp1ppp/1b3nbN/nPP5/BB2P3/q4N2/Pp1P2PP/R2Q1RK1 b kq - 0 1")
                .unwrap();
        game.make_move(game.moves().expect_matching(D7, D5, None));

        assert_eq!(game.en_passant_target, None);

        let mut game =
            Game::from_fen("r7/r7/bp1k3p/2p1p1pP/Pp1pP1P1/1P3P2/2P3K1/1R1RN3 w - - 9 40").unwrap();
        game.make_move(game.moves().expect_matching(C2, C4, None));

        assert_eq!(game.en_passant_target, Some(C3));

        let mut game =
            Game::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        game.make_move(game.moves().expect_matching(A2, A4, None));

        assert_eq!(game.en_passant_target, Some(A3));
    }

    #[test]
    fn test_islegal_castling_in_kiwipete() {
        crate::init();

        let game =
            Game::from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1")
                .unwrap();
        assert!(game.is_legal(Move::castles(E1, H1)));
    }

    #[test]
    fn test_islegal_en_passant_in_bench_position() {
        crate::init();

        let game =
            Game::from_fen("5r2/1p3k2/pBp1p1b1/3rq1b1/PPR1pPpp/4Q1P1/4P1BP/5RK1 b - f3 0 28")
                .unwrap();
        assert!(game.is_legal(Move::en_passant(G4, F3)));
    }
}
