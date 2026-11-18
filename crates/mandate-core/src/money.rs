//! Fixed-point money (DEC-041 rule 4): `i64` minor units (cents).
//! Arithmetic saturates rather than wrapping; scaling rounds half away from zero.

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign};

pub const MINOR_PER_UNIT: i64 = 100;

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Money(pub i64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub fn from_units(units: i64) -> Self {
        Money(units.saturating_mul(MINOR_PER_UNIT))
    }

    pub fn minor(self) -> i64 {
        self.0
    }

    /// Whole units, truncated toward zero.
    pub fn units(self) -> i64 {
        self.0 / MINOR_PER_UNIT
    }

    /// Multiply by a rational `num/den`, rounding half away from zero.
    /// Returns zero when `den == 0`.
    pub fn scale_ratio(self, num: i64, den: i64) -> Money {
        if den == 0 {
            return Money::ZERO;
        }
        let prod = (self.0 as i128) * (num as i128);
        let den = den as i128;
        let q = prod / den;
        let r = prod % den;
        let rounded = if (r.abs() * 2) >= den.abs() {
            q + if (prod < 0) ^ (den < 0) { -1 } else { 1 }
        } else {
            q
        };
        Money(rounded.clamp(i64::MIN as i128, i64::MAX as i128) as i64)
    }

    /// Multiply by a real factor via a fixed 1e6 basis, so float noise below
    /// one part per million never reaches the ledger.
    pub fn scale(self, factor: f64) -> Money {
        if !factor.is_finite() {
            return Money::ZERO;
        }
        let num = crate::math::round_half_away(factor * 1_000_000.0) as i64;
        self.scale_ratio(num, 1_000_000)
    }

    pub fn saturating_add(self, o: Money) -> Money {
        Money(self.0.saturating_add(o.0))
    }

    pub fn saturating_sub(self, o: Money) -> Money {
        Money(self.0.saturating_sub(o.0))
    }

    pub fn is_negative(self) -> bool {
        self.0 < 0
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, o: Money) -> Money {
        self.saturating_add(o)
    }
}
impl Sub for Money {
    type Output = Money;
    fn sub(self, o: Money) -> Money {
        self.saturating_sub(o)
    }
}
impl AddAssign for Money {
    fn add_assign(&mut self, o: Money) {
        *self = *self + o;
    }
}
impl SubAssign for Money {
    fn sub_assign(&mut self, o: Money) {
        *self = *self - o;
    }
}
impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Money {
        Money(self.0.saturating_neg())
    }
}
impl std::iter::Sum for Money {
    fn sum<I: Iterator<Item = Money>>(iter: I) -> Money {
        iter.fold(Money::ZERO, |a, b| a + b)
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.unsigned_abs();
        write!(f, "{sign}{}.{:02}", a / 100, a % 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn rounding_half_away_from_zero() {
        assert_eq!(Money(5).scale_ratio(1, 2), Money(3));
        assert_eq!(Money(-5).scale_ratio(1, 2), Money(-3));
        assert_eq!(Money(4).scale_ratio(1, 2), Money(2));
        assert_eq!(Money(100).scale(0.015), Money(2));
        assert_eq!(format!("{}", Money(-1234)), "-12.34");
    }

    proptest! {
        #[test]
        fn add_sub_inverse(a in -1_000_000_000_000i64..1_000_000_000_000, b in -1_000_000_000_000i64..1_000_000_000_000) {
            prop_assert_eq!((Money(a) + Money(b)) - Money(b), Money(a));
        }

        #[test]
        fn scale_by_one_is_identity(a in -1_000_000_000_000i64..1_000_000_000_000) {
            prop_assert_eq!(Money(a).scale(1.0), Money(a));
            prop_assert_eq!(Money(a).scale_ratio(7, 7), Money(a));
        }
    }
}
