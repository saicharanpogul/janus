//! `exp(x)` for Q32.32 values via range reduction + 9-term Taylor.
//!
//! Approach:
//! 1. Range-reduce `x` to `[0, ln(2))` by writing `x = k·ln(2) + r`
//!    where `k = floor(x / ln(2))` and `r ∈ [0, ln(2))`. Then
//!    `exp(x) = 2^k · exp(r)`.
//! 2. On `r ∈ [0, ln(2))`, compute `exp(r)` via Taylor series
//!    `1 + r + r²/2! + r³/3! + … + r^8/8!`. On this range the
//!    truncation error is bounded by `r^9/9! < (ln2)^9 / 362880 ≈ 1e-7`,
//!    which is comfortably below Q32.32's `2^-32 ≈ 2.3e-10` precision
//!    floor when we keep the full u128 mul width.
//! 3. Multiply by `2^k` (left-shift the integer part of the result).
//!
//! Returns `None` on overflow (k too large; result exceeds Q32.32 max).

use crate::{Q32_32, LN2_Q};

/// `exp(x)` in Q32.32. Returns `None` if the result overflows.
///
/// Domain: `x ≥ 0`. Negative inputs aren't supported because Q32.32
/// is unsigned; for LMSR we always pre-shift to keep `q_yes - q_min ≥ 0`.
pub fn exp_q(x: Q32_32) -> Option<Q32_32> {
    // Step 1: range reduction. Find k such that x ≈ k·ln(2) + r.
    let k = (x.0 / LN2_Q) as u32;
    // k can't exceed ~31 before 2^k overflows the integer part. Cap early.
    if k >= 32 {
        return None;
    }
    let k_times_ln2 = Q32_32::from_bits(LN2_Q.checked_mul(k as u64)?);
    let r = x.checked_sub(k_times_ln2)?;

    // Step 2: Taylor series exp(r) = Σ r^n / n! for n = 0..8.
    // Precompute reciprocal factorials in Q32.32 once:
    //   1/0! = 1, 1/1! = 1, 1/2! = 0.5, 1/3! = 1/6, ..., 1/8! = 1/40320.
    // We compute them dynamically to avoid hardcoded magic constants.
    let mut sum = Q32_32::ONE; // 1
    let mut term = Q32_32::ONE; // r^0 / 0! starts as 1
    for n in 1u64..=8 {
        // term_{n} = term_{n-1} × r / n
        term = (term.checked_mul(r)?).checked_div(Q32_32::from_int(n as u32))?;
        sum = sum.checked_add(term)?;
    }

    // Step 3: multiply by 2^k by left-shifting the entire 64-bit value.
    let shifted = sum.0.checked_shl(k)?;
    Some(Q32_32(shifted))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exp_zero_is_one() {
        let e = exp_q(Q32_32::ZERO).unwrap();
        assert_eq!(e.floor(), 1);
        assert_eq!(e.0, ONE);
    }

    #[test]
    fn exp_one_close_to_e() {
        let e = exp_q(Q32_32::from_int(1)).unwrap();
        let approx = e.to_f64();
        assert!((approx - core::f64::consts::E).abs() < 1e-6, "exp(1) ≈ e, got {}", approx);
    }

    #[test]
    fn exp_small_values() {
        for x in [0.0_f64, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0] {
            let q = Q32_32::from_f64(x);
            let r = exp_q(q).unwrap().to_f64();
            let truth = x.exp();
            // 1e-5 relative tolerance — accounts for Q32.32 precision loss.
            let rel_err = (r - truth).abs() / truth;
            assert!(
                rel_err < 1e-5,
                "exp({}) = {} (expected {}, rel err {})",
                x,
                r,
                truth,
                rel_err
            );
        }
    }

    #[test]
    fn exp_overflows_above_int_range() {
        // x > ~22 would push exp(x) past 2^32; we should return None.
        let big = Q32_32::from_int(40);
        assert!(exp_q(big).is_none(), "exp(40) must report overflow");
    }
}
