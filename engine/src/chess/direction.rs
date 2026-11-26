#[derive(Debug, Clone, Copy)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub const ALL: &'static [Self; 8] = &[
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    pub const CARDINAL: &'static [Self; 4] = &[Self::North, Self::East, Self::South, Self::West];

    pub const DIAGONAL: &'static [Self; 4] = &[Self::NorthEast, Self::SouthEast, Self::SouthWest, Self::NorthWest];
}

impl std::ops::Not for Direction {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::North => Self::South,
            Self::NorthEast => Self::SouthWest,
            Self::East => Self::West,
            Self::SouthEast => Self::NorthWest,
            Self::South => Self::North,
            Self::SouthWest => Self::NorthEast,
            Self::West => Self::East,
            Self::NorthWest => Self::SouthEast,
        }
    }
}
