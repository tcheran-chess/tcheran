use crate::chess::{direction::Direction, prelude::*};

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Bitboard(u64);

impl Bitboard {
    const NOT_A_FILE: Self = bitboards::A_FILE.invert();
    const NOT_H_FILE: Self = bitboards::H_FILE.invert();

    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(u64::MAX);

    #[inline(always)]
    pub const fn new(bits: u64) -> Self {
        Self(bits)
    }

    #[inline(always)]
    pub const fn as_u64(self) -> u64 {
        self.0
    }

    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    #[inline(always)]
    pub const fn any(self) -> bool {
        self.0 != 0
    }

    #[inline(always)]
    pub fn contains(self, square: Square) -> bool {
        (self & square.bb()).any()
    }

    #[inline(always)]
    pub const fn single(self) -> Square {
        debug_assert!(self.count() == 1);
        Square::from_array_index(self.trailing_zeros())
    }

    #[inline(always)]
    pub const fn invert(self) -> Self {
        Self(!self.0)
    }

    #[inline(always)]
    pub const fn lsb(self) -> Self {
        Self(1_u64.wrapping_shl(self.0.trailing_zeros()))
    }

    #[inline(always)]
    pub fn pop_lsb_inplace(&mut self) -> Self {
        let lsb = self.lsb();
        self.0 &= self.0 - 1;
        lsb
    }

    #[inline(always)]
    pub fn set(&mut self, square: Square) {
        self.0 |= square.bb().0;
    }

    #[inline(always)]
    pub fn with(&mut self, square: Square) -> Self {
        Self(self.0 | square.bb().0)
    }

    #[inline(always)]
    pub fn unset(&mut self, square: Square) {
        self.0 &= square.bb().invert().0;
    }

    #[inline(always)]
    pub fn without(&self, square: Square) -> Self {
        Self(self.0 & square.bb().invert().0)
    }

    #[inline(always)]
    pub const fn count(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[inline(always)]
    pub const fn trailing_zeros(self) -> usize {
        self.0.trailing_zeros() as usize
    }

    #[inline(always)]
    pub fn in_direction(self, direction: Direction) -> Self {
        match direction {
            Direction::North => self.north(),
            Direction::NorthEast => self.north_east(),
            Direction::East => self.east(),
            Direction::SouthEast => self.south_east(),
            Direction::South => self.south(),
            Direction::SouthWest => self.south_west(),
            Direction::West => self.west(),
            Direction::NorthWest => self.north_west(),
        }
    }

    #[inline(always)]
    pub fn forward(self, player: Player) -> Self {
        match player {
            White => self.north(),
            Black => self.south(),
        }
    }

    #[inline(always)]
    pub fn backward(self, player: Player) -> Self {
        match player {
            White => self.south(),
            Black => self.north(),
        }
    }

    #[inline(always)]
    pub const fn north(self) -> Self {
        Self(self.0 << 8)
    }

    #[inline(always)]
    pub const fn south(self) -> Self {
        Self(self.0 >> 8)
    }

    #[inline(always)]
    pub fn east(self) -> Self {
        // If we go east and land on A, we wrapped around.
        Self(self.0 << 1) & Self::NOT_A_FILE
    }

    #[inline(always)]
    pub fn north_east(self) -> Self {
        // If we go east and land on A, we wrapped around.
        Self(self.0 << 9) & Self::NOT_A_FILE
    }

    #[inline(always)]
    pub fn south_east(self) -> Self {
        // If we go east and land on A, we wrapped around.
        Self(self.0 >> 7) & Self::NOT_A_FILE
    }

    #[inline(always)]
    pub fn west(self) -> Self {
        // If we go west and land on H, we wrapped around.
        Self(self.0 >> 1) & Self::NOT_H_FILE
    }

    #[inline(always)]
    pub fn south_west(self) -> Self {
        // If we go west and land on H, we wrapped around.
        Self(self.0 >> 9) & Self::NOT_H_FILE
    }

    #[inline(always)]
    pub fn north_west(self) -> Self {
        // If we go west and land on H, we wrapped around.
        Self(self.0 << 7) & Self::NOT_H_FILE
    }
}

pub struct SquareIterator(Bitboard);

impl Iterator for SquareIterator {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            Some(Square::from_array_index(self.0.pop_lsb_inplace().trailing_zeros()))
        }
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = SquareIterator;

    fn into_iter(self) -> Self::IntoIter {
        SquareIterator(self)
    }
}

impl std::ops::Sub for Bitboard {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl std::ops::BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

#[cfg(test)]
impl std::ops::BitAnd<Square> for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Square) -> Self::Output {
        self & rhs.bb()
    }
}

impl std::ops::BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 = self.0 & rhs.0;
    }
}

#[cfg(test)]
impl std::ops::BitAndAssign<Square> for Bitboard {
    fn bitand_assign(&mut self, rhs: Square) {
        self.0 = self.0 & (rhs.bb().0);
    }
}

const impl std::ops::BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

const impl std::ops::BitOr<Square> for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Square) -> Self::Output {
        self | rhs.bb()
    }
}

impl std::ops::BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 = self.0 | rhs.0;
    }
}

impl std::ops::BitOrAssign<Square> for Bitboard {
    fn bitor_assign(&mut self, rhs: Square) {
        self.0 = self.0 | rhs.bb().0;
    }
}

impl std::ops::BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0)
    }
}

impl std::ops::BitXorAssign for Bitboard {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 = self.0 ^ rhs.0;
    }
}

impl std::ops::Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.invert()
    }
}

impl std::fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\n{}\n",
            (0..Rank::N)
                .rev()
                .map(|rank| {
                    (0..File::N)
                        .map(|file| match self.0 & (1 << (rank * 8 + file)) {
                            0 => ".",
                            _ => "*",
                        })
                        .collect::<Vec<&str>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

impl std::fmt::Display for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            (0..8)
                .rev()
                .map(|rank| {
                    (0..8)
                        .map(|file| match self.0 & (1 << (rank * 8 + file)) {
                            0 => ".",
                            _ => "*",
                        })
                        .collect::<Vec<&str>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

#[rustfmt::skip]
pub mod bitboards {
    use super::*;
    use crate::chess::player::Player;

    static BACK_RANKS: [Bitboard; Player::N] = [RANK_1, RANK_8];
    pub const fn back_rank(player: Player) -> Bitboard {
        BACK_RANKS[player.idx()]
    }

    static PAWN_BACK_RANKS: [Bitboard; Player::N] = [RANK_2, RANK_7];
    pub const fn pawn_back_rank(player: Player) -> Bitboard {
        PAWN_BACK_RANKS[player.idx()]
    }

    static DOUBLE_PUSH_RANKS: [Bitboard; Player::N] = [RANK_3, RANK_6];
    pub const fn double_push_rank(player: Player) -> Bitboard {
        DOUBLE_PUSH_RANKS[player.idx()]
    }

    pub const A_FILE: Bitboard = A1 | A2 | A3 | A4 | A5 | A6 | A7 | A8;
    pub const B_FILE: Bitboard = B1 | B2 | B3 | B4 | B5 | B6 | B7 | B8;
    pub const C_FILE: Bitboard = C1 | C2 | C3 | C4 | C5 | C6 | C7 | C8;
    pub const D_FILE: Bitboard = D1 | D2 | D3 | D4 | D5 | D6 | D7 | D8;
    pub const E_FILE: Bitboard = E1 | E2 | E3 | E4 | E5 | E6 | E7 | E8;
    pub const F_FILE: Bitboard = F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8;
    pub const G_FILE: Bitboard = G1 | G2 | G3 | G4 | G5 | G6 | G7 | G8;
    pub const H_FILE: Bitboard = H1 | H2 | H3 | H4 | H5 | H6 | H7 | H8;

    pub const R1: Bitboard = A1 | B1 | C1;

    pub const RANK_1: Bitboard = A1 | B1 | C1 | D1 | E1 | F1 | G1 | H1;
    pub const RANK_2: Bitboard = A2 | B2 | C2 | D2 | E2 | F2 | G2 | H2;
    pub const RANK_3: Bitboard = A3 | B3 | C3 | D3 | E3 | F3 | G3 | H3;
    pub const RANK_4: Bitboard = A4 | B4 | C4 | D4 | E4 | F4 | G4 | H4;
    pub const RANK_5: Bitboard = A5 | B5 | C5 | D5 | E5 | F5 | G5 | H5;
    pub const RANK_6: Bitboard = A6 | B6 | C6 | D6 | E6 | F6 | G6 | H6;
    pub const RANK_7: Bitboard = A7 | B7 | C7 | D7 | E7 | F7 | G7 | H7;
    pub const RANK_8: Bitboard = A8 | B8 | C8 | D8 | E8 | F8 | G8 | H8;

    pub const CORNERS: Bitboard = A1 | A8 | H1 | H8;
    pub const EDGES: Bitboard = RANK_1 | RANK_8 | A_FILE | H_FILE;

    pub const LIGHT_SQUARES: Bitboard =
        A8 | C8 | E8 | G8 |
        B7 | D7 | F7 | H7 |
        A6 | C6 | E6 | G6 |
        B5 | D5 | F5 | H5 |
        A4 | C4 | E4 | G4 |
        B3 | D3 | F3 | H3 |
        A2 | C2 | E2 | G2 |
        B1 | D1 | F1 | H1;

    pub const DARK_SQUARES: Bitboard =
        B8 | D8 | F8 | H8 |
        A7 | C7 | E7 | G7 |
        B6 | D6 | F6 | H6 |
        A5 | C5 | E5 | G5 |
        B4 | D4 | F4 | H4 |
        A3 | C3 | E3 | G3 |
        B2 | D2 | F2 | H2 |
        A1 | C1 | E1 | G1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_bitboard_display() {
        let bitboard = Bitboard::EMPTY;
        let formatted_bitboard = format!("{bitboard}");

        assert_eq!(
            formatted_bitboard,
            ". . . . . . . .\n. . . . . . . .\n. . . . . . . .\n. . . . . . . .\n. . . . . . . .\n. . . . . . . .\n. . . . . . . .\n. . . . . . . ."
        );
    }

    #[test]
    fn test_full_bitboard_display() {
        let bitboard = Bitboard::FULL;
        let formatted_bitboard = format!("{bitboard}");

        assert_eq!(
            formatted_bitboard,
            "* * * * * * * *\n* * * * * * * *\n* * * * * * * *\n* * * * * * * *\n* * * * * * * *\n* * * * * * * *\n* * * * * * * *\n* * * * * * * *"
        );
    }

    #[test]
    fn bitboard_size() {
        assert_eq!(size_of::<Bitboard>(), size_of::<u64>());
    }
}
