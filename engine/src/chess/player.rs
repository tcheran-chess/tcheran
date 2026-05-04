#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub const N: usize = 2;
    pub const ALL: [Self; Self::N] = [Self::White, Self::Black];

    pub const fn idx(self) -> usize {
        self as usize
    }

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

    fn index(&self, player: Player) -> &Self::Output {
        unsafe { self.get_unchecked(player.idx()) }
    }
}

impl<T> std::ops::IndexMut<Player> for [T; Player::N] {
    fn index_mut(&mut self, player: Player) -> &mut Self::Output {
        unsafe { self.get_unchecked_mut(player.idx()) }
    }
}
