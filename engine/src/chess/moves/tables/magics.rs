use crate::chess::{
    moves::tables::magic_consts::{
        ATTACKS_TABLE_SIZE, BISHOP_MAGICS, BISHOP_SHIFT, ROOK_MAGICS, ROOK_SHIFT,
    },
    prelude::*,
};

static ROOK_NOT_MASKS: [Bitboard; Square::N] = unsafe {
    std::mem::transmute::<[u8; size_of::<Bitboard>() * Square::N], [Bitboard; Square::N]>(
        *include_bytes!(concat!(env!("OUT_DIR"), "/rook_not_masks.bin")),
    )
};

static BISHOP_NOT_MASKS: [Bitboard; Square::N] = unsafe {
    std::mem::transmute::<[u8; size_of::<Bitboard>() * Square::N], [Bitboard; Square::N]>(
        *include_bytes!(concat!(env!("OUT_DIR"), "/bishop_not_masks.bin")),
    )
};

static ATTACKS_TABLE: [Bitboard; ATTACKS_TABLE_SIZE] = unsafe {
    std::mem::transmute::<
        [u8; size_of::<Bitboard>() * ATTACKS_TABLE_SIZE],
        [Bitboard; ATTACKS_TABLE_SIZE],
    >(*include_bytes!(concat!(env!("OUT_DIR"), "/magics.bin")))
};

pub const fn lookup_rook_attacks(s: Square, blockers: Bitboard) -> Bitboard {
    let index = table_index(s, blockers, &ROOK_MAGICS, &ROOK_NOT_MASKS, ROOK_SHIFT);
    ATTACKS_TABLE[index]
}

pub const fn lookup_bishop_attacks(s: Square, blockers: Bitboard) -> Bitboard {
    let index = table_index(s, blockers, &BISHOP_MAGICS, &BISHOP_NOT_MASKS, BISHOP_SHIFT);
    ATTACKS_TABLE[index]
}

const fn table_index(
    square: Square,
    blockers: Bitboard,
    magics: &[(u64, usize); Square::N],
    not_masks: &[Bitboard; Square::N],
    shift: usize,
) -> usize {
    let (magic, index) = magics[square.array_idx()];
    let relevant_occupancies = blockers.as_u64() | not_masks[square.array_idx()].as_u64();
    let mut offset = relevant_occupancies.wrapping_mul(magic);
    offset >>= Square::N - shift;
    index + offset as usize
}
