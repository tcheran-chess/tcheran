use crate::{
    chess::{
        Game, Move, PieceKind,
        moves::{MAX_LEGAL_MOVES, generate_quiets, generate_tacticals},
    },
    engine::{
        eval::Eval,
        params::*,
        search::{SearchStack, tables::Tables},
        see::{see, see_value},
    },
    util::arrayvec::ArrayVec,
};

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub enum GenStage {
    BestMove,
    GenTacticals,
    GoodTacticals,
    Killer,
    GenQuiets,
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

pub struct MovePicker {
    moves: ArrayVec<MoveEntry, MAX_LEGAL_MOVES>,
    previous_best_move: Option<Move>,
    skip_quiets: bool,

    pub stage: GenStage,

    bad_tacticals: ArrayVec<MoveEntry, MAX_LEGAL_MOVES>,
}

impl MovePicker {
    pub fn new(previous_best_move: Option<Move>) -> Self {
        Self {
            moves: ArrayVec::new(),
            previous_best_move,
            skip_quiets: false,

            stage: GenStage::BestMove,
            bad_tacticals: ArrayVec::new(),
        }
    }

    pub fn next(
        &mut self,
        game: &Game,
        tables: &Tables,
        stack: &SearchStack,
        plies: u8,
    ) -> Option<Move> {
        use GenStage::*;

        if self.stage == BestMove {
            self.stage = GenTacticals;

            if let Some(previous_best_move) = self.previous_best_move
                && game.is_legal(previous_best_move)
            {
                return Some(previous_best_move);
            }
        }

        if self.stage == GenTacticals {
            self.stage = GoodTacticals;

            generate_tacticals(game, &mut |mv| {
                if Some(mv) == self.previous_best_move {
                    return;
                }

                self.moves.push(MoveEntry { mv, score: 0 });
            });

            for entry in &mut self.moves {
                entry.score = score_tactical(game, entry.mv, tables);
            }
        }

        if self.stage == GoodTacticals {
            while let Some(entry) = next_best(&mut self.moves) {
                // If a move has good history, we can be more sure it should be searched
                // early even if its SEE score is poor.
                let threshold = -entry.score / 4;

                if !see(game, entry.mv, Eval(threshold)) {
                    self.bad_tacticals.push(entry);
                    continue;
                }

                return Some(entry.mv);
            }

            self.stage = Killer;
        }

        if self.stage == Killer {
            self.stage = GenQuiets;

            if !self.skip_quiets
                && let Some(killer) = tables.killer_moves.get(plies)
                && Some(killer) != self.previous_best_move
                && game.is_legal(killer)
            {
                return Some(killer);
            }
        }

        if self.stage == GenQuiets {
            self.stage = ScoreQuiets;

            if !self.skip_quiets {
                generate_quiets(game, &mut |mv| {
                    if Some(mv) == self.previous_best_move {
                        return;
                    }

                    if let Some(killer) = tables.killer_moves.get(plies)
                        && mv == killer
                    {
                        return;
                    }

                    self.moves.push(MoveEntry { mv, score: 0 });
                });
            }
        }

        if self.stage == ScoreQuiets {
            self.stage = Quiets;

            if !self.skip_quiets {
                for entry in &mut self.moves {
                    entry.score = score_quiet(game, entry.mv, tables, stack, plies);
                }
            }
        }

        if self.stage == Quiets {
            if !self.skip_quiets
                && let Some(entry) = next_best(&mut self.moves)
            {
                return Some(entry.mv);
            }

            self.stage = BadTacticals;
        }

        if self.stage == BadTacticals {
            if let Some(entry) = next_best(&mut self.bad_tacticals) {
                return Some(entry.mv);
            }

            self.stage = Done;
        }

        if self.stage == Done {
            return None;
        }

        unreachable!()
    }

    pub fn skip_quiets(&mut self) {
        self.skip_quiets = true;
    }
}

fn next_best(moves: &mut ArrayVec<MoveEntry, MAX_LEGAL_MOVES>) -> Option<MoveEntry> {
    if moves.is_empty() {
        return None;
    }

    let idx = moves
        .iter()
        .enumerate()
        .max_by_key(|&(_, entry)| entry.score)
        .map_or(0, |(i, _)| i);

    Some(moves.swap_remove(idx))
}

pub fn score_tactical(game: &Game, mv: Move, tables: &Tables) -> i32 {
    let mut score = 0;

    if let Some(captured_piece) = game.board.captured_piece(mv) {
        score += see_value(captured_piece).0;
    }

    if let Some(promotion_piece) = mv.promotion() {
        score += see_value(promotion_piece.piece()).0 - see_value(PieceKind::Pawn).0;
    }

    // Capture history max is 8192, so divide by 8 so max is roughly
    // equivalent to the see_value of a queen.
    score += tables.tactical_history.get(game, mv) / 8;

    score
}

pub fn score_quiet(game: &Game, mv: Move, tables: &Tables, stack: &SearchStack, plies: u8) -> i32 {
    tables.quiet_history.get(game, mv)
        + tables.conthist.get(game, stack, plies, mv)
        + movepick_direct_check_weight() * i32::from(game.is_direct_check(mv))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::squares::all::*;

    #[test]
    fn test_movepicker_does_not_double_yield_best_move() {
        crate::init();

        let game = Game::new();

        let mut moves: Vec<Move> = Vec::new();
        let mut move_picker = MovePicker::new(Some(Move::quiet(G1, F3)));

        while let Some(m) = move_picker.next(&game, &Tables::new(), &SearchStack::new(), 0) {
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

        while let Some(m) = move_provider.next(&game, &Tables::new(), &SearchStack::new(), 0) {
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

        while let Some(m) = move_provider.next(&game, &Tables::new(), &SearchStack::new(), 0) {
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

        while let Some(m) = move_provider.next(&game, &Tables::new(), &SearchStack::new(), 0) {
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
        let mut move_provider = MovePicker::new(None);
        move_provider.skip_quiets();

        while let Some(m) = move_provider.next(&game, &Tables::new(), &SearchStack::new(), 0) {
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

        while let Some(m) = move_provider.next(&game, &tables, &SearchStack::new(), 0) {
            moves.push(m);
        }

        assert_eq!(moves.len(), 4);
    }
}
