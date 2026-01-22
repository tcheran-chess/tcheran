use std::cmp::Ordering;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Depth(u8);

impl Depth {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u8) -> Self {
        Self(value)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn as_i32(self) -> i32 {
        self.0 as i32
    }

    pub const fn idx(self) -> usize {
        self.0 as usize
    }
}

impl PartialEq<u8> for Depth {
    fn eq(&self, other: &u8) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<u8> for Depth {
    fn partial_cmp(&self, other: &u8) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}

impl std::ops::Add<Self> for Depth {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl std::ops::Add<u8> for Depth {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0.saturating_add(rhs))
    }
}

impl std::ops::Add<i8> for Depth {
    type Output = Self;

    fn add(self, rhs: i8) -> Self::Output {
        Self(self.0.saturating_add_signed(rhs))
    }
}

impl std::ops::AddAssign<u8> for Depth {
    fn add_assign(&mut self, rhs: u8) {
        self.0 = self.0.saturating_add(rhs);
    }
}

impl std::ops::Sub<Self> for Depth {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.saturating_sub(rhs.0))
    }
}

impl std::ops::Sub<u8> for Depth {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        Self(self.0.saturating_sub(rhs))
    }
}

impl std::ops::SubAssign<u8> for Depth {
    fn sub_assign(&mut self, rhs: u8) {
        self.0 = self.0.saturating_sub(rhs);
    }
}

impl std::ops::Mul<i32> for Depth {
    type Output = i32;

    fn mul(self, rhs: i32) -> Self::Output {
        i32::from(self.0) * rhs
    }
}

impl std::ops::Mul<Self> for Depth {
    type Output = i32;

    fn mul(self, rhs: Self) -> Self::Output {
        i32::from(self.0) * i32::from(rhs.0)
    }
}

impl std::ops::Div<u8> for Depth {
    type Output = Self;

    fn div(self, rhs: u8) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl std::ops::Div<i32> for Depth {
    type Output = i32;

    fn div(self, rhs: i32) -> Self::Output {
        i32::from(self.0) / rhs
    }
}

impl std::fmt::Display for Depth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct DepthReduction(u32);

impl DepthReduction {
    pub fn new(v: u8) -> Self {
        Self(u32::from(v) * 1024)
    }

    #[inline]
    pub fn reduce_more_if(&mut self, predicate: bool, factor: u32) {
        self.0 = self.0.saturating_add(u32::from(predicate) * factor);
    }

    #[inline]
    pub fn reduce_less_if(&mut self, predicate: bool, factor: u32) {
        self.0 = self.0.saturating_sub(u32::from(predicate) * factor);
    }

    #[inline]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "In practice, we won't reduce by more than u8::MAX"
    )]
    pub fn value(&self) -> u8 {
        (self.0 / 1024) as u8
    }
}
