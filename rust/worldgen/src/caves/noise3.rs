//! Seeded 3D gradient noise for the cave walls.
//!
//! The noise uses only `+`, `-`, `*`, `floor` and a table of gradients.
//! Thus it gives the same bits on all computers.

use crate::noise::hash2;

/// Mixes three grid coordinates and a seed into 32 random bits.
#[inline]
pub fn hash3(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    hash2(
        x,
        y,
        seed ^ (z as u32).wrapping_mul(0x27D4_EB2F).rotate_left(7),
    )
}

/// The 12 edge directions of a cube, and 4 of them again. This is the
/// gradient set of Ken Perlin's improved noise.
const GRADIENTS: [[f32; 3]; 16] = [
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [1.0, -1.0, 0.0],
    [-1.0, -1.0, 0.0],
    [1.0, 0.0, 1.0],
    [-1.0, 0.0, 1.0],
    [1.0, 0.0, -1.0],
    [-1.0, 0.0, -1.0],
    [0.0, 1.0, 1.0],
    [0.0, -1.0, 1.0],
    [0.0, 1.0, -1.0],
    [0.0, -1.0, -1.0],
    [1.0, 1.0, 0.0],
    [-1.0, 1.0, 0.0],
    [0.0, -1.0, 1.0],
    [0.0, -1.0, -1.0],
];

#[inline]
fn corner(ix: i32, iy: i32, iz: i32, fx: f32, fy: f32, fz: f32, seed: u32) -> f32 {
    let g = GRADIENTS[(hash3(ix, iy, iz, seed) >> 28) as usize];
    g[0] * fx + g[1] * fy + g[2] * fz
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Gives 3D gradient noise at a point. The value is from -1 to 1.
pub fn noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xf, yf, zf) = (x.floor(), y.floor(), z.floor());
    let (ix, iy, iz) = (xf as i32, yf as i32, zf as i32);
    let (fx, fy, fz) = (x - xf, y - yf, z - zf);
    let (u, v, w) = (fade(fx), fade(fy), fade(fz));
    let (jx, jy, jz) = (ix.wrapping_add(1), iy.wrapping_add(1), iz.wrapping_add(1));
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let x00 = lerp(
        corner(ix, iy, iz, fx, fy, fz, seed),
        corner(jx, iy, iz, fx - 1.0, fy, fz, seed),
        u,
    );
    let x10 = lerp(
        corner(ix, jy, iz, fx, fy - 1.0, fz, seed),
        corner(jx, jy, iz, fx - 1.0, fy - 1.0, fz, seed),
        u,
    );
    let x01 = lerp(
        corner(ix, iy, jz, fx, fy, fz - 1.0, seed),
        corner(jx, iy, jz, fx - 1.0, fy, fz - 1.0, seed),
        u,
    );
    let x11 = lerp(
        corner(ix, jy, jz, fx, fy - 1.0, fz - 1.0, seed),
        corner(jx, jy, jz, fx - 1.0, fy - 1.0, fz - 1.0, seed),
        u,
    );
    let value = lerp(lerp(x00, x10, v), lerp(x01, x11, v), w);
    value.clamp(-1.0, 1.0)
}

/// Adds `octaves` layers of 3D noise. Each layer has about twice the
/// frequency and half the height of the layer before it. The value is
/// from -1 to 1.
pub fn fbm3(x: f32, y: f32, z: f32, seed: u32, octaves: u32) -> f32 {
    let mut sum = 0.0;
    let mut norm = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    for i in 0..octaves {
        let s = seed.wrapping_add(i.wrapping_mul(0x9E37_79B9));
        sum += noise3(x * freq, y * freq, z * freq, s) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    if norm > 0.0 {
        // The sum of a few layers rarely comes near the limit, so the
        // value is made 1.4 times stronger and clamped again.
        (sum / norm * 1.4).clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_is_smooth_bounded_and_seeded() {
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for i in 0..4000 {
            let p = (
                i as f32 * 0.173,
                i as f32 * 0.051 - 7.0,
                i as f32 * 0.029 + 3.0,
            );
            let n = fbm3(p.0, p.1, p.2, 9, 3);
            lo = lo.min(n);
            hi = hi.max(n);
            let near = fbm3(p.0 + 0.001, p.1, p.2, 9, 3);
            assert!((n - near).abs() < 0.02, "the noise must be smooth");
        }
        assert!(lo >= -1.0 && hi <= 1.0);
        assert!(hi - lo > 0.8, "the noise must use its range: {lo} to {hi}");
        assert_eq!(noise3(1.3, 2.6, 3.2, 4), noise3(1.3, 2.6, 3.2, 4));
        assert_ne!(noise3(1.3, 2.6, 3.2, 4), noise3(1.3, 2.6, 3.2, 5));
        assert_eq!(noise3(2.0, 3.0, 4.0, 4), 0.0, "zero at grid points");
    }
}
