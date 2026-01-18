pub mod lmr_table;

pub fn init() {
    lmr_table::init();
}

use std::cmp::min;

use crate::{
    chess::{
        game::Game,
        moves::{Move, MoveList},
        piece::{Piece, PieceKind},
        player::Player,
        square::Square,
        zobrist::ZobristHash,
    },
    engine::{eval::Eval, params::*, search::MAX_SEARCH_DEPTH_SIZE, util::mem::alloc_boxed},
};

pub struct HistoryEntry<const MAX: i16>(i16);

impl<const MAX: i16> HistoryEntry<MAX> {
    #[expect(clippy::cast_possible_truncation, reason = "Dipped into i32 to avoid overflows")]
    pub fn update(&mut self, bonus: i32) {
        let old = i32::from(self.0);
        let max = i32::from(MAX);

        self.0 = (old + bonus - (old * bonus.abs()) / max) as i16;
    }

    pub fn get(&self) -> i32 {
        i32::from(self.0)
    }
}

pub struct Tables {
    pub quiet_history: Box<QuietHistoryTable>,
    pub capture_history: Box<CaptureHistoryTable>,
    pub killer_moves: KillersTable,
    pub conthist: Box<ContHistTable>,
    pub corrhist: CorrectionHistories,
}

impl Tables {
    pub fn new() -> Self {
        Self {
            quiet_history: QuietHistoryTable::new(),
            capture_history: CaptureHistoryTable::new(),
            killer_moves: KillersTable::new(),
            conthist: ContHistTable::new(),
            corrhist: CorrectionHistories::new(),
        }
    }

    pub fn new_search(&mut self) {
        self.killer_moves = KillersTable::new();
    }
}

pub struct KillersTable([Option<Move>; MAX_SEARCH_DEPTH_SIZE]);

impl KillersTable {
    pub const fn new() -> Self {
        Self([None; MAX_SEARCH_DEPTH_SIZE])
    }

    pub fn get(&self, plies: u8) -> Option<Move> {
        self.0[plies as usize]
    }

    pub fn set(&mut self, plies: u8, mv: Move) {
        self.0[plies as usize] = Some(mv);
    }

    pub fn clear(&mut self, plies: u8) {
        self.0[plies as usize] = None;
    }
}

pub fn history_bonus(depth: u8) -> i32 {
    min(history_factor() * i32::from(depth) - history_offset(), history_max_bonus())
}

pub struct QuietHistoryTable(
    [[[[[HistoryEntry<{ Self::MAX }>; 2]; 2]; Square::N]; Square::N]; Player::N],
);

impl QuietHistoryTable {
    const MAX: i16 = 8192;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    pub fn get(&self, game: &Game, mv: Move) -> i32 {
        let from_threatened = usize::from(game.threats.contains(mv.src()));
        let to_threatened = usize::from(game.threats.contains(mv.dst()));

        self.0[game.player][mv.src()][mv.dst()][from_threatened][to_threatened].get()
    }

    fn update_for_move(&mut self, game: &Game, mv: Move, bonus: i32) {
        let from_threatened = usize::from(game.threats.contains(mv.src()));
        let to_threatened = usize::from(game.threats.contains(mv.dst()));

        self.0[game.player][mv.src()][mv.dst()][from_threatened][to_threatened].update(bonus);
    }

    pub fn update(&mut self, game: &Game, mv: Move, depth: u8, other_quiets_tried: &MoveList) {
        let bonus = history_bonus(depth);

        self.update_for_move(game, mv, bonus);

        for other_quiet in other_quiets_tried {
            self.update_for_move(game, *other_quiet, -bonus);
        }
    }
}

pub struct CaptureHistoryTable(
    [[[[[[HistoryEntry<{ Self::MAX }>; 2]; 2]; PieceKind::N]; Square::N]; PieceKind::N]; Player::N],
);

impl CaptureHistoryTable {
    pub const MAX: i16 = 8192;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    pub fn get(&self, game: &Game, mv: Move) -> i32 {
        let capturing_piece = game.board.piece_guaranteed_at(mv.src()).kind;
        let capture_square = mv.dst();
        let captured_piece = game.board.captured_piece(mv).expect("Move was a capture");
        let from_threatened = usize::from(game.threats.contains(mv.src()));
        let to_threatened = usize::from(game.threats.contains(mv.dst()));

        self.0[game.player][capturing_piece][capture_square][captured_piece][from_threatened]
            [to_threatened]
            .get()
    }

    fn update_for_move(&mut self, mv: Move, game: &Game, bonus: i32) {
        let capturing_piece = game.board.piece_guaranteed_at(mv.src()).kind;
        let capture_square = mv.dst();
        let captured_piece = game.board.captured_piece(mv).expect("Move was a capture");
        let from_threatened = usize::from(game.threats.contains(mv.src()));
        let to_threatened = usize::from(game.threats.contains(mv.dst()));

        self.0[game.player][capturing_piece][capture_square][captured_piece][from_threatened]
            [to_threatened]
            .update(bonus);
    }

    pub fn update(&mut self, mv: Move, game: &Game, depth: u8, other_captures_tried: &MoveList) {
        let bonus = history_bonus(depth);

        if mv.is_capture() {
            self.update_for_move(mv, game, bonus);
        }

        for other_capture in other_captures_tried {
            self.update_for_move(*other_capture, game, -bonus);
        }
    }
}

pub struct ContHistTable(
    [[[[HistoryEntry<{ Self::MAX }>; Square::N]; Piece::N]; Square::N]; Piece::N],
);

impl ContHistTable {
    const MAX: i16 = 16384;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    pub fn get(
        &self,
        game: &Game,
        previous_piece_moved: Piece,
        previous_moved_to: Square,
        mv: Move,
    ) -> i32 {
        let moved = game.board.piece_guaranteed_at(mv.src());
        self.0[previous_piece_moved][previous_moved_to][moved][mv.dst()].get()
    }

    fn update_for_move(
        &mut self,
        previous_piece_moved: Piece,
        previous_moved_to: Square,
        moved: Piece,
        moved_to: Square,
        bonus: i32,
    ) {
        self.0[previous_piece_moved][previous_moved_to][moved][moved_to].update(bonus);
    }

    pub fn update(
        &mut self,
        game: &Game,
        previous_piece_moved: Piece,
        previous_move: Move,
        mv: Move,
        depth: u8,
        quiets_tried: &MoveList,
    ) {
        let bonus = history_bonus(depth);

        let moved = game.board.piece_guaranteed_at(mv.src());
        self.update_for_move(previous_piece_moved, previous_move.dst(), moved, mv.dst(), bonus);

        for quiet_tried in quiets_tried {
            let try_moved = game.board.piece_guaranteed_at(quiet_tried.src());
            self.update_for_move(
                previous_piece_moved,
                previous_move.dst(),
                try_moved,
                quiet_tried.dst(),
                -bonus,
            );
        }
    }
}

pub struct CorrectionHistories {
    pawn: Box<CorrectionHistoryTable>,
    major: Box<CorrectionHistoryTable>,
    minor: Box<CorrectionHistoryTable>,
    non_pawn: [Box<CorrectionHistoryTable>; Player::N],
    threat: Box<CorrectionHistoryTable>,
}

impl CorrectionHistories {
    pub fn new() -> Self {
        Self {
            pawn: CorrectionHistoryTable::new(),
            major: CorrectionHistoryTable::new(),
            minor: CorrectionHistoryTable::new(),
            non_pawn: [CorrectionHistoryTable::new(), CorrectionHistoryTable::new()],
            threat: CorrectionHistoryTable::new(),
        }
    }

    pub fn get(&self, game: &mut Game) -> Eval {
        let corr = self.pawn.get(game.player, game.pawn_hash) * pawn_correction_history_weight()
            + self.major.get(game.player, game.major_piece_hash)
                * major_correction_history_weight()
            + self.minor.get(game.player, game.minor_piece_hash)
                * minor_correction_history_weight()
            + (self.non_pawn[Player::White].get(game.player, game.non_pawn_hash[Player::White])
                + self.non_pawn[Player::Black].get(game.player, game.non_pawn_hash[Player::Black]))
                * non_pawn_correction_history_weight()
            + self
                .threat
                .get(game.player, ZobristHash(game.threats.as_u64()))
                * threat_correction_history_weight();

        corr / 2048
    }

    pub fn update(&mut self, game: &mut Game, depth: u8, eval_diff: Eval) {
        self.pawn
            .update(game.player, game.pawn_hash, depth, eval_diff);

        self.major
            .update(game.player, game.major_piece_hash, depth, eval_diff);

        self.minor
            .update(game.player, game.minor_piece_hash, depth, eval_diff);

        self.non_pawn[Player::White].update(
            game.player,
            game.non_pawn_hash[Player::White],
            depth,
            eval_diff,
        );

        self.non_pawn[Player::Black].update(
            game.player,
            game.non_pawn_hash[Player::Black],
            depth,
            eval_diff,
        );

        self.threat
            .update(game.player, ZobristHash(game.threats.as_u64()), depth, eval_diff);
    }
}

const CORRECTION_HISTORY_SIZE: usize = 16384;
pub struct CorrectionHistoryTable(
    [[HistoryEntry<{ Self::MAX }>; CORRECTION_HISTORY_SIZE]; Player::N],
);

impl CorrectionHistoryTable {
    pub const MAX: i16 = 1024;
    pub const MAX_UPDATE: i32 = Self::MAX as i32 / 4;

    pub fn new() -> Box<Self> {
        alloc_boxed()
    }

    #[expect(clippy::cast_possible_truncation, reason = "u64 to usize")]
    pub fn get(&self, player: Player, key: ZobristHash) -> Eval {
        Eval(self.0[player][key.0 as usize % CORRECTION_HISTORY_SIZE].get())
    }

    #[expect(clippy::cast_possible_truncation, reason = "u64 to usize")]
    pub fn update(&mut self, player: Player, key: ZobristHash, depth: u8, eval_diff: Eval) {
        let raw_bonus = eval_diff.0 * i32::from(depth) / 8;
        let bonus = i32::clamp(raw_bonus, -Self::MAX_UPDATE, Self::MAX_UPDATE);

        self.0[player][key.0 as usize % CORRECTION_HISTORY_SIZE].update(bonus);
    }
}
