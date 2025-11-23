use crate::{
    chess::{moves::Move, zobrist::ZobristHash},
    engine::eval::Eval,
};

pub struct TranspositionTable {
    data: Vec<Option<TranspositionTableEntry>>,
    pub generation: u8,
    pub occupied: usize,
    size: usize,
}

#[derive(Clone)]
struct TranspositionTableEntry {
    pub key: ZobristHash,
    pub bound: NodeBound,
    pub score: i16,
    pub depth: u8,
    pub age: u8,
    pub best_move: Option<Move>,
}

pub struct TranspositionTableHit {
    pub bound: NodeBound,
    pub score: Eval,
    pub depth: u8,
    pub best_move: Option<Move>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NodeBound {
    Exact,
    Upper,
    Lower,
}

const _ASSERT_TT_DATA_SIZE: () = assert!(
    size_of::<TranspositionTableEntry>() == 16,
    "Transposition table entry size changed"
);

pub const fn calculate_number_of_entries(size_mb: usize) -> usize {
    let size_of_entry = size_of::<TranspositionTableEntry>();
    let total_size_in_bytes = size_mb * 1024 * 1024;
    total_size_in_bytes / size_of_entry
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let mut tt = Self {
            data: Vec::new(),
            size: 0,
            occupied: 0,
            generation: 0,
        };

        tt.resize(size_mb);
        tt
    }

    pub fn reset(&mut self) {
        for i in 0..self.data.len() {
            self.data[i] = None;
        }

        self.generation = 0;
        self.occupied = 0;
    }

    pub fn resize(&mut self, size_mb: usize) {
        if self.size == size_mb {
            return;
        }

        let number_of_entries = calculate_number_of_entries(size_mb);

        self.data.clear();
        self.data.resize(number_of_entries, None);
        self.data.shrink_to_fit();
        self.size = size_mb;
        self.occupied = 0;
        self.generation = 0;
    }

    pub fn new_generation(&mut self) {
        self.generation += 1;
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "The truncation is intended to get an index"
    )]
    fn get_entry_idx(&self, key: ZobristHash) -> usize {
        // PERF: There's likely a more performant way to do this
        key.0 as usize % self.data.len()
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "This is just an approximation, so a loss of precision is fine"
    )]
    pub fn occupancy(&self) -> usize {
        let decimal = self.occupied as f32 / self.data.len() as f32;
        let permille = decimal * 1000.0;
        permille as usize
    }

    fn should_overwrite(old: &TranspositionTableEntry, new: &TranspositionTableEntry) -> bool {
        // Always prioritise results from new searches
        if new.age != old.age {
            return true;
        }

        // Always prefer results that have been searched to a higher depth,
        // since they're more accurate
        if new.depth > old.depth {
            return true;
        }

        // If the new node is exact, always store it
        if new.bound == NodeBound::Exact {
            return true;
        }

        // Don't overwrite exact nodes
        old.bound != NodeBound::Exact
    }

    // When searching, mate scores are relative to the root position.
    // However, we may see the same position at different depths of the
    // tree due to transpositions.
    // As a result, when caching mate evaluations, we need to store them
    // as relative to the position at that point in the tree, rather than
    // relative to the root (by accounting for the difference between the
    // root and the current depth).
    pub fn with_mate_distance_from_position(eval: Eval, plies: u8) -> Eval {
        if eval.mating() {
            return Eval(eval.0 + i32::from(plies));
        }

        if eval.being_mated() {
            return Eval(eval.0 - i32::from(plies));
        }

        eval
    }

    pub fn with_mate_distance_from_root(eval: Eval, plies: u8) -> Eval {
        if eval.mating() {
            return Eval(eval.0 - i32::from(plies));
        }

        if eval.being_mated() {
            return Eval(eval.0 + i32::from(plies));
        }

        eval
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "Eval truncated to i16 but is guaranteed to be within those bounds"
    )]
    pub fn insert(
        &mut self,
        key: ZobristHash,
        bound: NodeBound,
        score: Eval,
        depth: u8,
        age: u8,
        best_move: Option<Move>,
        plies: u8,
    ) {
        let idx = self.get_entry_idx(key);

        let new_entry = TranspositionTableEntry {
            key,
            bound,
            score: Self::with_mate_distance_from_position(score, plies).0 as i16,
            depth,
            age,
            best_move,
        };

        // !: We know the exact size of the table and will always access within the bounds.
        unsafe {
            if let Some(existing_entry) = self.data.get_unchecked(idx) {
                if Self::should_overwrite(existing_entry, &new_entry) {
                    self.data[idx] = Some(new_entry);
                }
            } else {
                self.occupied += 1;
                self.data[idx] = Some(new_entry);
            }
        }
    }

    pub fn get(&self, key: ZobristHash, plies: u8) -> Option<TranspositionTableHit> {
        let idx = self.get_entry_idx(key);

        // !: We know the exact size of the table and will always access within the bounds.
        unsafe {
            if let Some(entry) = self.data.get_unchecked(idx)
                && entry.key == key
            {
                return Some(TranspositionTableHit {
                    bound: entry.bound,
                    score: Self::with_mate_distance_from_root(Eval(i32::from(entry.score)), plies),
                    depth: entry.depth,
                    best_move: entry.best_move,
                });
            }
        }

        None
    }
}
