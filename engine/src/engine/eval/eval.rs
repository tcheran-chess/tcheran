use crate::{
    chess::{Game, Player},
    engine::eval::nnue::NetworkStack,
};

#[cfg(not(feature = "datagen"))]
fn scale_eval(eval: Eval, game: &Game) -> Eval {
    use crate::engine::params::*;

    let material = i32::from(game.board.all_knights().count()) * see_knight_value()
        + i32::from(game.board.all_bishops().count()) * see_bishop_value()
        + i32::from(game.board.all_rooks().count()) * see_rook_value()
        + i32::from(game.board.all_queens().count()) * see_queen_value();

    let scale = material_scale_base() + material / material_scale_divisor();

    Eval((eval.0 * scale) / 1024)
}

pub fn eval(nnue: &mut NetworkStack, game: &Game) -> Eval {
    let eval = nnue.evaluate(game);

    #[cfg(not(feature = "datagen"))]
    let eval = scale_eval(eval, game);

    eval.clamp_to_non_mate()
}

/// An evaluation from the active player's perspective
///
/// In algorithms like negamax, in order for the same code to work
/// for both players, we need to players to try maximising their score.
/// Therefore, we need a way to represent an evaluation of the board as
/// seen by the active player in any given game state.
///
/// This can be easily turned back into a 'classical' evaluation (i.e.
/// from white's perspective).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Eval(pub i32);

impl Eval {
    pub const MAX: Self = Self(Self::MATE + 1);
    pub const MIN: Self = Self(Self::MATED - 1);
    pub const NONE: Self = Self(Self::MATED - 2);
    pub const DRAW: Self = Self(0);

    const MATE: i32 = 32000;
    const MATED: i32 = -Self::MATE;

    const TB_MATE: i32 = 31000;
    const TB_MATED: i32 = -Self::TB_MATE;

    const MAX_EVAL: i32 = 30000;
    const MIN_EVAL: i32 = -Self::MAX_EVAL;

    pub const fn new(eval: i32) -> Self {
        Self(eval)
    }

    pub fn mate_in(ply: u8) -> Self {
        Self(Self::MATE - i32::from(ply))
    }

    pub fn tb_mate_in(ply: u8) -> Self {
        Self(Self::TB_MATE - i32::from(ply))
    }

    pub fn mated_in(ply: u8) -> Self {
        Self(Self::MATED + i32::from(ply))
    }

    pub fn tb_mated_in(ply: u8) -> Self {
        Self(Self::TB_MATED + i32::from(ply))
    }

    #[inline]
    pub fn is_decisive(self) -> bool {
        self.is_win() || self.is_loss()
    }

    #[inline]
    pub fn is_tb(self) -> bool {
        (self.is_win() && self.0 <= Self::TB_MATE) || (self.is_loss() && self.0 >= Self::TB_MATED)
    }

    #[inline]
    pub fn is_win(self) -> bool {
        self.0 > Self::MAX_EVAL
    }

    #[inline]
    pub fn is_loss(self) -> bool {
        self.0 < Self::MIN_EVAL
    }

    #[inline]
    pub fn moves_to_mate(self) -> i32 {
        assert!(self.is_decisive() && !self.is_tb());

        if self.is_win() {
            (Self::MATE - self.0 + 1) / 2
        } else {
            (Self::MATED - self.0) / 2
        }
    }

    pub fn clamp_to_non_mate(self) -> Self {
        self.clamp(Self(Self::MIN_EVAL), Self(Self::MAX_EVAL))
    }

    pub fn to_white_eval(self, player: Player) -> WhiteEval {
        match player {
            Player::White => WhiteEval(self.0),
            Player::Black => -WhiteEval(self.0),
        }
    }
}

impl std::ops::Add for Eval {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::Add<i32> for Eval {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl std::ops::AddAssign for Eval {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl std::ops::Sub for Eval {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Sub<i32> for Eval {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl std::ops::SubAssign for Eval {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl std::ops::Mul<i32> for Eval {
    type Output = Self;

    fn mul(self, rhs: i32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl std::ops::Neg for Eval {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(self.0.saturating_neg())
    }
}

impl std::ops::Div<i32> for Eval {
    type Output = Self;

    fn div(self, rhs: i32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

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
