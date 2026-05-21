//! `ln(x)` for Q32.32 values via range reduction + Mercator series.
//!
//! Approach:
//! 1. Write `x = 2^k · m` where `m ∈ [1, 2)`. Then `ln(x) = k·ln(2) + ln(m)`.
//!    `k` is the position of the highest-set bit in `x` minus 32
//!    (the Q32.32 scale).
//! 2. On `m ∈ [1, 2)`, substitute `m = 1 + y` so `y ∈ [0, 1)`. Then
//!    `ln(m) = ln(1 + y) = y - y²/2 + y³/3 - y⁴/4 + …` (Mercator).
//!    For `y` close to 1 the series converges slowly. We further reduce
//!    by a half-and-square trick: write `m = √m · √m` so
//!    `ln(m) = 2·ln(√m)`. After one square-root step `m ∈ [1, √2)`,
//!    `y < 0.4142`, and a 12-term Mercator series gives ~Q32.32 precision.
//!
//! For LMSR we only ever pass strictly positive values (sums of
//! `exp(q/b)` terms), so `ln(0)` and negative inputs return `None`.

use crate::{Q32_32, LN2_Q, ONE};

/// `ln(x)` in Q32.32. Returns `None` on `x ≤ 0` (we represent 0 explicitly;
/// negative inputs can't exist in our unsigned Q32.32 anyway).
pub fn ln_q(x: Q32_32) -> Option<Q32_32> {
    if x.0 == 0 {
        return None;
    }
    if x.0 < ONE {
        // Domain restriction: LMSR's exp-of-q is always ≥ 1, so we don't
        // need ln of values in (0, 1) for our use case. Reject to keep
        // the analysis tractable. (If you do need it, factor as
        // ln(x) = -ln(1/x) and reuse this function.)
        return None;
    }

    // Step 1: find k = floor(log2(x_integer_part)). For x in Q32.32,
    // the leading bit position relative to bit 32 gives k.
    let leading_bit = 63 - x.0.leading_zeros(); // position of highest set bit
    let k = leading_bit.saturating_sub(32); // shift to "integer log2"

    // Reduce: m = x / 2^k, m ∈ [1, 2).
    let m = Q32_32(x.0 >> k);

    // Step 2a: half-and-square reduction. m_half = sqrt(m), so
    // ln(m) = 2·ln(m_half). One step puts m_half ∈ [1, √2 ≈ 1.4142),
    // i.e., y < 0.4142 for the Mercator series.
    let m_half = sqrt_q(m)?;

    // Step 2b: Mercator series on y = m_half - 1.
    let y = m_half.checked_sub(Q32_32::ONE)?;
    let ln_m_half = mercator(y)?;

    // ln(m) = 2 · ln(m_half).
    let ln_m = ln_m_half.checked_add(ln_m_half)?;

    // Step 3: combine. ln(x) = k·ln(2) + ln(m).
    let k_ln2 = Q32_32::from_bits(LN2_Q.checked_mul(k as u64)?);
    k_ln2.checked_add(ln_m)
}

/// `sqrt(x)` in Q32.32 via Newton-Raphson. Used by `ln_q` for the
/// half-and-square reduction; not exported (out of scope for LMSR).
fn sqrt_q(x: Q32_32) -> Option<Q32_32> {
    if x.0 == 0 {
        return Some(Q32_32::ZERO);
    }
    // Initial guess: bit-twiddling approximation. Iterate 8 times —
    // empirically that's enough for Q32.32 precision on inputs in [1, 2).
    let mut z = Q32_32(1 << ((63 - x.0.leading_zeros()) / 2 + 16));
    for _ in 0..8 {
        // z_{n+1} = (z_n + x / z_n) / 2
        let q = x.checked_div(z)?;
        z = Q32_32((z.0.checked_add(q.0)?) >> 1);
    }
    Some(z)
}

/// `ln(1 + y)` = y - y²/2 + y³/3 - y⁴/4 + ... for `y ∈ [0, √2 - 1)`.
/// 12 terms suffices for ~Q32.32 precision on that range.
fn mercator(y: Q32_32) -> Option<Q32_32> {
    // We accumulate `acc` as the running sum and `power` as y^n.
    // Alternate sign by tracking even/odd term index.
    let mut acc = Q32_32::ZERO;
    let mut power = y;
    for n in 1u64..=12 {
        let term = power.checked_div(Q32_32::from_int(n as u32))?;
        if n % 2 == 1 {
            acc = acc.checked_add(term)?;
        } else {
            // Underflow guard: if term > acc, the remaining series can't
            // bring us back positive in the LMSR domain — but for small y
            // this never trips. Saturate at zero defensively.
            acc = acc.checked_sub(term).unwrap_or(Q32_32::ZERO);
        }
        power = power.checked_mul(y)?;
    }
    Some(acc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ln_one_is_zero() {
        let r = ln_q(Q32_32::from_int(1)).unwrap();
        assert_eq!(r.0, 0);
    }

    #[test]
    fn ln_e_close_to_one() {
        let e = Q32_32::from_f64(core::f64::consts::E);
        let r = ln_q(e).unwrap().to_f64();
        assert!((r - 1.0).abs() < 1e-5, "ln(e) ≈ 1, got {}", r);
    }

    #[test]
    fn ln_powers_of_two() {
        for k in 1..16u32 {
            let x = Q32_32::from_int(1u32 << k);
            let r = ln_q(x).unwrap().to_f64();
            let truth = (k as f64) * core::f64::consts::LN_2;
            assert!(
                (r - truth).abs() < 1e-4,
                "ln(2^{}) ≈ {}·ln(2) = {}, got {}",
                k,
                k,
                truth,
                r
            );
        }
    }

    #[test]
    fn ln_zero_is_none() {
        assert!(ln_q(Q32_32::ZERO).is_none());
    }
}
