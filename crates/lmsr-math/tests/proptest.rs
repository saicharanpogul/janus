//! Property-based tests: every Q32.32 `exp` and `ln` call must agree
//! with the f64 reference to within a documented tolerance, across the
//! full input domain we care about for LMSR.

use janus_lmsr_math::{exp_q, ln_q, Q32_32};
use proptest::prelude::*;

/// Relative-error tolerance for fixed-point vs f64. 1e-4 = 0.01% — well
/// within what an LMSR pricing application needs for a 6-decimal token.
const REL_TOL: f64 = 1e-4;

fn assert_close(label: &str, fp: f64, truth: f64) {
    if truth.abs() < 1e-9 {
        // Avoid divide-by-zero; check absolute error.
        assert!(
            (fp - truth).abs() < 1e-6,
            "{}: fixed={} truth={} (abs err)",
            label,
            fp,
            truth
        );
    } else {
        let rel = ((fp - truth) / truth).abs();
        assert!(
            rel < REL_TOL,
            "{}: fixed={} truth={} rel_err={}",
            label,
            fp,
            truth,
            rel
        );
    }
}

proptest! {
    /// `exp_q(x)` agrees with `x.exp()` for x in [0, 20].
    #[test]
    fn exp_matches_f64(x in 0.0f64..20.0) {
        let q = Q32_32::from_f64(x);
        let r = exp_q(q).expect("no overflow in [0,20]");
        assert_close("exp", r.to_f64(), x.exp());
    }

    /// `ln_q(x)` agrees with `x.ln()` for x in [1, 2^16].
    #[test]
    fn ln_matches_f64(x in 1.0f64..65536.0) {
        let q = Q32_32::from_f64(x);
        let r = ln_q(q).expect("ln defined on [1, ∞)");
        assert_close("ln", r.to_f64(), x.ln());
    }

    /// `exp(ln(x)) ≈ x` round-trip for x in [1, 2^14].
    #[test]
    fn exp_of_ln_round_trip(x in 1.0f64..16384.0) {
        let q = Q32_32::from_f64(x);
        let lnx = ln_q(q).unwrap();
        let back = exp_q(lnx).unwrap();
        // Accumulated error from two transforms — relax tolerance a bit.
        let rel = (back.to_f64() - x).abs() / x;
        prop_assert!(rel < 5e-4, "exp(ln({})) = {} (rel err {})", x, back.to_f64(), rel);
    }

    /// `ln(exp(x)) ≈ x` round-trip for x in [0, 15].
    #[test]
    fn ln_of_exp_round_trip(x in 0.1f64..15.0) {
        let q = Q32_32::from_f64(x);
        let ex = exp_q(q).unwrap();
        let back = ln_q(ex).unwrap();
        let rel = (back.to_f64() - x).abs() / x;
        prop_assert!(rel < 5e-4, "ln(exp({})) = {} (rel err {})", x, back.to_f64(), rel);
    }
}
