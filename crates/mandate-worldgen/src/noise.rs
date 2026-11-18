//! Deterministic 2-D gradient noise (Perlin-style) seeded from the worldgen
//! RNG. Uses only arithmetic and `libm`-backed helpers (DEC-041).

use mandate_core::SimRng;
use mandate_core::math::{cos, floor, sin};

pub struct Noise2 {
    perm: [u16; 512],
    grads: [(f64, f64); 256],
}

impl Noise2 {
    pub fn new(rng: &mut SimRng) -> Self {
        let mut p: Vec<u16> = (0..256).collect();
        rng.shuffle(&mut p);
        let mut perm = [0u16; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        let mut grads = [(0.0, 0.0); 256];
        for g in grads.iter_mut() {
            let a = rng.next_f64() * std::f64::consts::TAU;
            *g = (cos(a), sin(a));
        }
        Noise2 { perm, grads }
    }

    #[inline]
    fn grad(&self, ix: i64, iy: i64, x: f64, y: f64) -> f64 {
        let h =
            self.perm[(self.perm[(ix & 255) as usize] as i64 + (iy & 255)) as usize & 511] as usize;
        let (gx, gy) = self.grads[h & 255];
        gx * x + gy * y
    }

    /// Gradient noise in roughly `[-1, 1]`.
    pub fn get(&self, x: f64, y: f64) -> f64 {
        let x0 = floor(x);
        let y0 = floor(y);
        let fx = x - x0;
        let fy = y - y0;
        let (ix, iy) = (x0 as i64, y0 as i64);
        let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let u = fade(fx);
        let v = fade(fy);
        let n00 = self.grad(ix, iy, fx, fy);
        let n10 = self.grad(ix + 1, iy, fx - 1.0, fy);
        let n01 = self.grad(ix, iy + 1, fx, fy - 1.0);
        let n11 = self.grad(ix + 1, iy + 1, fx - 1.0, fy - 1.0);
        let a = n00 + u * (n10 - n00);
        let b = n01 + u * (n11 - n01);
        (a + v * (b - a)) * 1.414
    }

    /// Fractal Brownian motion, normalised to roughly `[-1, 1]`.
    pub fn fbm(&self, x: f64, y: f64, octaves: u32, lacunarity: f64, gain: f64) -> f64 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for o in 0..octaves {
            // Offset each octave to decorrelate lattice artefacts.
            let off = o as f64 * 17.31;
            sum += amp * self.get(x * freq + off, y * freq - off);
            norm += amp;
            amp *= gain;
            freq *= lacunarity;
        }
        sum / norm
    }

    /// Ridged multifractal in `[0, 1]` (sharp crests for mountain ranges).
    pub fn ridged(&self, x: f64, y: f64, octaves: u32) -> f64 {
        let mut sum = 0.0;
        let mut amp = 0.5;
        let mut freq = 1.0;
        let mut norm = 0.0;
        for o in 0..octaves {
            let off = o as f64 * 31.7;
            let n = 1.0 - self.get(x * freq + off, y * freq + off).abs();
            sum += amp * n * n;
            norm += amp;
            amp *= 0.5;
            freq *= 2.0;
        }
        sum / norm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mandate_core::{RngStreams, Stream};

    #[test]
    fn noise_is_deterministic_and_bounded() {
        let mut a = RngStreams::new(5);
        let mut b = RngStreams::new(5);
        let na = Noise2::new(a.get(Stream::Worldgen));
        let nb = Noise2::new(b.get(Stream::Worldgen));
        for i in 0..500 {
            let (x, y) = (i as f64 * 0.37, i as f64 * 0.11);
            let v = na.fbm(x, y, 5, 2.0, 0.5);
            assert_eq!(v.to_bits(), nb.fbm(x, y, 5, 2.0, 0.5).to_bits());
            assert!((-1.5..=1.5).contains(&v));
            assert!((0.0..=1.0).contains(&na.ridged(x, y, 4)));
        }
    }
}
