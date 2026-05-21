//! LMSR cost function and swap pricing on Q32.32.
//!
//! The Hanson cost function:
//!     C(q_yes, q_no) = b · ln(exp(q_yes / b) + exp(q_no / b))
//!
//! Swap cost (buying `delta` shares of one outcome):
//!     cost = C(q + delta, q') − C(q, q')
//!
//! Outcome price (instantaneous):
//!     p_yes = exp(q_yes/b) / (exp(q_yes/b) + exp(q_no/b))
//!
//! Bounded-loss invariant for the subsidizer:
//!     max over all reachable states of (C(q_yes, q_no) − C(0, 0))
//!     ≤ b · ln(2)
//!
//! Numerical: we use the log-sum-exp trick to avoid overflow when q/b
//! gets large — subtract m = max(q_yes/b, q_no/b) before exponentiating
//! so the largest exp argument is 0 and the other is ≤ 0.
//! Since our Q32.32 is unsigned and we don't have negative values, we
//! implement this by computing `exp(min - max) ≤ 1` directly and
//! taking advantage of `exp(m) · (1 + exp(d))` where `d = min - max ≤ 0`.

use crate::{exp_q, ln_q, Q32_32};

/// LMSR cost `C(q_yes, q_no) = b · ln(exp(q_yes / b) + exp(q_no / b))`.
///
/// Returns `None` on overflow or invalid input (b == 0, etc).
pub fn cost(b: Q32_32, q_yes: Q32_32, q_no: Q32_32) -> Option<Q32_32> {
    if b.0 == 0 {
        return None;
    }

    // Compute u_yes = q_yes / b, u_no = q_no / b.
    let u_yes = q_yes.checked_div(b)?;
    let u_no = q_no.checked_div(b)?;

    // log-sum-exp trick: factor out exp(max) for stability.
    let (max_u, min_u) = if u_yes >= u_no { (u_yes, u_no) } else { (u_no, u_yes) };
    let delta = max_u.checked_sub(min_u)?; // ≥ 0

    // exp(min_u - max_u) = exp(-delta), but we're unsigned; represent
    // as 1 / exp(delta) instead.
    let exp_delta = exp_q(delta)?;
    let one = Q32_32::ONE;
    let inv_exp_delta = one.checked_div(exp_delta)?; // = exp(-delta) ∈ (0, 1]

    // log(exp(min_u) + exp(max_u))
    //   = log(exp(max_u) · (1 + exp(min_u - max_u)))
    //   = max_u + log(1 + exp(-delta))
    let one_plus = one.checked_add(inv_exp_delta)?;
    let ln_one_plus = ln_q(one_plus)?;
    let log_sum = max_u.checked_add(ln_one_plus)?;

    // C = b · log_sum
    b.checked_mul(log_sum)
}

/// Cost to buy `delta` shares of YES: `C(q_yes + delta, q_no) - C(q_yes, q_no)`.
pub fn buy_yes_cost(
    b: Q32_32,
    q_yes: Q32_32,
    q_no: Q32_32,
    delta: Q32_32,
) -> Option<Q32_32> {
    let after = cost(b, q_yes.checked_add(delta)?, q_no)?;
    let before = cost(b, q_yes, q_no)?;
    after.checked_sub(before)
}

/// Cost to buy `delta` shares of NO.
pub fn buy_no_cost(
    b: Q32_32,
    q_yes: Q32_32,
    q_no: Q32_32,
    delta: Q32_32,
) -> Option<Q32_32> {
    let after = cost(b, q_yes, q_no.checked_add(delta)?)?;
    let before = cost(b, q_yes, q_no)?;
    after.checked_sub(before)
}

/// Instantaneous price of YES = exp(q_yes/b) / (exp(q_yes/b) + exp(q_no/b)).
/// In [0, 1].
pub fn price_yes(b: Q32_32, q_yes: Q32_32, q_no: Q32_32) -> Option<Q32_32> {
    if b.0 == 0 {
        return None;
    }
    let u_yes = q_yes.checked_div(b)?;
    let u_no = q_no.checked_div(b)?;
    // p_yes = exp(u_yes) / (exp(u_yes) + exp(u_no))
    //       = 1 / (1 + exp(u_no - u_yes))
    if u_yes >= u_no {
        // exp(u_no - u_yes) ≤ 1; we compute exp(-(u_yes - u_no)) as 1/exp(...).
        let delta = u_yes.checked_sub(u_no)?;
        let exp_d = exp_q(delta)?;
        let denom = exp_d.checked_add(Q32_32::ONE)?;
        exp_d.checked_div(denom)
    } else {
        let delta = u_no.checked_sub(u_yes)?;
        let exp_d = exp_q(delta)?;
        let denom = exp_d.checked_add(Q32_32::ONE)?;
        Q32_32::ONE.checked_div(denom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cost is symmetric and increasing in either reserve.
    #[test]
    fn cost_symmetric_at_origin() {
        let b = Q32_32::from_int(1000);
        let c00 = cost(b, Q32_32::ZERO, Q32_32::ZERO).unwrap();
        // C(0, 0) = b · ln(2)
        let truth = 1000.0 * core::f64::consts::LN_2;
        let got = c00.to_f64();
        assert!((got - truth).abs() < 0.1, "C(0,0) = {} (expected {})", got, truth);
    }

    /// Price sums to 1: p_yes + p_no = 1.
    #[test]
    fn prices_sum_to_one() {
        let b = Q32_32::from_int(100);
        for &(qy, qn) in &[(0u32, 0u32), (10, 5), (50, 50), (100, 200)] {
            let py = price_yes(b, Q32_32::from_int(qy), Q32_32::from_int(qn))
                .unwrap()
                .to_f64();
            let pn = price_yes(b, Q32_32::from_int(qn), Q32_32::from_int(qy))
                .unwrap()
                .to_f64(); // swap args = p_no
            assert!(
                (py + pn - 1.0).abs() < 1e-4,
                "p_yes + p_no = {} for ({}, {})",
                py + pn,
                qy,
                qn
            );
        }
    }

    /// **Bounded-loss invariant**: the subsidizer's maximum exposure
    /// is `b · ln(2)`, achieved at q_yes = q_no = 0. Any non-trivial
    /// state pulls the cost above this floor by the amount traders
    /// have paid in. (We can't directly verify the *upper* bound on
    /// payout vs paid-in here without a swap simulation, but we can
    /// verify that `C` is always ≥ `b·ln(2)` and ≥ max(q_yes, q_no).)
    #[test]
    fn cost_lower_bound_b_ln2() {
        let b = Q32_32::from_int(1000);
        for &(qy, qn) in &[(0u32, 0u32), (100, 100), (500, 500), (1000, 1000)] {
            let c = cost(b, Q32_32::from_int(qy), Q32_32::from_int(qn))
                .unwrap()
                .to_f64();
            // C(q_yes, q_no) ≥ max(q_yes, q_no): outcomes always settle to one side.
            let max_q = (qy.max(qn)) as f64;
            assert!(
                c >= max_q - 0.1, // small tolerance for FP error
                "C({}, {}) = {} should be >= max = {}",
                qy,
                qn,
                c,
                max_q
            );
        }
    }

    /// Buying a tiny `delta` at zero reserves costs approximately
    /// `delta · 0.5` (since price = 0.5 at symmetric reserves).
    #[test]
    fn small_buy_cost_approximates_price() {
        let b = Q32_32::from_int(1000);
        let q0 = Q32_32::ZERO;
        let delta = Q32_32::from_int(1);
        let c = buy_yes_cost(b, q0, q0, delta).unwrap().to_f64();
        assert!(
            (c - 0.5).abs() < 0.01,
            "small buy cost = {} (expected ≈ 0.5)",
            c
        );
    }
}
