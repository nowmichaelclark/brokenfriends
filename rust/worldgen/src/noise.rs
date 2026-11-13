//! Seeded gradient noise with analytic derivatives.
//!
//! Gradient noise (Perlin noise) gives a smooth random value for each
//! point. The derivatives give the slope of the noise. The erosion code
//! uses the slope.

/// A noise value and its slope.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sample {
    /// The value, approximately in the range -1 to 1.
    pub value: f32,
    /// The change of the value for one unit on the x axis.
    pub dx: f32,
    /// The change of the value for one unit on the y axis.
    pub dy: f32,
}

/// Mixes two grid coordinates and a seed into 32 random bits.
#[inline]
pub fn hash2(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = seed
        ^ (x as u32).wrapping_mul(0x85EB_CA6B)
        ^ (y as u32).wrapping_mul(0xC2B2_AE35).rotate_left(13);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    h
}

/// Gives a random number from 0 to 1 for a grid point.
#[inline]
pub fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    (hash2(x, y, seed) >> 8) as f32 * (1.0 / 16_777_216.0)
}

/// 16 unit vectors, one for each 22.5 degrees. A table gives the same
/// bits on all computers. `sin` and `cos` do not.
const GRADIENTS: [(f32, f32); 16] = [
    (1.0, 0.0),
    (0.923_879_5, 0.382_683_43),
    (0.707_106_77, 0.707_106_77),
    (0.382_683_43, 0.923_879_5),
    (0.0, 1.0),
    (-0.382_683_43, 0.923_879_5),
    (-0.707_106_77, 0.707_106_77),
    (-0.923_879_5, 0.382_683_43),
    (-1.0, 0.0),
    (-0.923_879_5, -0.382_683_43),
    (-0.707_106_77, -0.707_106_77),
    (-0.382_683_43, -0.923_879_5),
    (0.0, -1.0),
    (0.382_683_43, -0.923_879_5),
    (0.707_106_77, -0.707_106_77),
    (0.923_879_5, -0.382_683_43),
];

#[inline]
fn gradient(x: i32, y: i32, seed: u32) -> (f32, f32) {
    GRADIENTS[(hash2(x, y, seed) >> 28) as usize]
}

/// Gives 2D gradient noise and its slope at a point.
pub fn gradient_noise(x: f32, y: f32, seed: u32) -> Sample {
    let xf = x.floor();
    let yf = y.floor();
    let (ix, iy) = (xf as i32, yf as i32);
    let (fx, fy) = (x - xf, y - yf);

    let g00 = gradient(ix, iy, seed);
    let g10 = gradient(ix.wrapping_add(1), iy, seed);
    let g01 = gradient(ix, iy.wrapping_add(1), seed);
    let g11 = gradient(ix.wrapping_add(1), iy.wrapping_add(1), seed);

    let a = g00.0 * fx + g00.1 * fy;
    let b = g10.0 * (fx - 1.0) + g10.1 * fy;
    let c = g01.0 * fx + g01.1 * (fy - 1.0);
    let d = g11.0 * (fx - 1.0) + g11.1 * (fy - 1.0);

    // Quintic fade curve and its derivative.
    let u = fx * fx * fx * (fx * (fx * 6.0 - 15.0) + 10.0);
    let v = fy * fy * fy * (fy * (fy * 6.0 - 15.0) + 10.0);
    let du = 30.0 * fx * fx * (fx * (fx - 2.0) + 1.0);
    let dv = 30.0 * fy * fy * (fy * (fy - 2.0) + 1.0);

    let k = a - b - c + d;
    let value = a + u * (b - a) + v * (c - a) + u * v * k;
    let dx = g00.0
        + u * (g10.0 - g00.0)
        + v * (g01.0 - g00.0)
        + u * v * (g00.0 - g10.0 - g01.0 + g11.0)
        + du * ((b - a) + v * k);
    let dy = g00.1
        + u * (g10.1 - g00.1)
        + v * (g01.1 - g00.1)
        + u * v * (g00.1 - g10.1 - g01.1 + g11.1)
        + dv * ((c - a) + u * k);

    // Raw gradient noise stays inside about +/-0.707. Scale it to +/-1.
    const SCALE: f32 = std::f32::consts::SQRT_2;
    Sample {
        value: value * SCALE,
        dx: dx * SCALE,
        dy: dy * SCALE,
    }
}

/// Adds layers (octaves) of gradient noise. This is fractal Brownian
/// motion (fBm). Each layer has `lacunarity` times the frequency and
/// `gain` times the height of the layer before it. The result is
/// approximately in the range -1 to 1.
pub fn fbm(x: f32, y: f32, seed: u32, octaves: u32, lacunarity: f32, gain: f32) -> Sample {
    let mut sum = Sample::default();
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for i in 0..octaves {
        let s = gradient_noise(
            x * freq,
            y * freq,
            seed.wrapping_add(i.wrapping_mul(0x9E37_79B9)),
        );
        sum.value += s.value * amp;
        sum.dx += s.dx * amp * freq;
        sum.dy += s.dy * amp * freq;
        norm += amp;
        amp *= gain;
        freq *= lacunarity;
    }
    if norm > 0.0 {
        sum.value /= norm;
        sum.dx /= norm;
        sum.dy /= norm;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_value() {
        assert_eq!(
            gradient_noise(12.34, -5.67, 7),
            gradient_noise(12.34, -5.67, 7)
        );
        assert_ne!(
            gradient_noise(12.34, -5.67, 7).value,
            gradient_noise(12.34, -5.67, 8).value
        );
    }

    #[test]
    fn value_is_zero_on_grid_points_and_in_range() {
        assert_eq!(gradient_noise(3.0, -4.0, 1).value, 0.0);
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..20_000 {
            let v = gradient_noise(i as f32 * 0.137, i as f32 * 0.071 - 300.0, 42).value;
            lo = lo.min(v);
            hi = hi.max(v);
        }
        assert!((-1.01..-0.5).contains(&lo), "lo = {lo}");
        assert!((0.5..1.01).contains(&hi), "hi = {hi}");
    }

    #[test]
    fn derivatives_agree_with_finite_differences() {
        let e = 1e-3;
        for i in 0..200 {
            let x = i as f32 * 0.173 - 11.0;
            let y = i as f32 * 0.291 + 3.0;
            for s in [gradient_noise as fn(f32, f32, u32) -> Sample, |x, y, s| {
                fbm(x, y, s, 4, 2.0, 0.5)
            }] {
                let c = s(x, y, 9);
                let ndx = (s(x + e, y, 9).value - s(x - e, y, 9).value) / (2.0 * e);
                let ndy = (s(x, y + e, 9).value - s(x, y - e, 9).value) / (2.0 * e);
                assert!(
                    (c.dx - ndx).abs() < 0.05,
                    "dx {} vs {} at {x},{y}",
                    c.dx,
                    ndx
                );
                assert!(
                    (c.dy - ndy).abs() < 0.05,
                    "dy {} vs {} at {x},{y}",
                    c.dy,
                    ndy
                );
            }
        }
    }

    #[test]
    fn hash01_is_in_range() {
        for i in -500..500 {
            let v = hash01(i, i * 7, 3);
            assert!((0.0..1.0).contains(&v));
        }
    }
}
