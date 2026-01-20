use std::{
    mem::transmute,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    chess::{moves::Move, zobrist::ZobristHash},
    engine::{eval::Eval, search::types::Depth},
};

pub struct TranspositionTable {
    data: Vec<AtomicTranspositionTableEntry>,
    generation: u8,
    size: usize,
}

#[derive(Clone)]
#[repr(transparent)]
struct BoundAndAge(u8);

impl BoundAndAge {
    const AGE_MASK: u8 = 0b0011_1111;

    fn new(bound: NodeBound, age: u8) -> Self {
        Self(age << 2 | bound as u8)
    }

    fn bound(&self) -> NodeBound {
        match self.0 & 0b11 {
            0b00 => NodeBound::Exact,
            0b01 => NodeBound::Upper,
            0b10 => NodeBound::Lower,
            0b11 => NodeBound::None,
            _ => unreachable!(),
        }
    }

    fn age(&self) -> u8 {
        self.0 >> 2
    }
}

#[derive(Clone)]
struct TranspositionTableEntry {
    pub key: ZobristHash,           // 8 bytes
    pub best_move: Option<Move>,    // 2 bytes
    pub score: i16,                 // 2 bytes
    pub eval: i16,                  // 2 bytes
    pub depth: u8,                  // 1 byte
    pub bound_and_age: BoundAndAge, // 1 byte
}

const _ASSERT_TT_DATA_SIZE: () =
    assert!(size_of::<TranspositionTableEntry>() == 16, "Transposition table entry size changed");

impl TranspositionTableEntry {
    fn bound(&self) -> NodeBound {
        self.bound_and_age.bound()
    }

    fn age(&self) -> u8 {
        self.bound_and_age.age()
    }
}

pub struct TranspositionTableHit {
    pub bound: NodeBound,
    pub score: Eval,
    pub eval: Eval,
    pub depth: Depth,
    pub best_move: Option<Move>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NodeBound {
    Exact,
    Upper,
    Lower,
    None,
}

struct TranspositionTableEntryBits {
    key: u64,
    data: u64,
}

struct AtomicTranspositionTableEntry {
    key: AtomicU64,
    data: AtomicU64,
}

impl AtomicTranspositionTableEntry {
    const fn empty() -> Self {
        Self {
            key: AtomicU64::new(0),
            data: AtomicU64::new(0),
        }
    }

    #[expect(clippy::transmute_undefined_repr, reason = "Confirmed that this transmute works")]
    fn write(&self, entry: TranspositionTableEntry) {
        let bits =
            unsafe { transmute::<TranspositionTableEntry, TranspositionTableEntryBits>(entry) };

        // XOR the key with the data. This means we can only retrieve the same key
        // here if our .read() retrieves the matching data.
        // If it retrieves non-matching data due to a race, the key will XOR back
        // to a different value and won't match the key for the position, so it won't be used.
        self.key.store(bits.key ^ bits.data, Ordering::Relaxed);
        self.data.store(bits.data, Ordering::Relaxed);
    }

    #[expect(clippy::transmute_undefined_repr, reason = "Confirmed that this transmute works")]
    fn read(&self) -> Option<TranspositionTableEntry> {
        let key = self.key.load(Ordering::Relaxed);
        let data = self.data.load(Ordering::Relaxed);

        let key = key ^ data;

        if key == 0 && data == 0 {
            return None;
        }

        let bits = TranspositionTableEntryBits { key, data };
        Some(unsafe { transmute::<TranspositionTableEntryBits, TranspositionTableEntry>(bits) })
    }
}

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
            generation: 0,
        };

        tt.resize(size_mb);
        tt
    }

    pub fn reset(&mut self) {
        for i in 0..self.data.len() {
            self.data[i] = AtomicTranspositionTableEntry::empty();
        }

        self.generation = 0;
    }

    pub fn resize(&mut self, size_mb: usize) {
        if self.size == size_mb {
            return;
        }

        let number_of_entries = calculate_number_of_entries(size_mb);

        self.data.clear();
        self.data
            .resize_with(number_of_entries, AtomicTranspositionTableEntry::empty);
        self.data.shrink_to_fit();
        self.size = size_mb;
        self.generation = 0;
    }

    pub fn new_generation(&mut self) {
        self.generation += 1;
        self.generation &= BoundAndAge::AGE_MASK;
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "The truncation is intended to get an index"
    )]
    fn get_entry_idx(&self, key: ZobristHash) -> usize {
        // (from Reckless: For details, see: https://lemire.me/blog/2016/06/27/a-fast-alternative-to-the-modulo-reduction)
        ((u128::from(key.0) * (self.data.len() as u128)) >> 64) as usize
    }

    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "This is just an approximation, so a loss of precision is fine"
    )]
    pub fn occupancy(&self) -> u64 {
        let mut occupied = 0;
        let estimate_n = 1000;

        for entry in self.data.iter().take(estimate_n) {
            if entry
                .read()
                .is_some_and(|e| e.bound_and_age.age() == self.generation)
            {
                occupied += 1;
            }
        }

        let decimal = occupied as f32 / estimate_n as f32;
        let permille = decimal * 1000.0;
        permille as u64
    }

    fn should_overwrite(old: &TranspositionTableEntry, new: &TranspositionTableEntry) -> bool {
        // Always prioritise results from new searches
        if new.age() != old.age() {
            return true;
        }

        // Always overwrite entries with no bound
        if old.bound() == NodeBound::None {
            return true;
        }

        // Never overwrite entries if the new entry has a None bound
        if new.bound() == NodeBound::None {
            return false;
        }

        // Always prefer results that have been searched to a higher depth,
        // since they're more accurate
        if new.depth > old.depth {
            return true;
        }

        // If the new node is exact, always store it
        if new.bound() == NodeBound::Exact {
            return true;
        }

        // Don't overwrite exact nodes
        old.bound() != NodeBound::Exact
    }

    // When searching, mate scores are relative to the root position.
    // However, we may see the same position at different depths of the
    // tree due to transpositions.
    // As a result, when caching mate evaluations, we need to store them
    // as relative to the position at that point in the tree, rather than
    // relative to the root (by accounting for the difference between the
    // root and the current depth).
    pub fn with_mate_distance_from_position(eval: Eval, plies: u8) -> Eval {
        if eval == Eval::NONE {
            return eval;
        }

        if eval.mating() {
            return Eval(eval.0 + i32::from(plies));
        }

        if eval.being_mated() {
            return Eval(eval.0 - i32::from(plies));
        }

        eval
    }

    pub fn with_mate_distance_from_root(eval: Eval, plies: u8) -> Eval {
        if eval == Eval::NONE {
            return eval;
        }

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
        &self,
        key: ZobristHash,
        bound: NodeBound,
        best_move: Option<Move>,
        score: Eval,
        eval: Eval,
        depth: Depth,
        plies: u8,
    ) {
        let idx = self.get_entry_idx(key);

        let new_entry = TranspositionTableEntry {
            key,
            score: Self::with_mate_distance_from_position(score, plies).0 as i16,
            eval: eval.0 as i16,
            depth: depth.as_u8(),
            best_move,
            bound_and_age: BoundAndAge::new(bound, self.generation),
        };

        // !: We know the exact size of the table and will always access within the bounds.
        unsafe {
            if let Some(existing_entry) = self.data.get_unchecked(idx).read() {
                if Self::should_overwrite(&existing_entry, &new_entry) {
                    self.data[idx].write(new_entry);
                }
            } else {
                self.data[idx].write(new_entry);
            }
        }
    }

    pub fn get(&self, key: ZobristHash, plies: u8) -> Option<TranspositionTableHit> {
        let idx = self.get_entry_idx(key);

        // !: We know the exact size of the table and will always access within the bounds.
        unsafe {
            if let Some(entry) = self.data.get_unchecked(idx).read()
                && entry.key == key
            {
                return Some(TranspositionTableHit {
                    bound: entry.bound(),
                    score: Self::with_mate_distance_from_root(Eval(i32::from(entry.score)), plies),
                    depth: Depth::new(entry.depth),
                    eval: Eval(i32::from(entry.eval)),
                    best_move: entry.best_move,
                });
            }
        }

        None
    }

    pub fn prefetch(&self, hash: ZobristHash) {
        let idx = self.get_entry_idx(hash);
        let entry = &self.data[idx];

        #[cfg(target_arch = "x86_64")]
        unsafe {
            use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            _mm_prefetch((entry as *const AtomicTranspositionTableEntry).cast::<i8>(), _MM_HINT_T0);
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            // Prevent warnings on platforms that can't prefetch TT entries
            _ = entry;
        }
    }
}
