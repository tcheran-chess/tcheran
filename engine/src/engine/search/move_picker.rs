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
        search::tables::{CaptureHistoryTable, Tables},
        see::see,
    },
};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum GenStage {
    BestMove,
    GenTacticals,
    GoodTacticals,
    GenQuiets,
    Killer,
    CounterMove,
    BadTacticals,
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
    only_tacticals: bool,

    pub stage: GenStage,

    bad_tacticals: ArrayVec<MoveEntry, MAX_LEGAL_MOVES>,
}

impl MovePicker {
    pub fn new(previous_best_move: Option<Move>) -> Self {
        Self {
            moves: ArrayVec::new(),
            movegencache: MovegenCache::new(),
            previous_best_move,
            only_tacticals: false,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub fn new_loud(previous_best_move: Option<Move>) -> Self {
        Self {
            moves: ArrayVec::new(),
            movegencache: MovegenCache::new(),
            previous_best_move,
            only_tacticals: true,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub(crate) fn next(&mut self, game: &Game, tables: &Tables, plies: u8) -> Option<Move> {
        use GenStage::*;

        if self.stage == BestMove {
            self.stage = GenTacticals;

            if let Some(previous_best_move) = self.previous_best_move {
                return Some(previous_best_move);
            }
        }

        if self.stage == GenTacticals {
            self.stage = GoodTacticals;

            movegen::generate_tacticals(game, &mut self.movegencache, &mut |mv| {
                self.moves.push(MoveEntry { mv, score: 0 });
            });

            for entry in &mut self.moves {
                entry.score = score_tactical(game, entry.mv, tables);
            }
        }

        if self.stage == GoodTacticals {
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

            self.stage = if self.only_tacticals { Done } else { GenQuiets };
        }

        if self.stage == GenQuiets {
            self.stage = Killer;

            if !self.only_tacticals {
                movegen::generate_quiets(game, &self.movegencache, &mut |mv| {
                    self.moves.push(MoveEntry { mv, score: 0 });
                });
            }
        }

        if self.stage == Killer {
            self.stage = CounterMove;

            if !self.only_tacticals
                && let Some(killer) = tables.killer_moves.get(plies)
                && self.moves.remove(killer)
                && Some(killer) != self.previous_best_move
            {
                return Some(killer);
            }
        }

        if self.stage == CounterMove {
            self.stage = BadTacticals;

            if !self.only_tacticals
                && let Some(previous_move) = game.history.last().and_then(|h| h.mv)
                && let Some(counter_move) = tables.countermoves.get(game.player, previous_move)
                && self.moves.remove(counter_move)
                && Some(counter_move) != self.previous_best_move
            {
                return Some(counter_move);
            }
        }

        if self.stage == BadTacticals {
            while let Some(entry) = self.bad_tacticals.next_best() {
                if Some(entry.mv) == self.previous_best_move {
                    continue;
                }

                return Some(entry.mv);
            }

            self.stage = if self.only_tacticals {
                Done
            } else {
                ScoreQuiets
            };
        }

        if self.stage == ScoreQuiets {
            self.stage = Quiets;

            if !self.only_tacticals {
                for entry in &mut self.moves {
                    entry.score = score_quiet(game, entry.mv, tables);
                }
            }
        }

        if self.stage == Quiets {
            if !self.only_tacticals {
                while let Some(entry) = self.moves.next_best() {
                    if Some(entry.mv) == self.previous_best_move {
                        continue;
                    }

                    return Some(entry.mv);
                }
            }

            self.stage = Done;
        }

        if self.stage == Done {
            return None;
        }

        unreachable!()
    }

    pub fn yield_only_tacticals(&mut self) {
        self.only_tacticals = true;
    }
}

const MVV_ORDER: [i32; PieceKind::N] = [
    0,
    CaptureHistoryTable::MAX,
    CaptureHistoryTable::MAX * 2,
    CaptureHistoryTable::MAX * 3,
    CaptureHistoryTable::MAX * 4,
    CaptureHistoryTable::MAX * 5,
];

pub fn score_tactical(game: &Game, mv: Move, tables: &Tables) -> i32 {
    let moved_piece = game.board.piece_guaranteed_at(mv.src());

    if mv.is_capture() {
        let captured_piece_kind = if mv.is_en_passant() {
            PieceKind::Pawn
        } else {
            game.board.piece_guaranteed_at(mv.dst()).kind
        };

        return MVV_ORDER[captured_piece_kind]
            + tables.capture_history.get(
                game.player,
                moved_piece.kind,
                mv.dst(),
                captured_piece_kind,
            );
    }

    // For other tactials (i.e. promotions), explore highest value pieces first
    i32::MAX - 100 - moved_piece.kind as i32
}

pub fn score_quiet(game: &Game, mv: Move, tables: &Tables) -> i32 {
    tables.quiet_history.get(game, mv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::square::squares::all::*;

    #[test]
    fn test_movepicker_does_not_double_yield_best_move() {
        crate::init();

        let game = Game::new();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_picker = MovePicker::new(Some(Move::quiet(G1, F3)));

        while let Some(m) = move_picker.next(&game, &Tables::new(), 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn test_movepicker_does_not_skip_bad_tacticals_when_no_good_tacticals() {
        crate::init();

        let game = Game::from_fen("rnbqkbnr/pp1ppppp/8/2p5/3P4/5N2/PPP1PPPP/RNBQKB1R b KQkq - 0 2")
            .unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(None);

        while let Some(m) = move_provider.next(&game, &Tables::new(), 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 23);
    }

    #[test]
    fn test_movepicker_does_not_return_to_start_if_no_bad_tacticals() {
        crate::init();

        let game =
            Game::from_fen("rnbqkb1r/ppp1pppp/5n2/3p4/4P3/2N5/PPPP1PPP/R1BQKBNR w KQkq - 0 3")
                .unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(None);

        while let Some(m) = move_provider.next(&game, &Tables::new(), 0) {
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

        while let Some(m) = move_provider.next(&game, &Tables::new(), 0) {
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
        let mut move_provider = MovePicker::new_loud(None);

        while let Some(m) = move_provider.next(&game, &Tables::new(), 0) {
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

        let mut tables = Tables::new();
        tables.killer_moves.set(0, Move::quiet(B7, D5));

        while let Some(m) = move_provider.next(&game, &tables, 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 4);
    }
}
