use crate::{
    chess::prelude::*,
    engine::eval::nnue::{
        NNUE,
        nnue::{input_bucket, should_mirror},
    },
};

pub fn apply_updates(nnue: &mut NNUE, game: &Game) {
    // First, we need to cycle through our stack and update any network accumulators that we
    // deferred updates for.
    for pov in Player::ALL {
        if nnue.current_entry().threats.correct[pov] {
            continue;
        }

        // We can efficiently update if there hasn't been a move since our last 'correct' accumulator
        // that causes us to need to do a full refresh (e.g. a king crossing the horizontal mirroring
        // boundary).
        match last_good_efficient_update_index(nnue, pov) {
            Some(i) => {
                roll_forward_updates(nnue, i, game, pov);
            }
            None => {
                nnue.stack[nnue.current_idx].threats.refresh(&game.board, pov);
            }
        }
    }
}

fn last_good_efficient_update_index(nnue: &NNUE, pov: Player) -> Option<usize> {
    // Starting at the current accumulator, look back until we find either a correct accumulator
    // we can work forward from, or a change in board state that means we can't do efficient updates.
    for i in (0..=nnue.current_idx).rev() {
        let entry = &nnue.stack[i].threats;

        if entry.correct[pov] {
            return Some(i);
        }

        // If the king crossed over the middle of the board, the board either becomes, or stops being
        // mirrored and will require a full refresh.
        if entry.changes.moved_piece.kind == King {
            let from = entry.changes.mv.from();
            let to = entry.changes.mv.to();

            let crossed_mirroring_boundary = should_mirror(from) != should_mirror(to);
            if crossed_mirroring_boundary {
                return None;
            }

            let changed_input_bucket =
                input_bucket(entry.changes.mv.from(), pov) != input_bucket(entry.changes.mv.to(), pov);

            if changed_input_bucket {
                return None;
            }
        }
    }

    None
}

fn roll_forward_updates(nnue: &mut NNUE, last_good_idx: usize, game: &Game, pov: Player) {
    // We know the king is on the same side of the board for all entries since last_good_idx by definition
    let king = game.board.king_square(pov);

    for i in last_good_idx..nnue.current_idx {
        if let (prev, [entry, ..]) = nnue.stack.split_at_mut(i + 1) {
            entry.threats.update(&prev[i].threats, king, pov);
        }
    }
}
