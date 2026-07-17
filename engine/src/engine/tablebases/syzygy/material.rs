use std::{cmp::Ordering, fmt};

use crate::chess::{Board, Piece, PieceKind, Player};

#[derive(Clone, Eq, PartialEq, Hash)]
pub(crate) struct MaterialSide {
    by_role: [u8; PieceKind::N],
}

impl MaterialSide {
    const fn empty() -> MaterialSide {
        MaterialSide {
            by_role: [0; PieceKind::N],
        }
    }

    fn from_str_part(s: &str) -> Result<MaterialSide, ()> {
        let mut side = MaterialSide::empty();
        for ch in s.as_bytes() {
            let role = Self::piece_kind_from_char(char::from(*ch)).ok_or(())?;
            side.by_role[role] += 1;
        }
        Ok(side)
    }

    pub const fn piece_kind_from_char(ch: char) -> Option<PieceKind> {
        match ch {
            'P' | 'p' => Some(PieceKind::Pawn),
            'N' | 'n' => Some(PieceKind::Knight),
            'B' | 'b' => Some(PieceKind::Bishop),
            'R' | 'r' => Some(PieceKind::Rook),
            'Q' | 'q' => Some(PieceKind::Queen),
            'K' | 'k' => Some(PieceKind::King),
            _ => None,
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.by_role.into_iter().map(usize::from).sum()
    }

    pub(crate) fn has_pawns(&self) -> bool {
        self.by_role[PieceKind::Pawn] > 0
    }

    fn unique_roles(&self) -> usize {
        self.by_role.into_iter().filter(|c| *c == 1).count()
    }
}

impl Ord for MaterialSide {
    fn cmp(&self, other: &MaterialSide) -> Ordering {
        use PieceKind::*;

        self.count()
            .cmp(&other.count())
            .then_with(|| self.by_role[King].cmp(&other.by_role[King]))
            .then_with(|| self.by_role[Queen].cmp(&other.by_role[Queen]))
            .then_with(|| self.by_role[Rook].cmp(&other.by_role[Rook]))
            .then_with(|| self.by_role[Bishop].cmp(&other.by_role[Bishop]))
            .then_with(|| self.by_role[Knight].cmp(&other.by_role[Knight]))
            .then_with(|| self.by_role[Pawn].cmp(&other.by_role[Pawn]))
    }
}

impl PartialOrd for MaterialSide {
    fn partial_cmp(&self, other: &MaterialSide) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for MaterialSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for kind in PieceKind::ALL {
            let count = self.by_role[kind];
            f.write_str(&format!("{:?}", kind).repeat(usize::from(count)))?;
        }
        Ok(())
    }
}

impl fmt::Debug for MaterialSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count() > 0 {
            <Self as fmt::Display>::fmt(self, f)
        } else {
            f.write_str("-")
        }
    }
}

/// A material key.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Material {
    pub(crate) by_color: [MaterialSide; Player::N],
}

impl Material {
    fn empty() -> Material {
        Material {
            by_color: [const { MaterialSide::empty() }; Player::N],
        }
    }

    /// Get the material configuration for a [`Board`].
    pub fn from_board(board: &Board) -> Material {
        use Player::*;

        Material {
            by_color: [
                MaterialSide {
                    by_role: [
                        board.pawns(White).count(),
                        board.knights(White).count(),
                        board.bishops(White).count(),
                        board.rooks(White).count(),
                        board.queens(White).count(),
                        board.king(White).count(),
                    ],
                },
                MaterialSide {
                    by_role: [
                        board.pawns(Black).count(),
                        board.knights(Black).count(),
                        board.bishops(Black).count(),
                        board.rooks(Black).count(),
                        board.queens(Black).count(),
                        board.king(Black).count(),
                    ],
                },
            ],
        }
    }

    pub(crate) fn from_iter<I>(iter: I) -> Material
    where
        I: IntoIterator<Item = Piece>,
    {
        let mut material = Material::empty();
        for piece in iter {
            material.by_color[piece.player].by_role[piece.kind] += 1;
        }
        material
    }

    pub(crate) fn from_str(s: &str) -> Result<Material, ()> {
        if s.len() > 64 + 1 {
            return Err(());
        }

        let (white, black) = s.split_once('v').ok_or(())?;
        Ok(Material {
            by_color: [
                MaterialSide::from_str_part(white)?,
                MaterialSide::from_str_part(black)?,
            ],
        })
    }

    pub(crate) fn count(&self) -> usize {
        self.by_color.iter().map(|side| side.count()).sum()
    }

    pub(crate) fn is_symmetric(&self) -> bool {
        use Player::*;

        self.by_color[White] == self.by_color[Black]
    }

    pub(crate) fn has_pawns(&self) -> bool {
        self.by_color.iter().any(|side| side.has_pawns())
    }

    pub(crate) fn unique_pieces(&self) -> usize {
        self.by_color.iter().map(|side| side.unique_roles()).sum()
    }

    pub(crate) fn min_like_man(&self) -> usize {
        usize::from(
            self.by_color
                .iter()
                .flat_map(|side| side.by_role)
                .filter(|c| 2 <= *c)
                .min()
                .unwrap_or(0),
        )
    }

    pub(crate) fn into_swapped(self) -> Material {
        Material {
            by_color: [
                self.by_color[Player::Black].clone(),
                self.by_color[Player::White].clone(),
            ],
        }
    }

    pub(crate) fn to_normalized(&self) -> NormalizedMaterial {
        use Player::*;

        let white = self.by_color[White].clone();
        let black = self.by_color[Black].clone();

        NormalizedMaterial(Material {
            by_color: if white < black {
                [black, white]
            } else {
                [white, black]
            },
        })
    }
}

impl fmt::Display for Material {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}v{}", self.by_color[Player::White], self.by_color[Player::Black])
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct NormalizedMaterial(Material);

impl NormalizedMaterial {
    pub fn inner(&self) -> &Material {
        &self.0
    }
}
