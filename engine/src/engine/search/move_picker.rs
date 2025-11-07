use crate::{
    chess::{
        game::Game,
        movegen,
        movegen::MovegenCache,
        moves::{Move, MoveList},
    },
    engine::search::{
        SearchContext, move_ordering,
        move_ordering::{score_quiet, score_tactical},
    },
};

const MAX_MOVES: usize = u8::MAX as usize;

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

pub struct MovePicker {
    moves: MoveList,
    movegencache: MovegenCache,
    scores: [i32; MAX_MOVES],
    previous_best_move: Option<Move>,
    only_captures: bool,

    stage: GenStage,
    idx: usize,
    captures_end: usize,
    first_bad_capture: Option<usize>,
    first_quiet: usize,
}

impl MovePicker {
    pub fn new(previous_best_move: Option<Move>) -> Self {
        Self {
            moves: MoveList::new(),
            movegencache: MovegenCache::new(),
            scores: [0; MAX_MOVES],
            previous_best_move,
            only_captures: false,

            stage: GenStage::BestMove,
            idx: 0,
            captures_end: 0,
            first_bad_capture: None,
            first_quiet: 0,
        }
    }

    pub fn new_loud() -> Self {
        Self {
            moves: MoveList::new(),
            movegencache: MovegenCache::new(),
            scores: [0; MAX_MOVES],
            previous_best_move: None,
            only_captures: true,

            stage: GenStage::BestMove,
            idx: 0,
            captures_end: 0,
            first_bad_capture: None,
            first_quiet: 0,
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

            movegen::generate_captures(game, &mut self.moves, &mut self.movegencache);

            self.captures_end = self.moves.len();
            self.first_quiet = self.moves.len();

            for i in 0..self.moves.len() {
                self.scores[i] = score_tactical(game, *self.moves.get(i));
            }
        }

        if self.stage == GoodCaptures {
            if let Some((mv, score)) = self.next_best_move(self.captures_end) {
                // If the move we just picked was a losing capture, we're going to skip the rest of the captures.
                // Record that, and skip the remainder of the captures since we'll be trying quiet moves next.
                if score < move_ordering::GOOD_CAPTURE_SCORE {
                    self.first_bad_capture = Some(self.idx - 1);
                    self.idx = self.captures_end;
                } else {
                    return Some(mv);
                }
            }

            if self.only_captures {
                self.stage = Done;
            } else {
                self.stage = GenQuiets;
            }
        }

        if self.stage == GenQuiets {
            self.stage = Killer1;

            movegen::generate_quiets(game, &mut self.moves, &self.movegencache);
        }

        if self.stage == Killer1 {
            self.stage = Killer2;

            if let Some(killer1) = ctx.killer_moves.get_0(plies) {
                for i in self.first_quiet..self.moves.len() {
                    if *self.moves.get(i) == killer1 {
                        self.moves.swap(self.first_quiet, i);
                        self.first_quiet += 1;

                        if Some(killer1) != self.previous_best_move {
                            return Some(killer1);
                        }
                    }
                }
            }
        }

        if self.stage == Killer2 {
            self.stage = CounterMove;

            if let Some(killer2) = ctx.killer_moves.get_1(plies) {
                for i in self.first_quiet..self.moves.len() {
                    if *self.moves.get(i) == killer2 {
                        self.moves.swap(self.first_quiet, i);
                        self.first_quiet += 1;

                        if Some(killer2) != self.previous_best_move {
                            return Some(killer2);
                        }
                    }
                }
            }
        }

        if self.stage == CounterMove {
            match self.first_bad_capture {
                // If we didn't see any bad captures before, we can skip straight to the end
                None => self.stage = ScoreQuiets,

                // If we saw any bad captures, go back and try those too
                Some(first_bad_capture_idx) => {
                    self.idx = first_bad_capture_idx;
                    self.stage = BadCaptures;
                }
            }

            if let Some(previous_move) = game.history.last().and_then(|h| h.mv) {
                if let Some(counter_move) = ctx.countermove_table.get(game.player, previous_move) {
                    for i in self.first_quiet..self.moves.len() {
                        if *self.moves.get(i) == counter_move {
                            self.moves.swap(self.first_quiet, i);
                            self.first_quiet += 1;

                            if Some(counter_move) != self.previous_best_move {
                                return Some(counter_move);
                            }
                        }
                    }
                }
            }
        }

        if self.stage == BadCaptures {
            if let Some((mv, _)) = self.next_best_move(self.captures_end) {
                return Some(mv);
            }

            self.stage = if self.only_captures {
                Done
            } else {
                ScoreQuiets
            };
        }

        if self.stage == ScoreQuiets {
            self.stage = Quiets;
            self.idx = self.first_quiet;

            for i in self.idx..self.moves.len() {
                self.scores[i] = score_quiet(game, *self.moves.get(i), ctx.history_table);
            }
        }

        if self.stage == Quiets {
            if let Some((mv, _)) = self.next_best_move(self.moves.len()) {
                return Some(mv);
            }

            self.stage = Done;
        }

        if self.stage == Done {
            return None;
        }

        unreachable!()
    }

    fn next_best_move(&mut self, limit: usize) -> Option<(Move, i32)> {
        loop {
            if self.idx == limit {
                return None;
            }

            // Start with the next move that we haven't tried yet
            let mut best_move_idx = self.idx;
            let mut best_move_score = self.scores[self.idx];

            // Check if there's a better move later on in the list
            for i in self.idx + 1..limit {
                let move_score = self.scores[i];

                if move_score > best_move_score {
                    best_move_score = move_score;
                    best_move_idx = i;
                }
            }

            let best_move = *self.moves.get(best_move_idx);

            // Move our best move to the start of the moves we haven't tried
            self.moves.swap(self.idx, best_move_idx);
            self.scores.swap(self.idx, best_move_idx);

            self.idx += 1;

            // We always return the best move first, before doing move generation.
            // We don't want to return it again from the movelist, so skip it.
            if Some(best_move) == self.previous_best_move {
                continue;
            }

            return Some((best_move, best_move_score));
        }
    }
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
}
