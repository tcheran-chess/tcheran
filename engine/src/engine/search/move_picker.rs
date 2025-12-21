use crate::{
    chess::{
        arrayvec::ArrayVec,
        game::Game,
        movegen,
        moves::{MAX_LEGAL_MOVES, Move},
        piece::PieceKind,
    },
    engine::{
        eval::Eval,
        search::{Params, SearchStack, tables::Tables},
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
    ScoreQuiets,
    Quiets,
    BadTacticals,
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
    previous_best_move: Option<Move>,
    only_tacticals: bool,

    see_margin: Eval,

    pub stage: GenStage,

    bad_tacticals: ArrayVec<MoveEntry, MAX_LEGAL_MOVES>,
}

impl MovePicker {
    pub fn new(previous_best_move: Option<Move>, see_margin: Eval) -> Self {
        Self {
            moves: ArrayVec::new(),
            previous_best_move,
            only_tacticals: false,
            see_margin,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub fn new_loud(previous_best_move: Option<Move>, see_margin: Eval) -> Self {
        Self {
            moves: ArrayVec::new(),
            previous_best_move,
            only_tacticals: true,
            see_margin,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub fn next(
        &mut self,
        game: &Game,
        tables: &Tables,
        stack: &SearchStack,
        params: &Params,
        plies: u8,
    ) -> Option<Move> {
        use GenStage::*;

        if self.stage == BestMove {
            self.stage = GenTacticals;

            if let Some(previous_best_move) = self.previous_best_move {
                return Some(previous_best_move);
            }
        }

        if self.stage == GenTacticals {
            self.stage = GoodTacticals;

            movegen::generate_tacticals(game, &mut |mv| {
                self.moves.push(MoveEntry { mv, score: 0 });
            });

            for entry in &mut self.moves {
                entry.score = score_tactical(game, entry.mv, tables, params);
            }
        }

        if self.stage == GoodTacticals {
            while let Some(entry) = self.moves.next_best() {
                if Some(entry.mv) == self.previous_best_move {
                    continue;
                }

                if !see(game, entry.mv, self.see_margin, params) {
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
                movegen::generate_quiets(game, &mut |mv| {
                    self.moves.push(MoveEntry { mv, score: 0 });
                });
            }
        }

        if self.stage == Killer {
            self.stage = ScoreQuiets;

            if !self.only_tacticals
                && let Some(killer) = tables.killer_moves.get(plies)
                && self.moves.remove(killer)
                && Some(killer) != self.previous_best_move
            {
                return Some(killer);
            }
        }

        if self.stage == ScoreQuiets {
            self.stage = Quiets;

            if !self.only_tacticals {
                for entry in &mut self.moves {
                    entry.score = score_quiet(game, entry.mv, tables, stack, plies);
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

            self.stage = BadTacticals;
        }

        if self.stage == BadTacticals {
            while let Some(entry) = self.bad_tacticals.next_best() {
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

    pub fn yield_only_tacticals(&mut self) {
        self.only_tacticals = true;
    }
}

pub fn score_tactical(game: &Game, mv: Move, tables: &Tables, params: &Params) -> i32 {
    let moved_piece = game.board.piece_guaranteed_at(mv.src());

    let mut score = 0;

    if mv.is_capture() {
        let captured_piece_kind = if mv.is_en_passant() {
            PieceKind::Pawn
        } else {
            game.board.piece_guaranteed_at(mv.dst()).kind
        };

        score += params.see_values[captured_piece_kind].0;

        // Capture history max is 8192, so divide by 8 so max is roughly
        // equivalent to the see_value of a queen.
        score += tables.capture_history.get(
            game.player,
            moved_piece.kind,
            mv.dst(),
            captured_piece_kind,
        ) / 8;
    }

    if mv.is_promotion() {
        score += params.see_values[PieceKind::Queen].0 - params.see_values[PieceKind::Pawn].0;
    }

    score
}

pub fn score_quiet(game: &Game, mv: Move, tables: &Tables, stack: &SearchStack, plies: u8) -> i32 {
    let conthist1_bonus = stack
        .get_prev(plies, 1)
        .and_then(|s| s.mv)
        .map_or(0, |(prev_move, prev_moved)| {
            tables.conthist.get(game, prev_moved, prev_move.dst(), mv)
        });

    let conthist2_bonus = stack
        .get_prev(plies, 2)
        .and_then(|s| s.mv)
        .map_or(0, |(prev_move, prev_moved)| {
            tables.conthist.get(game, prev_moved, prev_move.dst(), mv)
        });

    tables.quiet_history.get(game, mv) + conthist1_bonus + conthist2_bonus
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
        let mut move_picker = MovePicker::new(Some(Move::quiet(G1, F3)), Eval(0));

        while let Some(m) =
            move_picker.next(&game, &Tables::new(), &SearchStack::new(), &Params::default(), 0)
        {
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
        let mut move_provider = MovePicker::new(None, Eval(0));

        while let Some(m) =
            move_provider.next(&game, &Tables::new(), &SearchStack::new(), &Params::default(), 0)
        {
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
        let mut move_provider = MovePicker::new(None, Eval(0));

        while let Some(m) =
            move_provider.next(&game, &Tables::new(), &SearchStack::new(), &Params::default(), 0)
        {
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
        let mut move_provider = MovePicker::new(None, Eval(0));

        while let Some(m) =
            move_provider.next(&game, &Tables::new(), &SearchStack::new(), &Params::default(), 0)
        {
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
        let mut move_provider = MovePicker::new_loud(None, Eval(0));

        while let Some(m) =
            move_provider.next(&game, &Tables::new(), &SearchStack::new(), &Params::default(), 0)
        {
            moves.push(m);
        }

        assert_eq!(moves.len(), 1);
    }

    #[test]
    fn test_movepicker_bug_after_see_move_ordering_1() {
        crate::init();

        let game = Game::from_fen("r2k3r/1b4bq/8/3R4/8/8/7B/4K2R b K - 3 2").unwrap();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_provider = MovePicker::new(Some(Move::quiet(D8, E7)), Eval(0));

        let mut tables = Tables::new();
        tables.killer_moves.set(0, Move::quiet(B7, D5));

        while let Some(m) =
            move_provider.next(&game, &tables, &SearchStack::new(), &Params::default(), 0)
        {
            moves.push(m);
        }

        assert_eq!(moves.len(), 4);
    }
}
