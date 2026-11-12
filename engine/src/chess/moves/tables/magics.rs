use crate::chess::{
    moves::tables::magic_consts::{
        ATTACKS_TABLE_SIZE, BISHOP_MAGICS, BISHOP_SHIFT, ROOK_MAGICS, ROOK_SHIFT,
    },
    prelude::*,
};

macro_rules! table {
    ($file:literal, $len:expr) => {
        unsafe {
            std::mem::transmute::<[u8; size_of::<Bitboard>() * $len], [Bitboard; $len]>(
                *include_bytes!(concat!(env!("OUT_DIR"), "/", $file)),
            )
        }
    };
}

static ROOK_NOT_MASKS: [Bitboard; Square::N] = table!("rook_not_masks.bin", Square::N);
static BISHOP_NOT_MASKS: [Bitboard; Square::N] = table!("bishop_not_masks.bin", Square::N);
static ATTACKS_TABLE: [Bitboard; ATTACKS_TABLE_SIZE] = table!("magics.bin", ATTACKS_TABLE_SIZE);

pub const fn lookup_rook_attacks(s: Square, blockers: Bitboard) -> Bitboard {
    ATTACKS_TABLE[table_index(s, blockers, &ROOK_MAGICS, &ROOK_NOT_MASKS, ROOK_SHIFT)]
}

pub const fn lookup_bishop_attacks(s: Square, blockers: Bitboard) -> Bitboard {
    ATTACKS_TABLE[table_index(s, blockers, &BISHOP_MAGICS, &BISHOP_NOT_MASKS, BISHOP_SHIFT)]
}

const fn table_index(
    square: Square,
    blockers: Bitboard,
    magics: &[(u64, usize); Square::N],
    not_masks: &[Bitboard; Square::N],
    shift: usize,
) -> usize {
    let (magic, index) = magics[square.idx()];
    let relevant_occupancies = blockers.or(not_masks[square.idx()]).raw();
    let offset = relevant_occupancies.wrapping_mul(magic) >> (Square::N - shift);
    index + offset as usize
}
