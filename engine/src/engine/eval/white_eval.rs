use crate::{chess::Player, engine::eval::Eval};

/// A classical evaluation value from the white player's perspective
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct WhiteEval(pub i32);

impl WhiteEval {
    pub fn for_player(self, player: Player) -> Eval {
        match player {
            Player::White => Eval(self.0),
            Player::Black => Eval((-self).0),
        }
    }
}

impl std::ops::Add for WhiteEval {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign for WhiteEval {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::ops::Sub for WhiteEval {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::SubAssign for WhiteEval {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl std::ops::Mul<i32> for WhiteEval {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl std::ops::Neg for WhiteEval {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(self.0.saturating_neg())
    }
}

impl std::fmt::Display for WhiteEval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let formatted_value = f64::from(self.0) / 100.0;
        write!(f, "{formatted_value}")
    }
}
