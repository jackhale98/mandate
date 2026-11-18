//! Deterministic math (DEC-041 rule 4). All transcendental functions go through
//! `libm` so native and WASM builds produce bit-identical results.
//! `sqrt` and basic arithmetic are IEEE-754 correctly rounded on all targets.

#[inline]
pub fn exp(x: f64) -> f64 {
    libm::exp(x)
}
#[inline]
pub fn ln(x: f64) -> f64 {
    libm::log(x)
}
#[inline]
pub fn pow(x: f64, y: f64) -> f64 {
    libm::pow(x, y)
}
#[inline]
pub fn sin(x: f64) -> f64 {
    libm::sin(x)
}
#[inline]
pub fn cos(x: f64) -> f64 {
    libm::cos(x)
}
#[inline]
pub fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}
#[inline]
pub fn log10(x: f64) -> f64 {
    libm::log10(x)
}
#[inline]
pub fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}
#[inline]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}
#[inline]
pub fn floor(x: f64) -> f64 {
    libm::floor(x)
}

/// Round half away from zero (portable; avoids `f64::round` semantics drift).
#[inline]
pub fn round_half_away(x: f64) -> f64 {
    if x >= 0.0 {
        libm::floor(x + 0.5)
    } else {
        -libm::floor(-x + 0.5)
    }
}

/// Logistic sigmoid.
#[inline]
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + exp(-x))
}

/// Clamp to `[0, 1]`, mapping NaN to 0 (NaN guard at system boundaries).
#[inline]
pub fn clamp01(x: f64) -> f64 {
    if x.is_nan() { 0.0 } else { x.clamp(0.0, 1.0) }
}

/// Clamp to `[lo, hi]`, mapping NaN to `lo`.
#[inline]
pub fn clamp(x: f64, lo: f64, hi: f64) -> f64 {
    if x.is_nan() { lo } else { x.clamp(lo, hi) }
}

#[inline]
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Numerically stable softmax over utilities, written into `out`.
pub fn softmax(utilities: &[f64], out: &mut Vec<f64>) {
    out.clear();
    if utilities.is_empty() {
        return;
    }
    let max = utilities.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut sum = 0.0;
    for &u in utilities {
        let e = exp(u - max);
        out.push(e);
        sum += e;
    }
    if sum > 0.0 {
        for v in out.iter_mut() {
            *v /= sum;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn softmax_sums_to_one() {
        let mut out = Vec::new();
        softmax(&[1.0, 2.0, 3.0, -50.0], &mut out);
        let s: f64 = out.iter().sum();
        assert!((s - 1.0).abs() < 1e-12);
        assert!(out[2] > out[1] && out[1] > out[0]);
    }

    #[test]
    fn rounding() {
        assert_eq!(round_half_away(2.5), 3.0);
        assert_eq!(round_half_away(-2.5), -3.0);
        assert_eq!(round_half_away(2.4999), 2.0);
        assert_eq!(clamp01(f64::NAN), 0.0);
    }
}
