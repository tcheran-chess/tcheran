use crate::chess::{
    direction::Direction,
    player::Player,
    square::{File, Rank, Square},
};

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
        Square::from_bitboard(self)
    }

    #[inline(always)]
    pub const fn invert(self) -> Self {
        Self(!self.0)
    }

    #[inline(always)]
    pub const fn lsb(self) -> Self {
        Self((1_u64).wrapping_shl(self.0.trailing_zeros()))
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
    pub fn without(&mut self, square: Square) -> Self {
        Self(self.0 & square.bb().invert().0)
    }

    #[inline(always)]
    #[expect(clippy::cast_possible_truncation, reason = "u64 can have at most 64 ones")]
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

    #[inline(always)]
    pub const fn flip_vertically(self) -> Self {
        Self(u64::swap_bytes(self.0))
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

impl std::ops::BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[cfg(test)]
impl std::ops::BitOr<Square> for Bitboard {
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

#[cfg(test)]
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
    use crate::chess::square::squares::all::*;

    pub const fn back_rank(player: Player) -> Bitboard {
        match player {
            Player::White => RANK_1,
            Player::Black => RANK_8,
        }
    }

    pub const fn pawn_back_rank(player: Player) -> Bitboard {
        match player {
            Player::White => RANK_2,
            Player::Black => RANK_7,
        }
    }

    pub const fn double_push_rank(player: Player) -> Bitboard {
        match player {
            Player::White => RANK_3,
            Player::Black => RANK_6,
        }
    }

    pub const A1_BB: Bitboard = A1.bb();
    pub const A2_BB: Bitboard = A2.bb();
    pub const A3_BB: Bitboard = A3.bb();
    pub const A4_BB: Bitboard = A4.bb();
    pub const A5_BB: Bitboard = A5.bb();
    pub const A6_BB: Bitboard = A6.bb();
    pub const A7_BB: Bitboard = A7.bb();
    pub const A8_BB: Bitboard = A8.bb();

    pub const B1_BB: Bitboard = B1.bb();
    pub const B2_BB: Bitboard = B2.bb();
    pub const B3_BB: Bitboard = B3.bb();
    pub const B4_BB: Bitboard = B4.bb();
    pub const B5_BB: Bitboard = B5.bb();
    pub const B6_BB: Bitboard = B6.bb();
    pub const B7_BB: Bitboard = B7.bb();
    pub const B8_BB: Bitboard = B8.bb();

    pub const C1_BB: Bitboard = C1.bb();
    pub const C2_BB: Bitboard = C2.bb();
    pub const C3_BB: Bitboard = C3.bb();
    pub const C4_BB: Bitboard = C4.bb();
    pub const C5_BB: Bitboard = C5.bb();
    pub const C6_BB: Bitboard = C6.bb();
    pub const C7_BB: Bitboard = C7.bb();
    pub const C8_BB: Bitboard = C8.bb();

    pub const D1_BB: Bitboard = D1.bb();
    pub const D2_BB: Bitboard = D2.bb();
    pub const D3_BB: Bitboard = D3.bb();
    pub const D4_BB: Bitboard = D4.bb();
    pub const D5_BB: Bitboard = D5.bb();
    pub const D6_BB: Bitboard = D6.bb();
    pub const D7_BB: Bitboard = D7.bb();
    pub const D8_BB: Bitboard = D8.bb();

    pub const E1_BB: Bitboard = E1.bb();
    pub const E2_BB: Bitboard = E2.bb();
    pub const E3_BB: Bitboard = E3.bb();
    pub const E4_BB: Bitboard = E4.bb();
    pub const E5_BB: Bitboard = E5.bb();
    pub const E6_BB: Bitboard = E6.bb();
    pub const E7_BB: Bitboard = E7.bb();
    pub const E8_BB: Bitboard = E8.bb();

    pub const F1_BB: Bitboard = F1.bb();
    pub const F2_BB: Bitboard = F2.bb();
    pub const F3_BB: Bitboard = F3.bb();
    pub const F4_BB: Bitboard = F4.bb();
    pub const F5_BB: Bitboard = F5.bb();
    pub const F6_BB: Bitboard = F6.bb();
    pub const F7_BB: Bitboard = F7.bb();
    pub const F8_BB: Bitboard = F8.bb();

    pub const G1_BB: Bitboard = G1.bb();
    pub const G2_BB: Bitboard = G2.bb();
    pub const G3_BB: Bitboard = G3.bb();
    pub const G4_BB: Bitboard = G4.bb();
    pub const G5_BB: Bitboard = G5.bb();
    pub const G6_BB: Bitboard = G6.bb();
    pub const G7_BB: Bitboard = G7.bb();
    pub const G8_BB: Bitboard = G8.bb();

    pub const H1_BB: Bitboard = H1.bb();
    pub const H2_BB: Bitboard = H2.bb();
    pub const H3_BB: Bitboard = H3.bb();
    pub const H4_BB: Bitboard = H4.bb();
    pub const H5_BB: Bitboard = H5.bb();
    pub const H6_BB: Bitboard = H6.bb();
    pub const H7_BB: Bitboard = H7.bb();
    pub const H8_BB: Bitboard = H8.bb();

    pub const A_FILE: Bitboard = Bitboard::new(A1_BB.0 | A2_BB.0 | A3_BB.0 | A4_BB.0 | A5_BB.0 | A6_BB.0 | A7_BB.0 | A8_BB.0);
    pub const B_FILE: Bitboard = Bitboard::new(B1_BB.0 | B2_BB.0 | B3_BB.0 | B4_BB.0 | B5_BB.0 | B6_BB.0 | B7_BB.0 | B8_BB.0);
    pub const C_FILE: Bitboard = Bitboard::new(C1_BB.0 | C2_BB.0 | C3_BB.0 | C4_BB.0 | C5_BB.0 | C6_BB.0 | C7_BB.0 | C8_BB.0);
    pub const D_FILE: Bitboard = Bitboard::new(D1_BB.0 | D2_BB.0 | D3_BB.0 | D4_BB.0 | D5_BB.0 | D6_BB.0 | D7_BB.0 | D8_BB.0);
    pub const E_FILE: Bitboard = Bitboard::new(E1_BB.0 | E2_BB.0 | E3_BB.0 | E4_BB.0 | E5_BB.0 | E6_BB.0 | E7_BB.0 | E8_BB.0);
    pub const F_FILE: Bitboard = Bitboard::new(F1_BB.0 | F2_BB.0 | F3_BB.0 | F4_BB.0 | F5_BB.0 | F6_BB.0 | F7_BB.0 | F8_BB.0);
    pub const G_FILE: Bitboard = Bitboard::new(G1_BB.0 | G2_BB.0 | G3_BB.0 | G4_BB.0 | G5_BB.0 | G6_BB.0 | G7_BB.0 | G8_BB.0);
    pub const H_FILE: Bitboard = Bitboard::new(H1_BB.0 | H2_BB.0 | H3_BB.0 | H4_BB.0 | H5_BB.0 | H6_BB.0 | H7_BB.0 | H8_BB.0);

    pub const RANK_1: Bitboard = Bitboard::new(A1_BB.0 | B1_BB.0 | C1_BB.0 | D1_BB.0 | E1_BB.0 | F1_BB.0 | G1_BB.0 | H1_BB.0);
    pub const RANK_2: Bitboard = Bitboard::new(A2_BB.0 | B2_BB.0 | C2_BB.0 | D2_BB.0 | E2_BB.0 | F2_BB.0 | G2_BB.0 | H2_BB.0);
    pub const RANK_3: Bitboard = Bitboard::new(A3_BB.0 | B3_BB.0 | C3_BB.0 | D3_BB.0 | E3_BB.0 | F3_BB.0 | G3_BB.0 | H3_BB.0);
    pub const RANK_4: Bitboard = Bitboard::new(A4_BB.0 | B4_BB.0 | C4_BB.0 | D4_BB.0 | E4_BB.0 | F4_BB.0 | G4_BB.0 | H4_BB.0);
    pub const RANK_5: Bitboard = Bitboard::new(A5_BB.0 | B5_BB.0 | C5_BB.0 | D5_BB.0 | E5_BB.0 | F5_BB.0 | G5_BB.0 | H5_BB.0);
    pub const RANK_6: Bitboard = Bitboard::new(A6_BB.0 | B6_BB.0 | C6_BB.0 | D6_BB.0 | E6_BB.0 | F6_BB.0 | G6_BB.0 | H6_BB.0);
    pub const RANK_7: Bitboard = Bitboard::new(A7_BB.0 | B7_BB.0 | C7_BB.0 | D7_BB.0 | E7_BB.0 | F7_BB.0 | G7_BB.0 | H7_BB.0);
    pub const RANK_8: Bitboard = Bitboard::new(A8_BB.0 | B8_BB.0 | C8_BB.0 | D8_BB.0 | E8_BB.0 | F8_BB.0 | G8_BB.0 | H8_BB.0);

    pub const CORNERS: Bitboard = Bitboard::new(A1_BB.0 | A8_BB.0 | H1_BB.0 | H8_BB.0);
    pub const EDGES: Bitboard = Bitboard::new(RANK_1.0 | RANK_8.0 | A_FILE.0 | H_FILE.0);

    pub const LIGHT_SQUARES: Bitboard = Bitboard::new(
        A8_BB.0 | C8_BB.0 | E8_BB.0 | G8_BB.0 |
        B7_BB.0 | D7_BB.0 | F7_BB.0 | H7_BB.0 |
        A6_BB.0 | C6_BB.0 | E6_BB.0 | G6_BB.0 |
        B5_BB.0 | D5_BB.0 | F5_BB.0 | H5_BB.0 |
        A4_BB.0 | C4_BB.0 | E4_BB.0 | G4_BB.0 |
        B3_BB.0 | D3_BB.0 | F3_BB.0 | H3_BB.0 |
        A2_BB.0 | C2_BB.0 | E2_BB.0 | G2_BB.0 |
        B1_BB.0 | D1_BB.0 | F1_BB.0 | H1_BB.0 );

    pub const DARK_SQUARES: Bitboard = Bitboard::new(
        B8_BB.0 | D8_BB.0 | F8_BB.0 | H8_BB.0 |
        A7_BB.0 | C7_BB.0 | E7_BB.0 | G7_BB.0 |
        B6_BB.0 | D6_BB.0 | F6_BB.0 | H6_BB.0 |
        A5_BB.0 | C5_BB.0 | E5_BB.0 | G5_BB.0 |
        B4_BB.0 | D4_BB.0 | F4_BB.0 | H4_BB.0 |
        A3_BB.0 | C3_BB.0 | E3_BB.0 | G3_BB.0 |
        B2_BB.0 | D2_BB.0 | F2_BB.0 | H2_BB.0 |
        A1_BB.0 | C1_BB.0 | E1_BB.0 | G1_BB.0 );
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
        assert_eq!(std::mem::size_of::<Bitboard>(), std::mem::size_of::<u64>());
    }
}
