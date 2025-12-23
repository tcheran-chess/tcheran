#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub const N: usize = 2;
    pub const ALL: [Self; 2] = [Self::White, Self::Black];

    pub fn other(self) -> Self {
        !self
    }
}

impl std::ops::Not for Player {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
}

impl<T> std::ops::Index<Player> for [T; Player::N] {
    type Output = T;

    fn index(&self, index: Player) -> &Self::Output {
        unsafe { self.get_unchecked(index as usize) }
    }
}

impl<T> std::ops::IndexMut<Player> for [T; Player::N] {
    fn index_mut(&mut self, index: Player) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(index as usize) }
    }
}
