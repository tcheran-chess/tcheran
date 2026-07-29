use std::cmp::min;

use crate::{
    chess::{Game, Move, Piece, PieceKind, Player, Square, moves::MoveList, zobrist::ZobristHash},
    engine::{
        eval::Eval,
        params::*,
        search::{MAX_PLIES_ARRAY_SIZE, SearchContext, SearchStack, types::Depth},
    },
};

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "Calculation is intentionally approximate"
)]
pub fn init() {
    let tactial_base = tactical_lmr_base() as f32 / 100.0;
    let tactial_factor = tactical_lmr_factor() as f32 / 100.0;
    let quiet_base = quiet_lmr_base() as f32 / 100.0;
    let quiet_factor = quiet_lmr_factor() as f32 / 100.0;

    unsafe {
        for depth in 1..MAX_PLIES_ARRAY_SIZE {
            for move_count in 1..64 {
                for is_quiet in [false, true] {
                    let base = if is_quiet { quiet_base } else { tactial_base };
                    let factor = if is_quiet {
                        quiet_factor
                    } else {
                        tactial_factor
                    };

                    let reduction = (base
                        + (f32::ln(depth as f32) * f32::ln(move_count as f32) / factor))
                        as u8;

                    LMR_TABLE[depth][move_count][usize::from(is_quiet)] = reduction;
                }
            }
        }
    }
}

static mut LMR_TABLE: [[[u8; 2]; 64]; MAX_PLIES_ARRAY_SIZE] = [[[0; 2]; 64]; MAX_PLIES_ARRAY_SIZE];

pub fn lmr_reduction(depth: Depth, move_count: usize, is_quiet: bool) -> u8 {
    unsafe {
        LMR_TABLE[depth.idx().min(MAX_PLIES_ARRAY_SIZE - 1)][move_count.min(63)]
            [usize::from(is_quiet)]
    }
}

pub struct HistoryEntry<const MAX: i16>(i16);

impl<const MAX: i16> HistoryEntry<MAX> {
    #[expect(clippy::cast_possible_truncation, reason = "Dipped into i32 to avoid overflows")]
    pub fn update(&mut self, bonus: i32) {
        let old = i32::from(self.0);
        let max = i32::from(MAX);

        self.0 = (old + bonus - old * bonus.abs() / max) as i16;
    }

    #[expect(clippy::cast_possible_truncation, reason = "Dipped into i32 to avoid overflows")]
    pub fn update_with_base(&mut self, base: i32, bonus: i32) {
        let old = i32::from(self.0);
        let max = i32::from(MAX);

        self.0 = (old + bonus - (base * bonus.abs()) / max) as i16;
    }

    pub fn get(&self) -> i32 {
        i32::from(self.0)
    }
}

pub struct Tables {
    pub quiet_history: Box<QuietHistoryTable>,
    pub tactical_history: Box<TacticalHistoryTable>,
    pub conthist: Box<ContHistTable>,
    pub corrhist: CorrectionHistories,
}

impl Tables {
    pub fn new() -> Self {
        Self {
            quiet_history: QuietHistoryTable::new(),
            tactical_history: TacticalHistoryTable::new(),
            conthist: ContHistTable::new(),
            corrhist: CorrectionHistories::new(),
        }
    }
}

type ByPlayer<T> = [T; Player::N];
type FromTo<T> = [[T; Square::N]; Square::N];
type PieceTo<T> = [[T; Square::N]; Piece::N];
type Threats<T> = [[T; 2]; 2];

#[inline]
fn threat_indices(game: &Game, mv: Move) -> (usize, usize) {
    let from_threatened = usize::from(game.threats.contains(mv.from()));
    let to_threatened = usize::from(game.threats.contains(mv.to()));

    (from_threatened, to_threatened)
}

pub struct QuietHistoryTable(ByPlayer<FromTo<Threats<HistoryEntry<{ Self::MAX }>>>>);

pub fn quiet_history_bonus(depth: Depth) -> i32 {
    min(
        depth * quiet_history_bonus_factor() - quiet_history_bonus_offset(),
        quiet_history_max_bonus(),
    )
}

impl QuietHistoryTable {
    const MAX: i16 = 8192;

    pub fn new() -> Box<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    pub fn get(&self, game: &Game, mv: Move) -> i32 {
        let (from_threatened, to_threatened) = threat_indices(game, mv);
        self.0[game.player][mv.from()][mv.to()][from_threatened][to_threatened].get()
    }

    fn update_for_move(&mut self, game: &Game, mv: Move, bonus: i32) {
        let (from_threatened, to_threatened) = threat_indices(game, mv);
        self.0[game.player][mv.from()][mv.to()][from_threatened][to_threatened].update(bonus);
    }

    pub fn update(&mut self, game: &Game, mv: Move, depth: Depth, other_quiets_tried: &MoveList) {
        let bonus = quiet_history_bonus(depth);

        self.update_for_move(game, mv, bonus);

        for other_quiet in other_quiets_tried {
            self.update_for_move(game, *other_quiet, -bonus);
        }
    }
}

pub struct TacticalHistoryTable(PieceTo<[Threats<HistoryEntry<{ Self::MAX }>>; PieceKind::N + 1]>);

pub fn capture_history_bonus(depth: Depth) -> i32 {
    min(
        depth * capture_history_bonus_factor() - capture_history_bonus_offset(),
        capture_history_max_bonus(),
    )
}

impl TacticalHistoryTable {
    pub const MAX: i16 = 8192;

    pub fn new() -> Box<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    pub fn get(&self, game: &Game, mv: Move) -> i32 {
        let capturing_piece = game.board.piece_guaranteed_at(mv.from());
        let capture_square = mv.to();
        let captured_piece = game.board.captured_piece(mv).map_or(0, |p| p as usize + 1);
        let (from_threatened, to_threatened) = threat_indices(game, mv);

        self.0[capturing_piece][capture_square][captured_piece][from_threatened][to_threatened]
            .get()
    }

    fn update_for_move(&mut self, mv: Move, game: &Game, bonus: i32) {
        let capturing_piece = game.board.piece_guaranteed_at(mv.from());
        let capture_square = mv.to();
        let captured_piece = game.board.captured_piece(mv).map_or(0, |p| p as usize + 1);
        let (from_threatened, to_threatened) = threat_indices(game, mv);

        self.0[capturing_piece][capture_square][captured_piece][from_threatened][to_threatened]
            .update(bonus);
    }

    pub fn update(&mut self, mv: Move, game: &Game, depth: Depth, other_captures_tried: &MoveList) {
        let bonus = capture_history_bonus(depth);

        self.update_for_move(mv, game, bonus);

        for other_capture in other_captures_tried {
            self.update_for_move(*other_capture, game, -bonus);
        }
    }
}

pub struct ContHistTable(PieceTo<PieceTo<HistoryEntry<{ Self::MAX }>>>);

pub fn continuation_history_bonus(depth: Depth) -> i32 {
    min(
        depth * continuation_history_bonus_factor() - continuation_history_bonus_offset(),
        continuation_history_max_bonus(),
    )
}

impl ContHistTable {
    const MAX: i16 = 16384;
    const PLIES: [usize; 3] = [1, 2, 4];

    pub fn new() -> Box<Self> {
        unsafe { Box::new_zeroed().assume_init() }
    }

    fn get_ply(
        &self,
        game: &Game,
        previous_piece_moved: Piece,
        previous_moved_to: Square,
        mv: Move,
    ) -> i32 {
        let moved = game.board.piece_guaranteed_at(mv.from());
        self.0[previous_piece_moved][previous_moved_to][moved][mv.to()].get()
    }

    pub fn get(&self, game: &Game, stack: &SearchStack, plies: u8, mv: Move) -> i32 {
        Self::PLIES
            .into_iter()
            .map(|i| {
                stack
                    .get_prev(plies, i)
                    .and_then(|s| s.mv)
                    .map_or(0, |(prev_move, prev_moved)| {
                        self.get_ply(game, prev_moved, prev_move.to(), mv)
                    })
            })
            .sum::<i32>()
    }

    fn update_for_move(
        &mut self,
        previous_piece_moved: Piece,
        previous_moved_to: Square,
        moved: Piece,
        moved_to: Square,
        total: i32,
        bonus: i32,
    ) {
        self.0[previous_piece_moved][previous_moved_to][moved][moved_to]
            .update_with_base(total, bonus);
    }

    fn update_ply(
        &mut self,
        game: &Game,
        previous_piece_moved: Piece,
        previous_move: Move,
        mv: Move,
        depth: Depth,
        quiets_tried: &MoveList,
        total: i32,
    ) {
        let bonus = continuation_history_bonus(depth);

        let moved = game.board.piece_guaranteed_at(mv.from());
        self.update_for_move(
            previous_piece_moved,
            previous_move.to(),
            moved,
            mv.to(),
            total,
            bonus,
        );

        for quiet_tried in quiets_tried {
            let try_moved = game.board.piece_guaranteed_at(quiet_tried.from());
            self.update_for_move(
                previous_piece_moved,
                previous_move.to(),
                try_moved,
                quiet_tried.to(),
                total,
                -bonus,
            );
        }
    }

    pub fn update(
        &mut self,
        game: &Game,
        stack: &SearchStack,
        plies: u8,
        mv: Move,
        depth: Depth,
        quiets_tried: &MoveList,
    ) {
        let total = self.get(game, stack, plies, mv);

        for i in Self::PLIES {
            if let Some(last_ply) = stack.get_prev(plies, i)
                && let Some((last_move, last_moved)) = last_ply.mv
            {
                self.update_ply(game, last_moved, last_move, mv, depth, quiets_tried, total);
            }
        }
    }
}

pub struct CorrectionHistories {
    pawn: Box<CorrectionHistoryTable>,
    major: Box<CorrectionHistoryTable>,
    minor: Box<CorrectionHistoryTable>,
    non_pawn: [Box<CorrectionHistoryTable>; Player::N],
    threat: Box<CorrectionHistoryTable>,
    oneply: Box<
        [[[[HistoryEntry<{ CorrectionHistoryTable::MAX }>; Square::N]; Piece::N]; Square::N];
            Piece::N],
    >,
}

impl CorrectionHistories {
    pub fn new() -> Self {
        Self {
            pawn: CorrectionHistoryTable::new(),
            major: CorrectionHistoryTable::new(),
            minor: CorrectionHistoryTable::new(),
            non_pawn: [CorrectionHistoryTable::new(), CorrectionHistoryTable::new()],
            threat: CorrectionHistoryTable::new(),
            oneply: unsafe { Box::new_zeroed().assume_init() },
        }
    }

    pub fn get(&self, game: &mut Game, ctx: &SearchContext<'_>, plies: u8) -> Eval {
        let mut corr = self.pawn.get(game.player, game.pawn_hash)
            * pawn_correction_history_weight()
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

        if let Some(their_last_ply) = ctx.stack.get_prev(plies, 1)
            && let Some(our_last_ply) = ctx.stack.get_prev(plies, 2)
            && let Some((their_last_move, their_moved_piece)) = their_last_ply.mv
            && let Some((our_last_mv, our_moved_piece)) = our_last_ply.mv
        {
            corr += Eval(
                self.oneply[our_moved_piece][our_last_mv.to()][their_moved_piece]
                    [their_last_move.to()]
                .get(),
            ) * one_ply_contcorrhist_weight();
        }

        corr / 2048
    }

    pub fn update(
        &mut self,
        game: &mut Game,
        stack: &SearchStack,
        plies: u8,
        depth: Depth,
        eval_diff: Eval,
    ) {
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

        if let Some(their_last_ply) = stack.get_prev(plies, 1)
            && let Some(our_last_ply) = stack.get_prev(plies, 2)
            && let Some((their_last_move, their_moved_piece)) = their_last_ply.mv
            && let Some((our_last_mv, our_moved_piece)) = our_last_ply.mv
        {
            let raw_bonus = eval_diff.0 * depth.as_i32() / 8;
            let bonus = i32::clamp(
                raw_bonus,
                -CorrectionHistoryTable::MAX_UPDATE,
                CorrectionHistoryTable::MAX_UPDATE,
            );

            self.oneply[our_moved_piece][our_last_mv.to()][their_moved_piece][their_last_move.to()]
                .update(bonus);
        }
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
        unsafe { Box::new_zeroed().assume_init() }
    }

    #[expect(clippy::cast_possible_truncation, reason = "u64 to usize")]
    pub fn get(&self, player: Player, key: ZobristHash) -> Eval {
        Eval(self.0[player][key.0 as usize % CORRECTION_HISTORY_SIZE].get())
    }

    #[expect(clippy::cast_possible_truncation, reason = "u64 to usize")]
    pub fn update(&mut self, player: Player, key: ZobristHash, depth: Depth, eval_diff: Eval) {
        let raw_bonus = eval_diff.0 * depth.as_i32() / 8;
        let bonus = i32::clamp(raw_bonus, -Self::MAX_UPDATE, Self::MAX_UPDATE);

        self.0[player][key.0 as usize % CORRECTION_HISTORY_SIZE].update(bonus);
    }
}
