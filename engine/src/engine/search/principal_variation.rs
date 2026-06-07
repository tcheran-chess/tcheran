use crate::{chess::moves::Move, engine::search::MAX_PLIES_ARRAY_SIZE, util::arrayvec::ArrayVec};

#[derive(Clone)]
pub struct PrincipalVariation(ArrayVec<Move, MAX_PLIES_ARRAY_SIZE>);

impl PrincipalVariation {
    #[inline]
    pub const fn new() -> Self {
        Self(ArrayVec::new())
    }

    #[inline]
    pub fn clear(&mut self) {
        self.0.clear();
    }

    #[inline]
    pub fn push(&mut self, mv: Move, child_pv: &Self) {
        self.0.clear();
        self.0.push(mv);

        for mv in child_pv.iter() {
            self.0.push(*mv);
        }
    }

    #[inline]
    pub fn append(&mut self, mv: Move) {
        self.0.push(mv);
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        self.0.iter()
    }

    #[inline]
    pub fn first(&self) -> Option<&Move> {
        self.0.iter().next()
    }

    pub fn len(&self) -> u8 {
        u8::try_from(self.0.len()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chess::square::squares::all::*;

    #[test]
    fn test_new_pv_has_no_moves() {
        let pv = PrincipalVariation::new();
        assert_eq!(pv.len(), 0);
    }

    #[test]
    fn test_combining_pvs_works() {
        let pv = PrincipalVariation::new();

        let mut pv_2 = PrincipalVariation::new();
        pv_2.push(Move::quiet(A1, B1), &pv);

        let mut pv_3 = PrincipalVariation::new();
        pv_3.push(Move::quiet(C1, D1), &pv_2);

        let mut pv_4 = PrincipalVariation::new();
        pv_4.push(Move::quiet(E1, F1), &pv_3);

        assert_eq!(pv_4.len(), 3);
        assert_eq!(pv_4.0.get(0).from(), E1);
        assert_eq!(pv_4.0.get(1).from(), C1);
        assert_eq!(pv_4.0.get(2).from(), A1);
    }
}
