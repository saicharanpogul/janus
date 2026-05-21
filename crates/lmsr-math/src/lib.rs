//! Q32.32 fixed-point arithmetic for the LMSR cost function on BPF.
//!
//! Why fixed-point: Solana BPF doesn't expose IEEE-754 floats; we need
//! deterministic, overflow-safe arithmetic for the cost function
//!     C(q_yes, q_no) = b · ln(exp(q_yes / b) + exp(q_no / b))
//! so that every validator computes the same swap price for the same
//! reserves regardless of platform.
//!
//! Q32.32 = 64-bit unsigned, with the top 32 bits as the integer part
//! and the bottom 32 bits as the fractional part. This gives a range
//! of [0, 2^32) and ~9-10 decimal digits of fractional precision —
//! enough for LMSR liquidity parameters up to a few billion units with
//! 6-decimal-place outcome tokens.
//!
//! All operations panic on overflow under `std` (caught by tests);
//! production callers use the `checked_*` variants which return
//! `Option<Q32_32>`.

#![cfg_attr(not(feature = "std"), no_std)]

mod cost;
mod exp;
mod ln;

pub use cost::{buy_no_cost, buy_yes_cost, cost, price_yes};
pub use exp::exp_q;
pub use ln::ln_q;

/// Scale factor: 2^32. One unit of the integer part.
pub const ONE: u64 = 1u64 << 32;
/// 2^16, useful as a half-scale constant for some range reductions.
pub const HALF_SCALE: u64 = 1u64 << 16;
/// ln(2) in Q32.32, used by the exp range-reduction step. Computed
/// once as `f64.ln(2.0) * 2^32` and pinned as a constant.
/// Value: 0.6931471805599453 × 2^32 = 2977044472.5...
pub const LN2_Q: u64 = 2977044472;

/// Q32.32 fixed-point unsigned scalar.
///
/// Layout: `bits = integer_part << 32 | fractional_part`.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Q32_32(pub u64);

impl Q32_32 {
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(ONE);
    pub const MAX: Self = Self(u64::MAX);

    /// Construct from an integer (the value is `n` exactly, not `n/2^32`).
    /// Panics if `n` doesn't fit in the integer part.
    pub const fn from_int(n: u32) -> Self {
        Self((n as u64) << 32)
    }

    /// Construct from raw bits — `Q32_32(bits)`.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Integer part — drops fractional bits.
    pub const fn floor(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Fractional part as bits (0 .. 2^32-1).
    pub const fn frac_bits(self) -> u32 {
        self.0 as u32
    }

    /// Round to nearest integer.
    pub const fn round(self) -> u32 {
        ((self.0 + (1u64 << 31)) >> 32) as u32
    }

    /// Convert to `f64` (for testing only — never use on BPF).
    #[cfg(feature = "std")]
    pub fn to_f64(self) -> f64 {
        (self.0 as f64) / (ONE as f64)
    }

    /// Construct from `f64`. Saturates on overflow.
    #[cfg(feature = "std")]
    pub fn from_f64(x: f64) -> Self {
        if x < 0.0 {
            Self::ZERO
        } else if x >= (u64::MAX as f64) / (ONE as f64) {
            Self::MAX
        } else {
            Self((x * (ONE as f64)) as u64)
        }
    }

    // ----- Arithmetic -----

    pub const fn checked_add(self, other: Self) -> Option<Self> {
        match self.0.checked_add(other.0) {
            Some(s) => Some(Self(s)),
            None => None,
        }
    }

    pub const fn checked_sub(self, other: Self) -> Option<Self> {
        match self.0.checked_sub(other.0) {
            Some(s) => Some(Self(s)),
            None => None,
        }
    }

    /// `self × other`, with a single rounding step. Uses u128
    /// intermediate to avoid mid-multiply overflow.
    pub const fn checked_mul(self, other: Self) -> Option<Self> {
        let prod = (self.0 as u128) * (other.0 as u128);
        // Right-shift by 32 to undo the doubled scale factor; round to nearest.
        let scaled = (prod + (1u128 << 31)) >> 32;
        if scaled > (u64::MAX as u128) {
            None
        } else {
            Some(Self(scaled as u64))
        }
    }

    /// `self / other`. Uses u128 intermediate, returns None on
    /// divide-by-zero.
    pub const fn checked_div(self, other: Self) -> Option<Self> {
        if other.0 == 0 {
            return None;
        }
        let num = (self.0 as u128) << 32;
        let result = num / (other.0 as u128);
        if result > (u64::MAX as u128) {
            None
        } else {
            Some(Self(result as u64))
        }
    }
}

impl core::ops::Add for Q32_32 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        self.checked_add(other).expect("Q32_32 add overflow")
    }
}
impl core::ops::Sub for Q32_32 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        self.checked_sub(other).expect("Q32_32 sub overflow")
    }
}
impl core::ops::Mul for Q32_32 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        self.checked_mul(other).expect("Q32_32 mul overflow")
    }
}
impl core::ops::Div for Q32_32 {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        self.checked_div(other).expect("Q32_32 div underflow or overflow")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_int_round_trip() {
        for n in [0u32, 1, 42, 1_000, u32::MAX / 2] {
            assert_eq!(Q32_32::from_int(n).floor(), n);
        }
    }

    #[test]
    fn arithmetic_smoke() {
        let a = Q32_32::from_int(3);
        let b = Q32_32::from_int(4);
        assert_eq!((a + b).floor(), 7);
        assert_eq!((b - a).floor(), 1);
        // 3 * 4 = 12
        assert_eq!((a * b).floor(), 12);
        // 12 / 4 = 3
        assert_eq!(((a * b) / b).floor(), 3);
    }

    #[test]
    fn f64_round_trip_smoke() {
        for x in [0.0_f64, 0.5, 1.0, 1.5, 100.0, 12345.6789] {
            let q = Q32_32::from_f64(x);
            let back = q.to_f64();
            assert!((back - x).abs() < 1e-8, "round-trip {} → {}", x, back);
        }
    }
}
