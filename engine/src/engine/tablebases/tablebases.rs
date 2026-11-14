use std::{cmp::Reverse, collections::HashSet};

use crate::{
    chess::{MAX_LEGAL_MOVES, prelude::*},
    engine::{
        eval::Eval,
        search::{RootMoves, RootTbInfo},
        tablebases::fathom,
    },
    util::arrayvec::ArrayVec,
};

#[derive(Clone)]
pub struct Tablebase {
    pub is_enabled: bool,
    pub n_men: u8,
}

impl Tablebase {
    pub fn new() -> Self {
        Self {
            is_enabled: false,
            n_men: 0,
        }
    }

    pub fn set_paths(&mut self, path: &str) {
        let was_set = fathom::tb_init(path);
        let n_men = fathom::n_men();

        assert!(was_set && n_men != 0, "Invalid tablebase path: {}", path);

        self.is_enabled = true;
        self.n_men = n_men as u8;
    }

    pub fn can_probe(&self, game: &Game) -> bool {
        if !self.is_enabled {
            return false;
        }

        if game.board.occupancy().count() > self.n_men {
            return false;
        }

        true
    }

    pub fn n_men(&self) -> u8 {
        self.n_men
    }

    pub fn wdl(&self, game: &Game) -> Option<Outcome> {
        debug_assert!(self.is_enabled);

        if game.castle_rights[White].any() || game.castle_rights[Black].any() {
            return None;
        }

        if game.halfmove_clock != 0 {
            return None;
        }

        fathom::tb_probe_wdl(game)
    }

    pub fn root_wdl_dtz(
        &self,
        game: &Game,
        allowed_moves: &RootMoves,
    ) -> Option<(RootMoves, RootTbInfo)> {
        debug_assert!(self.is_enabled);

        if game.castle_rights[White].any() || game.castle_rights[Black].any() {
            return None;
        }

        let has_repeated_position = Self::has_repeated_position(game);

        let (root_moves, dtz_found) = fathom::tb_probe_root_dtz(game, has_repeated_position)
            .map(|moves| (moves, true))
            .or_else(|| fathom::tb_probe_root_wdl(game).map(|moves| (moves, false)))?;

        let mut root_moves_vec: Vec<_> = root_moves
            .into_iter()
            .filter(|m| allowed_moves.has_move(m.mv))
            .collect();

        // If moves immediately draw, set their rank to 0.
        let mut g = game.clone();
        for m in &mut root_moves_vec {
            g.make_move(m.mv);

            if g.is_repeated_position(0) {
                m.tb_rank = 0;
                m.tb_score = Eval::DRAW;
            }

            g.undo_move();
        }

        root_moves_vec.sort_by_key(|m| Reverse(m.tb_rank));

        let best_rank = root_moves_vec.first()?.tb_rank;

        let root_moves: ArrayVec<_, { MAX_LEGAL_MOVES }> = root_moves_vec
            .into_iter()
            .take_while(|mv| mv.tb_rank == best_rank)
            .collect();

        let probe_wdl = !dtz_found && root_moves.get(0).tb_score.is_win();

        Some((RootMoves::new(root_moves), RootTbInfo { probe_wdl }))
    }

    fn has_repeated_position(game: &Game) -> bool {
        let mut seen = HashSet::new();
        seen.insert(game.hash);

        game.history
            .iter()
            .rev()
            .take(game.halfmove_clock as usize)
            .any(|position| !seen.insert(position.hash))
    }
}
