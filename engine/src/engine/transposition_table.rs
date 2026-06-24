use std::{
    mem::transmute,
    sync::atomic::{AtomicU8, AtomicU64, Ordering},
};

use crate::{
    chess::{moves::Move, zobrist::ZobristHash},
    engine::{eval::Eval, search::types::Depth},
};

pub struct TranspositionTable {
    data: Vec<RawCluster>,
    generation: AtomicU8,
    size: usize,
}

#[derive(Clone)]
#[repr(transparent)]
struct Flags(u8);

const MAX_AGE: u8 = 1 << 5;
const AGE_MASK: u8 = MAX_AGE - 1;

impl Flags {
    fn new(bound: NodeBound, age: u8, was_pv: bool) -> Self {
        Self(age << 3 | u8::from(was_pv) << 2 | bound as u8)
    }

    fn bound(&self) -> NodeBound {
        match self.0 & 0b11 {
            0b00 => NodeBound::None,
            0b01 => NodeBound::Exact,
            0b10 => NodeBound::Upper,
            0b11 => NodeBound::Lower,
            _ => unreachable!(),
        }
    }

    fn was_pv(&self) -> bool {
        self.0 & 0b100 == 0b100
    }

    fn age(&self) -> u8 {
        self.0 >> 3
    }
}

#[inline]
#[expect(clippy::cast_possible_truncation, reason = "Truncation is intended")]
fn tt_key(hash: ZobristHash) -> u16 {
    hash.0 as u16
}

#[derive(Clone)]
struct TranspositionTableEntry {
    pub key: u16,                // 2 bytes
    pub best_move: Option<Move>, // 2 bytes
    pub score: i16,              // 2 bytes
    pub eval: i16,               // 2 bytes
    pub depth: u8,               // 1 byte
    pub flags: Flags,            // 1 byte
}

const _ASSERT_TT_DATA_SIZE: () =
    assert!(size_of::<TranspositionTableEntry>() == 10, "Transposition table entry size changed");

impl TranspositionTableEntry {
    fn bound(&self) -> NodeBound {
        self.flags.bound()
    }

    fn age(&self) -> u8 {
        self.flags.age()
    }

    fn was_pv(&self) -> bool {
        self.flags.was_pv()
    }

    fn relative_age(&self, age: u8) -> i32 {
        i32::from((MAX_AGE + age - self.age()) & AGE_MASK)
    }

    fn is_empty(&self) -> bool {
        self.bound() == NodeBound::None
    }
}

pub struct TranspositionTableHit {
    pub bound: NodeBound,
    pub score: Eval,
    pub eval: Eval,
    pub depth: Depth,
    pub best_move: Option<Move>,
    pub was_pv: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum NodeBound {
    None,
    Exact,
    Upper,
    Lower,
}

const ENTRIES_PER_CLUSTER: usize = 3;

#[repr(align(32))]
pub struct Cluster {
    entries: [TranspositionTableEntry; ENTRIES_PER_CLUSTER],
    _padding: u16,
}

#[repr(align(32))]
pub struct RawCluster {
    bits: [AtomicU64; 4],
}

impl RawCluster {
    fn write(&self, cluster: Cluster) {
        let bits = unsafe { transmute::<Cluster, [u64; 4]>(cluster) };

        self.bits[0].store(bits[0], Ordering::Relaxed);
        self.bits[1].store(bits[1], Ordering::Relaxed);
        self.bits[2].store(bits[2], Ordering::Relaxed);
        self.bits[3].store(bits[3], Ordering::Relaxed);
    }

    fn read(&self) -> Cluster {
        let b0 = self.bits[0].load(Ordering::Relaxed);
        let b1 = self.bits[1].load(Ordering::Relaxed);
        let b2 = self.bits[2].load(Ordering::Relaxed);
        let b3 = self.bits[3].load(Ordering::Relaxed);

        unsafe { transmute::<[u64; 4], Cluster>([b0, b1, b2, b3]) }
    }
}

pub const fn calculate_number_of_clusters(size_mb: usize) -> usize {
    let size_of_entry = size_of::<RawCluster>();
    let total_size_in_bytes = size_mb * 1024 * 1024;
    total_size_in_bytes / size_of_entry
}

impl TranspositionTable {
    pub fn new(size_mb: usize) -> Self {
        let mut tt = Self {
            data: Vec::new(),
            size: 0,
            generation: AtomicU8::default(),
        };

        tt.resize(size_mb, 1);
        tt
    }

    pub fn reset(&mut self, threads: usize) {
        unsafe {
            std::thread::scope(|scope| {
                let len = self.data.len();
                let slice = std::slice::from_raw_parts_mut(self.data.as_mut_ptr(), len);

                let chunk_size = len.div_ceil(threads);
                for chunk in slice.chunks_mut(chunk_size) {
                    scope.spawn(|| chunk.as_mut_ptr().write_bytes(0, chunk.len()));
                }
            });
        }

        self.generation.store(0, Ordering::Relaxed);
    }

    // Safety: Does not zero the memory it returns so all callers must zero it
    unsafe fn allocate(len: usize) -> Vec<RawCluster> {
        use std::{
            alloc::{Layout, alloc, handle_alloc_error},
            ptr::slice_from_raw_parts_mut,
        };

        unsafe {
            let layout = Layout::array::<RawCluster>(len).unwrap();

            let ptr = alloc(layout);
            if ptr.is_null() {
                handle_alloc_error(layout);
            }

            Box::from_raw(slice_from_raw_parts_mut(ptr.cast(), len)).into()
        }
    }

    pub fn resize(&mut self, size_mb: usize, threads: usize) {
        if self.size == size_mb {
            return;
        }

        // Force deallocation of the existing memory
        self.data.clear();
        self.data.shrink_to_fit();

        let number_of_clusters = calculate_number_of_clusters(size_mb);

        unsafe {
            self.data = Self::allocate(number_of_clusters);
            self.reset(threads);
        }

        self.size = size_mb;
    }

    pub fn new_generation(&self) {
        let mut generation = self.generation.load(Ordering::Relaxed);

        generation += 1;
        generation &= AGE_MASK;

        self.generation.store(generation, Ordering::Relaxed);
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "The truncation is intended to get an index"
    )]
    fn get_cluster_idx(&self, key: ZobristHash) -> usize {
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
        let generation = self.generation.load(Ordering::Relaxed);

        for raw_cluster in self.data.iter().take(estimate_n) {
            let cluster = raw_cluster.read();

            for entry in cluster.entries {
                if entry.age() == generation {
                    occupied += 1;
                }
            }
        }

        let occupied_entries = occupied as f32 / estimate_n as f32;
        let occupied_clusters = occupied_entries / ENTRIES_PER_CLUSTER as f32;
        let permille = occupied_clusters * 1000.0;
        permille as u64
    }

    fn should_overwrite(old: &TranspositionTableEntry, new: &TranspositionTableEntry) -> bool {
        // Always prioritise results from new positions
        if old.key != new.key {
            return true;
        }

        // Always prioritise results from new searches
        if new.age() != old.age() {
            return true;
        }

        // If the new node is exact, always store it
        if new.bound() == NodeBound::Exact {
            return true;
        }

        // Weight entries by depth searched, preferring newer entries unless old entries are searched
        // to significantly higher depths
        if new.depth + 4 > old.depth {
            return true;
        }

        false
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

        if eval.is_win() {
            return Eval(eval.0 + i32::from(plies));
        }

        if eval.is_loss() {
            return Eval(eval.0 - i32::from(plies));
        }

        eval
    }

    pub fn with_mate_distance_from_root(eval: Eval, plies: u8) -> Eval {
        if eval == Eval::NONE {
            return eval;
        }

        if eval.is_win() {
            return Eval(eval.0 - i32::from(plies));
        }

        if eval.is_loss() {
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
        was_pv: bool,
    ) {
        let idx = self.get_cluster_idx(key);
        let key = tt_key(key);

        let mut cluster = self.data[idx].read();
        let age = self.generation.load(Ordering::Relaxed);

        let mut least_valuable_entry_idx = 0;
        let mut least_valuable_entry = None;
        let mut worst_value = i32::MAX;

        // Try and pick the least valuable existing entry
        for (idx, entry) in cluster.entries.iter().enumerate() {
            if entry.is_empty() || entry.key == key {
                least_valuable_entry = Some(entry);
                least_valuable_entry_idx = idx;
                break;
            }

            let entry_value = i32::from(entry.depth) - 4 * entry.relative_age(age);

            if entry_value < worst_value {
                least_valuable_entry = Some(entry);
                least_valuable_entry_idx = idx;
                worst_value = entry_value;
            }
        }

        let mut new_entry = TranspositionTableEntry {
            key,
            score: Self::with_mate_distance_from_position(score, plies).0 as i16,
            eval: eval.0 as i16,
            depth: depth.as_u8(),
            best_move,
            flags: Flags::new(bound, age, was_pv),
        };

        if best_move.is_none()
            && let Some(lve) = least_valuable_entry
            && lve.key == key
            && lve.best_move.is_some()
        {
            new_entry.best_move = lve.best_move;
        }

        if least_valuable_entry.is_none()
            || Self::should_overwrite(least_valuable_entry.unwrap(), &new_entry)
        {
            cluster.entries[least_valuable_entry_idx] = new_entry;
            self.data[idx].write(cluster);
        }
    }

    pub fn get(&self, key: ZobristHash, plies: u8) -> Option<TranspositionTableHit> {
        let idx = self.get_cluster_idx(key);
        let key = tt_key(key);

        // !: We know the exact size of the table and will always access within the bounds.
        unsafe {
            let cluster = self.data.get_unchecked(idx).read();

            for entry in cluster.entries {
                if entry.key == key {
                    return Some(TranspositionTableHit {
                        bound: entry.bound(),
                        score: Self::with_mate_distance_from_root(
                            Eval(i32::from(entry.score)),
                            plies,
                        ),
                        depth: Depth::new(entry.depth),
                        eval: Eval(i32::from(entry.eval)),
                        best_move: entry.best_move,
                        was_pv: entry.was_pv(),
                    });
                }
            }
        }

        None
    }

    pub fn prefetch(&self, hash: ZobristHash) {
        cfg_select! {
            target_arch = "x86_64" => {
                use std::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};

                let idx = self.get_cluster_idx(hash);

                unsafe {
                    let ptr = self.data.as_ptr().add(idx).cast();
                    _mm_prefetch::<_MM_HINT_T0>(ptr);
                }
            }
            _ => {
                // Prevent warnings on platforms that can't prefetch TT entries
                _ = hash;
            }
        }
    }
}
