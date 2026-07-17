use crate::chess::{Bitboard, Player, bitboards};

#[derive(PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum File {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
}

static FILE_NOTATION: [&str; File::N] = ["a", "b", "c", "d", "e", "f", "g", "h"];
static FILE_BITBOARDS: [Bitboard; File::N] = [
    bitboards::A_FILE,
    bitboards::B_FILE,
    bitboards::C_FILE,
    bitboards::D_FILE,
    bitboards::E_FILE,
    bitboards::F_FILE,
    bitboards::G_FILE,
    bitboards::H_FILE,
];

impl File {
    pub const ALL: [Self; 8] = {
        use File::*;
        [A, B, C, D, E, F, G, H]
    };

    pub const N: usize = Self::ALL.len();

    pub const fn from_idx(idx: u8) -> Self {
        debug_assert!(idx < 8);
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }

    #[inline(always)]
    pub const fn idx(self) -> u8 {
        self as u8
    }

    pub const fn notation(self) -> &'static str {
        FILE_NOTATION[self as usize]
    }

    pub const fn bitboard(self) -> Bitboard {
        FILE_BITBOARDS[self as usize]
    }
}

impl std::fmt::Debug for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl<T> std::ops::Index<File> for [T; File::N] {
    type Output = T;

    fn index(&self, index: File) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> std::ops::IndexMut<File> for [T; File::N] {
    fn index_mut(&mut self, index: File) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum Rank {
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
}

static RANK_NOTATION: [&str; File::N] = ["1", "2", "3", "4", "5", "6", "7", "8"];

impl Rank {
    pub const ALL: [Self; 8] = {
        use Rank::*;
        [R1, R2, R3, R4, R5, R6, R7, R8]
    };

    pub const N: usize = Self::ALL.len();

    #[inline(always)]
    pub fn from_idx(idx: u8) -> Self {
        debug_assert!(idx < 8);
        unsafe { std::mem::transmute::<u8, Self>(idx) }
    }

    #[inline(always)]
    pub const fn idx(self) -> u8 {
        self as u8
    }

    pub const fn notation(self) -> &'static str {
        RANK_NOTATION[self as usize]
    }
}

impl std::fmt::Debug for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl std::fmt::Display for Rank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl<T> std::ops::Index<Rank> for [T; Rank::N] {
    type Output = T;

    fn index(&self, index: Rank) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> std::ops::IndexMut<Rank> for [T; Rank::N] {
    fn index_mut(&mut self, index: Rank) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash)]
pub struct Square(u8);

impl Square {
    pub const N: usize = 64;

    pub const fn from_file_and_rank(file: File, rank: Rank) -> Self {
        Self::from_idxs(file.idx(), rank.idx())
    }

    pub const fn from_index(idx: u8) -> Self {
        debug_assert!(idx < 64);
        Self(idx)
    }

    #[expect(clippy::cast_possible_truncation, reason = "idx is guaranteed to be 0-63")]
    pub const fn from_array_index(idx: usize) -> Self {
        debug_assert!(idx < 64);
        Self(idx as u8)
    }

    pub const fn from_idxs(file_idx: u8, rank_idx: u8) -> Self {
        let idx = rank_idx * 8 + file_idx;
        Self::from_index(idx)
    }

    #[inline(always)]
    pub const fn bb(self) -> Bitboard {
        Bitboard::new(1 << self.0)
    }

    #[inline(always)]
    pub const fn idx(self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub fn rank(self) -> Rank {
        Rank::from_idx(self.idx() / 8)
    }

    #[inline(always)]
    pub fn file(self) -> File {
        File::from_idx(self.idx() % 8)
    }

    pub fn notation(self) -> String {
        format!("{}{}", self.file(), self.rank())
    }

    #[inline(always)]
    pub fn forward(self, player: Player) -> Self {
        match player {
            Player::White => self.north(),
            Player::Black => self.south(),
        }
    }

    #[inline(always)]
    pub fn backward(self, player: Player) -> Self {
        match player {
            Player::White => self.south(),
            Player::Black => self.north(),
        }
    }

    #[inline(always)]
    pub fn north(self) -> Self {
        Self(self.0 + 8)
    }

    #[inline(always)]
    pub fn south(self) -> Self {
        Self(self.0 - 8)
    }

    #[inline(always)]
    pub fn east(self) -> Self {
        Self(self.0 + 1)
    }

    #[inline(always)]
    pub fn west(self) -> Self {
        Self(self.0 - 1)
    }

    #[inline(always)]
    pub const fn mirror_vertically(self) -> Self {
        Self(self.idx() ^ 0b11_1000)
    }

    #[inline(always)]
    pub const fn relative_for(self, player: Player) -> Self {
        match player {
            Player::White => self,
            Player::Black => self.mirror_vertically(),
        }
    }

    #[inline(always)]
    pub fn mirrored_horizontally_if(self, should_mirror: bool) -> Self {
        Self(self.0 ^ (7 * u8::from(should_mirror)))
    }
}

impl std::fmt::Debug for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl std::fmt::Display for Square {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.notation())
    }
}

impl<T> std::ops::Index<Square> for [T; Square::N] {
    type Output = T;

    fn index(&self, index: Square) -> &Self::Output {
        unsafe { self.get_unchecked(index.0 as usize) }
    }
}

impl<T> std::ops::IndexMut<Square> for [T; Square::N] {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index.0 as usize) }
    }
}

#[cfg(test)]
impl std::ops::BitOr for Square {
    type Output = Bitboard;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.bb() | rhs.bb()
    }
}

pub mod ranks {
    use super::*;

    static BACK_RANKS: [Rank; Player::N] = [Rank::R1, Rank::R8];
    pub fn back_rank(player: Player) -> Rank {
        BACK_RANKS[player]
    }

    static PAWN_BACK_RANKS: [Rank; Player::N] = [Rank::R2, Rank::R7];
    pub fn pawn_back_rank(player: Player) -> Rank {
        PAWN_BACK_RANKS[player]
    }

    static PROMOTION_RANKS: [Rank; Player::N] = [Rank::R7, Rank::R2];
    pub fn promotion_rank(player: Player) -> Rank {
        PROMOTION_RANKS[player]
    }
}

pub mod squares {
    use self::all::*;
    use crate::chess::{player::Player, square::Square};

    pub const WHITE_STANDARD_KINGSIDE_ROOK_START: Square = H1;
    pub const WHITE_STANDARD_QUEENSIDE_ROOK_START: Square = A1;
    pub const BLACK_STANDARD_KINGSIDE_ROOK_START: Square = H8;
    pub const BLACK_STANDARD_QUEENSIDE_ROOK_START: Square = A8;

    static KINGSIDE_CASTLE_END_KING: [Square; Player::N] = [G1, G8];
    pub const fn kingside_king_castle_end(player: Player) -> Square {
        KINGSIDE_CASTLE_END_KING[player.idx()]
    }

    static QUEENSIDE_CASTLE_END_KING: [Square; Player::N] = [C1, C8];
    pub const fn queenside_king_castle_end(player: Player) -> Square {
        QUEENSIDE_CASTLE_END_KING[player.idx()]
    }

    static KINGSIDE_CASTLE_END_ROOK: [Square; Player::N] = [F1, F8];
    pub const fn kingside_rook_castle_end(player: Player) -> Square {
        KINGSIDE_CASTLE_END_ROOK[player.idx()]
    }

    static QUEENSIDE_CASTLE_END_ROOK: [Square; Player::N] = [D1, D8];
    pub const fn queenside_rook_castle_end(player: Player) -> Square {
        QUEENSIDE_CASTLE_END_ROOK[player.idx()]
    }

    pub mod all {
        use super::super::*;

        // For convenience
        pub const A1: Square = Square::from_file_and_rank(File::A, Rank::R1);
        pub const A2: Square = Square::from_file_and_rank(File::A, Rank::R2);
        pub const A3: Square = Square::from_file_and_rank(File::A, Rank::R3);
        pub const A4: Square = Square::from_file_and_rank(File::A, Rank::R4);
        pub const A5: Square = Square::from_file_and_rank(File::A, Rank::R5);
        pub const A6: Square = Square::from_file_and_rank(File::A, Rank::R6);
        pub const A7: Square = Square::from_file_and_rank(File::A, Rank::R7);
        pub const A8: Square = Square::from_file_and_rank(File::A, Rank::R8);

        pub const B1: Square = Square::from_file_and_rank(File::B, Rank::R1);
        pub const B2: Square = Square::from_file_and_rank(File::B, Rank::R2);
        pub const B3: Square = Square::from_file_and_rank(File::B, Rank::R3);
        pub const B4: Square = Square::from_file_and_rank(File::B, Rank::R4);
        pub const B5: Square = Square::from_file_and_rank(File::B, Rank::R5);
        pub const B6: Square = Square::from_file_and_rank(File::B, Rank::R6);
        pub const B7: Square = Square::from_file_and_rank(File::B, Rank::R7);
        pub const B8: Square = Square::from_file_and_rank(File::B, Rank::R8);

        pub const C1: Square = Square::from_file_and_rank(File::C, Rank::R1);
        pub const C2: Square = Square::from_file_and_rank(File::C, Rank::R2);
        pub const C3: Square = Square::from_file_and_rank(File::C, Rank::R3);
        pub const C4: Square = Square::from_file_and_rank(File::C, Rank::R4);
        pub const C5: Square = Square::from_file_and_rank(File::C, Rank::R5);
        pub const C6: Square = Square::from_file_and_rank(File::C, Rank::R6);
        pub const C7: Square = Square::from_file_and_rank(File::C, Rank::R7);
        pub const C8: Square = Square::from_file_and_rank(File::C, Rank::R8);

        pub const D1: Square = Square::from_file_and_rank(File::D, Rank::R1);
        pub const D2: Square = Square::from_file_and_rank(File::D, Rank::R2);
        pub const D3: Square = Square::from_file_and_rank(File::D, Rank::R3);
        pub const D4: Square = Square::from_file_and_rank(File::D, Rank::R4);
        pub const D5: Square = Square::from_file_and_rank(File::D, Rank::R5);
        pub const D6: Square = Square::from_file_and_rank(File::D, Rank::R6);
        pub const D7: Square = Square::from_file_and_rank(File::D, Rank::R7);
        pub const D8: Square = Square::from_file_and_rank(File::D, Rank::R8);

        pub const E1: Square = Square::from_file_and_rank(File::E, Rank::R1);
        pub const E2: Square = Square::from_file_and_rank(File::E, Rank::R2);
        pub const E3: Square = Square::from_file_and_rank(File::E, Rank::R3);
        pub const E4: Square = Square::from_file_and_rank(File::E, Rank::R4);
        pub const E5: Square = Square::from_file_and_rank(File::E, Rank::R5);
        pub const E6: Square = Square::from_file_and_rank(File::E, Rank::R6);
        pub const E7: Square = Square::from_file_and_rank(File::E, Rank::R7);
        pub const E8: Square = Square::from_file_and_rank(File::E, Rank::R8);

        pub const F1: Square = Square::from_file_and_rank(File::F, Rank::R1);
        pub const F2: Square = Square::from_file_and_rank(File::F, Rank::R2);
        pub const F3: Square = Square::from_file_and_rank(File::F, Rank::R3);
        pub const F4: Square = Square::from_file_and_rank(File::F, Rank::R4);
        pub const F5: Square = Square::from_file_and_rank(File::F, Rank::R5);
        pub const F6: Square = Square::from_file_and_rank(File::F, Rank::R6);
        pub const F7: Square = Square::from_file_and_rank(File::F, Rank::R7);
        pub const F8: Square = Square::from_file_and_rank(File::F, Rank::R8);

        pub const G1: Square = Square::from_file_and_rank(File::G, Rank::R1);
        pub const G2: Square = Square::from_file_and_rank(File::G, Rank::R2);
        pub const G3: Square = Square::from_file_and_rank(File::G, Rank::R3);
        pub const G4: Square = Square::from_file_and_rank(File::G, Rank::R4);
        pub const G5: Square = Square::from_file_and_rank(File::G, Rank::R5);
        pub const G6: Square = Square::from_file_and_rank(File::G, Rank::R6);
        pub const G7: Square = Square::from_file_and_rank(File::G, Rank::R7);
        pub const G8: Square = Square::from_file_and_rank(File::G, Rank::R8);

        pub const H1: Square = Square::from_file_and_rank(File::H, Rank::R1);
        pub const H2: Square = Square::from_file_and_rank(File::H, Rank::R2);
        pub const H3: Square = Square::from_file_and_rank(File::H, Rank::R3);
        pub const H4: Square = Square::from_file_and_rank(File::H, Rank::R4);
        pub const H5: Square = Square::from_file_and_rank(File::H, Rank::R5);
        pub const H6: Square = Square::from_file_and_rank(File::H, Rank::R6);
        pub const H7: Square = Square::from_file_and_rank(File::H, Rank::R7);
        pub const H8: Square = Square::from_file_and_rank(File::H, Rank::R8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::square::squares::all::*;

    #[test]
    fn square_from_index() {
        assert_eq!(Square::from_index(0), A1);
        assert_eq!(Square::from_index(63), H8);
    }

    #[test]
    fn square_from_idxs() {
        assert_eq!(Square::from_idxs(0, 0), A1);
        assert_eq!(Square::from_idxs(7, 7), H8);
    }

    #[test]
    fn square_from_file_and_rank() {
        assert_eq!(Square::from_file_and_rank(File::A, Rank::R1), A1);
        assert_eq!(Square::from_file_and_rank(File::H, Rank::R8), H8);
    }

    #[test]
    fn square_size() {
        assert_eq!(std::mem::size_of::<Square>(), std::mem::size_of::<u8>());
    }
}
