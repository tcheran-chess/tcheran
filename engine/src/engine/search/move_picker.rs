use crate::{
    chess::{
        arrayvec::ArrayVec,
        game::Game,
        movegen,
        movegen::MovegenCache,
        moves::{MAX_LEGAL_MOVES, Move},
        piece::PieceKind,
    },
    engine::{
        eval::Eval,
        search::{SearchContext, tables::HistoryTable},
        see::see,
    },
};

#[derive(Eq, PartialEq)]
enum GenStage {
    BestMove,
    GenCaptures,
    GoodCaptures,
    GenQuiets,
    Killer1,
    Killer2,
    CounterMove,
    BadCaptures,
    ScoreQuiets,
    Quiets,
    Done,
}

#[derive(Debug, Clone, Copy)]
struct MoveEntry {
    mv: Move,
    score: i32,
}

trait MoveListExt {
    fn next_best(&mut self) -> Option<MoveEntry>;
    fn remove(&mut self, mv: Move) -> bool;
}

impl MoveListExt for ArrayVec<MoveEntry, MAX_LEGAL_MOVES> {
    fn next_best(&mut self) -> Option<MoveEntry> {
        if self.is_empty() {
            return None;
        }

        let idx = self
            .iter()
            .enumerate()
            .max_by_key(|&(_, entry)| entry.score)
            .map_or(0, |(i, _)| i);

        Some(self.swap_remove(idx))
    }

    fn remove(&mut self, mv: Move) -> bool {
        let idx = self
            .iter()
            .enumerate()
            .find(|&(_, entry)| entry.mv == mv)
            .map(|(i, _)| i);

        let Some(idx) = idx else {
            return false;
        };

        self.swap_remove(idx);
        true
    }
}

pub struct MovePicker {
    moves: ArrayVec<MoveEntry, MAX_LEGAL_MOVES>,
    movegencache: MovegenCache,
    previous_best_move: Option<Move>,
    only_captures: bool,

    stage: GenStage,

    bad_tacticals: ArrayVec<MoveEntry, MAX_LEGAL_MOVES>,
}

impl MovePicker {
    pub fn new(previous_best_move: Option<Move>) -> Self {
        Self {
            moves: ArrayVec::new(),
            movegencache: MovegenCache::new(),
            previous_best_move,
            only_captures: false,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub fn new_loud() -> Self {
        Self {
            moves: ArrayVec::new(),
            movegencache: MovegenCache::new(),
            previous_best_move: None,
            only_captures: true,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub(crate) fn next(&mut self, game: &Game, ctx: &SearchContext<'_>, plies: u8) -> Option<Move> {
        use GenStage::*;

        if self.stage == BestMove {
            self.stage = GenCaptures;

            if let Some(previous_best_move) = self.previous_best_move {
                return Some(previous_best_move);
            }
        }

        if self.stage == GenCaptures {
            self.stage = GoodCaptures;

            movegen::generate_captures(game, &mut self.movegencache, &mut |mv| {
                self.moves.push(MoveEntry { mv, score: 0 });
            });

            for entry in &mut self.moves {
                entry.score = score_tactical(game, entry.mv);
            }
        }

        if self.stage == GoodCaptures {
            while let Some(entry) = self.moves.next_best() {
                if Some(entry.mv) == self.previous_best_move {
                    continue;
                }

                if !see(game, entry.mv, Eval(0)) {
                    self.bad_tacticals.push(entry);
                    continue;
                }

                return Some(entry.mv);
            }

            self.stage = if self.only_captures { Done } else { GenQuiets };
        }

        if self.stage == GenQuiets {
            self.stage = Killer1;

            movegen::generate_quiets(game, &self.movegencache, &mut |mv| {
                self.moves.push(MoveEntry { mv, score: 0 });
            });
        }

        if self.stage == Killer1 {
            self.stage = Killer2;

            if let Some(killer) = ctx.killer_moves.get_0(plies)
                && self.moves.remove(killer)
                && Some(killer) != self.previous_best_move
            {
                return Some(killer);
            }
        }

        if self.stage == Killer2 {
            self.stage = CounterMove;

            if let Some(killer) = ctx.killer_moves.get_1(plies)
                && self.moves.remove(killer)
                && Some(killer) != self.previous_best_move
            {
                return Some(killer);
            }
        }

        if self.stage == CounterMove {
            self.stage = BadCaptures;

            if let Some(previous_move) = game.history.last().and_then(|h| h.mv)
                && let Some(counter_move) = ctx.countermove_table.get(game.player, previous_move)
                && self.moves.remove(counter_move)
                && Some(counter_move) != self.previous_best_move
            {
                return Some(counter_move);
            }
        }

        if self.stage == BadCaptures {
            while let Some(entry) = self.bad_tacticals.next_best() {
                if Some(entry.mv) == self.previous_best_move {
                    continue;
                }

                return Some(entry.mv);
            }

            self.stage = if self.only_captures {
                Done
            } else {
                ScoreQuiets
            };
        }

        if self.stage == ScoreQuiets {
            self.stage = Quiets;

            for entry in &mut self.moves {
                entry.score = score_quiet(game, entry.mv, ctx.history_table);
            }
        }

        if self.stage == Quiets {
            while let Some(entry) = self.moves.next_best() {
                if Some(entry.mv) == self.previous_best_move {
                    continue;
                }

                return Some(entry.mv);
            }

            self.stage = Done;
        }

        if self.stage == Done {
            return None;
        }

        unreachable!()
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "Guaranteed to fit inside an i32"
)]
const PIECES: i32 = PieceKind::N as i32;

const MVV_ORDER: [i32; PieceKind::N] = [0, PIECES, PIECES * 2, PIECES * 3, PIECES * 4, PIECES * 5];
const LVA_ORDER: [i32; PieceKind::N] = [5, 4, 3, 2, 1, 0];

#[inline]
pub fn mvv_lva(victim: PieceKind, attacker: PieceKind) -> i32 {
    let victim_score = MVV_ORDER[victim];
    let attacker_score = LVA_ORDER[attacker];

    victim_score + attacker_score
}

pub fn score_tactical(game: &Game, mv: Move) -> i32 {
    let moved_piece = game.board.piece_guaranteed_at(mv.src());

    if mv.is_capture() {
        if mv.is_en_passant() {
            return mvv_lva(PieceKind::Pawn, PieceKind::Pawn);
        }

        let captured_piece = game.board.piece_guaranteed_at(mv.dst());
        return mvv_lva(captured_piece.kind, moved_piece.kind);
    }

    // For other tactials (i.e. promotions, explore highest value pieces first)
    i32::MAX - LVA_ORDER[moved_piece.kind]
}

pub fn score_quiet(game: &Game, mv: Move, history: &HistoryTable) -> i32 {
    history.get(game.player, mv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chess::{game::Game, square::squares::all::*},
        engine::{
            options::EngineOptions,
            search::{PersistentState, TimeControl, time_control::TimeStrategy},
        },
    };

    #[test]
    fn test_movepicker_does_not_double_yield_best_move() {
        crate::init();

        let game = Game::new();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_picker = MovePicker::new(Some(Move::quiet(G1, F3)));

        let mut persistent_state = PersistentState::new(16);
        let options = EngineOptions::default();
        let mut time_strategy = TimeStrategy::new(&game, &TimeControl::Infinite, None, &options);
        let ctx = SearchContext::new(&mut persistent_state, &mut time_strategy, &options);

        while let Some(m) = move_picker.next(&game, &ctx, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn test_movepicker_does_not_skip_bad_captures_when_no_good_captures() {
        crate::init();

        let game = Game::from_fen("rnbqkbnr/pp1ppppp/8/2p5/3P4/5N2/PPP1PPPP/RNBQKB1R b KQkq - 0 2")
            .unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(None);

        let mut persistent_state = PersistentState::new(16);
        let options = EngineOptions::default();
        let mut time_strategy = TimeStrategy::new(&game, &TimeControl::Infinite, None, &options);
        let ctx = SearchContext::new(&mut persistent_state, &mut time_strategy, &options);

        while let Some(m) = move_provider.next(&game, &ctx, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 23);
    }

    #[test]
    fn test_movepicker_does_not_return_to_start_if_no_bad_captures() {
        crate::init();

        let game =
            Game::from_fen("rnbqkb1r/ppp1pppp/5n2/3p4/4P3/2N5/PPPP1PPP/R1BQKBNR w KQkq - 0 3")
                .unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(None);

        let mut persistent_state = PersistentState::new(16);
        let options = EngineOptions::default();
        let mut time_strategy = TimeStrategy::new(&game, &TimeControl::Infinite, None, &options);
        let ctx = SearchContext::new(&mut persistent_state, &mut time_strategy, &options);

        while let Some(m) = move_provider.next(&game, &ctx, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 33);
    }

    #[test]
    fn test_movepicker_yields_en_passant_correctly() {
        crate::init();

        let game =
            Game::from_fen("r1bqkb1r/ppp1pppp/2n2n2/2Pp4/8/5N2/PP1PPPPP/RNBQKB1R w KQkq d6 0 4")
                .unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(None);

        let mut persistent_state = PersistentState::new(16);
        let options = EngineOptions::default();
        let mut time_strategy = TimeStrategy::new(&game, &TimeControl::Infinite, None, &options);
        let ctx = SearchContext::new(&mut persistent_state, &mut time_strategy, &options);

        while let Some(m) = move_provider.next(&game, &ctx, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 24);
    }

    #[test]
    fn test_movepicker_generates_caps_in_quiescence() {
        crate::init();

        let game =
            Game::from_fen("rnb1kbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2").unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new_loud();

        let mut persistent_state = PersistentState::new(16);
        let options = EngineOptions::default();
        let mut time_strategy = TimeStrategy::new(&game, &TimeControl::Infinite, None, &options);
        let ctx = SearchContext::new(&mut persistent_state, &mut time_strategy, &options);

        while let Some(m) = move_provider.next(&game, &ctx, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 1);
    }

    #[test]
    fn test_movepicker_bug_after_see_move_ordering_1() {
        crate::init();

        let game = Game::from_fen("r2k3r/1b4bq/8/3R4/8/8/7B/4K2R b K - 3 2").unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(Some(Move::quiet(D8, E7)));

        let mut persistent_state = PersistentState::new(16);
        let options = EngineOptions::default();
        let mut time_strategy = TimeStrategy::new(&game, &TimeControl::Infinite, None, &options);
        let mut ctx = SearchContext::new(&mut persistent_state, &mut time_strategy, &options);

        ctx.killer_moves.try_push(0, Move::quiet(B7, D5));
        ctx.killer_moves.try_push(0, Move::quiet(D8, E8));

        while let Some(m) = move_provider.next(&game, &ctx, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 4);
    }

    fn score_mvv_lva(game: &Game, mv: Move) -> i32 {
        let moved_piece = game.board.piece_guaranteed_at(mv.src());
        let captured_piece = game.board.piece_at(mv.dst());

        match captured_piece {
            Some(captured_piece) => mvv_lva(captured_piece.kind, moved_piece.kind),
            None => 0,
        }
    }

    #[test]
    fn test_mvv_lva() {
        crate::init();

        let game = Game::from_fen("k3B3/8/n1q1R1r1/1P6/1NQn4/7P/2r5/5K2 w - - 0 1").unwrap();
        let mut moves: Vec<MoveEntry> = game
            .moves()
            .iter()
            .filter(|m| m.is_capture())
            .copied()
            .map(|mv| MoveEntry { mv, score: 0 })
            .collect();

        for mv in &mut moves {
            mv.score = score_mvv_lva(&game, mv.mv);
        }

        moves.sort_unstable_by_key(|m| -m.score);

        assert_eq!(moves[0].mv, Move::capture(B5, C6)); // PxQ
        assert_eq!(moves[1].mv, Move::capture(B4, C6)); // NxQ
        assert_eq!(moves[2].mv, Move::capture(E8, C6)); // BxQ
        assert_eq!(moves[3].mv, Move::capture(E6, C6)); // RxQ
        assert_eq!(moves[4].mv, Move::capture(C4, C6)); // QxQ
        assert_eq!(moves[5].mv, Move::capture(B4, C2)); // NxR
        assert_eq!(moves[6].mv, Move::capture(E8, G6)); // BxR
        assert_eq!(moves[7].mv, Move::capture(E6, G6)); // RxR
        assert_eq!(moves[8].mv, Move::capture(C4, C2)); // QxR
        assert_eq!(moves[9].mv, Move::capture(B5, A6)); // PxN
        assert_eq!(moves[10].mv, Move::capture(B4, A6)); // NxN
        assert_eq!(moves[11].mv, Move::capture(C4, D4)); // QxN
    }
}
