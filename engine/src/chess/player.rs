#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Player {
    White,
    Black,
}

impl Player {
    pub const N: usize = 2;

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

#[derive(Debug, Clone)]
pub struct ByPlayer<T>([T; Player::N]);

impl<T> ByPlayer<T> {
    pub fn new(white: T, black: T) -> Self {
        Self([white, black])
    }

    #[inline(always)]
    pub fn white(&self) -> &T {
        self.for_player(Player::White)
    }

    #[inline(always)]
    pub fn black(&self) -> &T {
        self.for_player(Player::Black)
    }

    #[inline(always)]
    pub fn for_player(&self, player: Player) -> &T {
        &self.0[player]
    }

    #[inline(always)]
    pub fn for_player_mut(&mut self, player: Player) -> &mut T {
        &mut self.0[player]
    }

    #[inline(always)]
    pub fn inner(&self) -> &[T; Player::N] {
        &self.0
    }
}
