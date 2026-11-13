//! Maths functions that give the same bits on all computers.
//!
//! The standard `sin`, `cos`, `exp` and `powf` use the maths library of
//! the operating system. Their last bits can be different on Linux and
//! on Windows. The functions here use only `+`, `-`, `*`, `/` and `sqrt`,
//! which are exact by the IEEE 754 standard. Thus the same seed makes
//! the same world on all computers.

use std::f64::consts::{FRAC_PI_2, LN_2};

/// Gives the sine and the cosine of an angle in radians.
/// The error is less than 1e-6 for angles between -1e5 and 1e5.
#[inline]
pub fn sin_cos(x: f32) -> (f32, f32) {
    let x = f64::from(x);
    let k = (x * (1.0 / FRAC_PI_2)).round();
    let r = x - k * FRAC_PI_2;
    let r2 = r * r;
    // Taylor series on -pi/4 ..= pi/4.
    let s = r
        * (1.0
            + r2 * (-1.0 / 6.0
                + r2 * (1.0 / 120.0
                    + r2 * (-1.0 / 5040.0 + r2 * (1.0 / 362_880.0 + r2 * (-1.0 / 39_916_800.0))))));
    let c = 1.0
        + r2 * (-0.5
            + r2 * (1.0 / 24.0
                + r2 * (-1.0 / 720.0 + r2 * (1.0 / 40_320.0 + r2 * (-1.0 / 3_628_800.0)))));
    let (s, c) = match (k as i64).rem_euclid(4) {
        0 => (s, c),
        1 => (c, -s),
        2 => (-s, -c),
        _ => (-c, s),
    };
    (s as f32, c as f32)
}

/// Gives `e` to the power `x`.
#[inline]
pub fn exp(x: f32) -> f32 {
    let x = f64::from(x).clamp(-700.0, 700.0);
    let k = (x * (1.0 / LN_2)).round();
    let r = x - k * LN_2;
    // Taylor series on -0.35 ..= 0.35.
    let p = 1.0
        + r * (1.0
            + r * (0.5
                + r * (1.0 / 6.0
                    + r * (1.0 / 24.0
                        + r * (1.0 / 120.0
                            + r * (1.0 / 720.0 + r * (1.0 / 5040.0 + r * (1.0 / 40_320.0))))))));
    // 2^k from the bits of an f64.
    let two_k = f64::from_bits(((k as i64 + 1023) as u64) << 52);
    (p * two_k) as f32
}

/// Gives the natural logarithm of `x`. `x` must be more than 0.
#[inline]
pub fn ln(x: f32) -> f32 {
    let x = f64::from(x);
    let bits = x.to_bits();
    let mut e = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | (1023u64 << 52));
    if m > std::f64::consts::SQRT_2 {
        m *= 0.5;
        e += 1;
    }
    // ln(m) = 2 atanh(s), with s = (m - 1) / (m + 1) and |s| < 0.172.
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let series = 1.0
        + s2 * (1.0 / 3.0
            + s2 * (1.0 / 5.0 + s2 * (1.0 / 7.0 + s2 * (1.0 / 9.0 + s2 * (1.0 / 11.0)))));
    (2.0 * s * series + e as f64 * LN_2) as f32
}

/// Gives `x` to the power `p`. If `x` is 0 or less, the result is 0.
#[inline]
pub fn pow(x: f32, p: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    exp(p * ln(x))
}

/// Gives a smooth step from 0 (at `a`) to 1 (at `b`).
#[inline]
pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sin_cos_agrees_with_std() {
        for i in -4000..4000 {
            let x = i as f32 * 0.0137;
            let (s, c) = sin_cos(x);
            assert!((s - x.sin()).abs() < 2e-6, "sin {x}: {s} vs {}", x.sin());
            assert!((c - x.cos()).abs() < 2e-6, "cos {x}: {c} vs {}", x.cos());
        }
    }

    #[test]
    fn exp_ln_pow_agree_with_std() {
        for i in -400..100 {
            let x = i as f32 * 0.05;
            let e = exp(x);
            assert!((e - x.exp()).abs() <= x.exp() * 1e-6, "exp {x}");
        }
        for i in 1..2000 {
            let x = i as f32 * 0.013;
            assert!(
                (ln(x) - x.ln()).abs() < 2e-6,
                "ln {x}: {} vs {}",
                ln(x),
                x.ln()
            );
            let p = pow(x, 1.5);
            assert!((p - x.powf(1.5)).abs() <= x.powf(1.5) * 1e-5, "pow {x}");
        }
        assert_eq!(pow(0.0, 1.5), 0.0);
    }
}
