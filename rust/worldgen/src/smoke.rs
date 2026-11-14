//! The shape of the smoke wall at the edge of a zone (R14).
//!
//! A wall of smoke surrounds the zone. Its inner face is the "front". The
//! front is a radius from the centre of the zone for each angle. The
//! seed and the time set the front, the height of the wall, the width of
//! its band and its density. The result moves in and out slowly.
//!
//! The shape is a sum of a few waves around the circle. Each wave has a
//! whole number of lobes, thus the wall closes with no seam. This module
//! does not use Godot. The same seed and the same time give the same
//! shape on all computers.

use crate::detmath::sin_cos;
use crate::noise::{hash01, hash2};
use std::f32::consts::TAU;

/// The gain that makes the sum of the waves fill the range from -1 to 1.
const FIELD_GAIN: f32 = 1.6;
/// The number of waves in each field.
const WAVES: usize = 5;
/// The lobes of each wave around the circle. Low numbers make large
/// bays and headlands. High numbers make small ones.
const LOBES: [f32; WAVES] = [2.0, 3.0, 5.0, 8.0, 13.0];

/// The values that the designer can change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmokeParams {
    /// The mean radius of the front, as a part of the half size of the zone.
    pub front_fraction: f32,
    /// How far the front moves from its mean, as a part of the half size.
    pub swing_fraction: f32,
    /// The closest that the front can come to the border of the zone, in metres.
    pub border_margin_m: f32,
    /// The closest that the front can come to the centre of the zone, in metres.
    pub min_front_m: f32,
    /// The lowest height of the wall above the ground, in metres.
    pub height_min_m: f32,
    /// The highest height of the wall above the ground, in metres.
    pub height_max_m: f32,
    /// The smallest width of the band of smoke behind the front, in metres.
    pub band_min_m: f32,
    /// The largest width of the band, in metres.
    pub band_max_m: f32,
    /// The most that one wave moves the front, in metres per second. A
    /// bear walks at 1.4 m/s, thus the wall must be slower than that.
    pub front_speed_mps: f32,
    /// The time that a wave of the height, the band and the density needs
    /// for one cycle, in seconds. It changes the look only.
    pub period_s: f32,
}

impl Default for SmokeParams {
    fn default() -> Self {
        Self {
            front_fraction: 0.74,
            swing_fraction: 0.17,
            border_margin_m: 40.0,
            min_front_m: 320.0,
            height_min_m: 60.0,
            height_max_m: 320.0,
            band_min_m: 80.0,
            band_max_m: 260.0,
            front_speed_mps: 0.35,
            period_s: 200.0,
        }
    }
}

/// The shape of the wall at one angle and one time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSample {
    /// The radius of the inner face of the wall from the centre, in metres.
    pub front_m: f32,
    /// The height of the wall above the local ground, in metres.
    pub height_m: f32,
    /// The width of the band of smoke behind the front, in metres.
    pub band_m: f32,
    /// How thick the smoke is, from 0.4 (thin) to 1 (thick).
    pub density: f32,
}

/// One wave: `sin(lobes * angle + phase + speed * time)`.
#[derive(Clone, Copy, Debug)]
struct Wave {
    lobes: f32,
    phase: f32,
    speed: f32,
    weight: f32,
}

/// The shape of the wall for one seed.
#[derive(Clone, Debug)]
pub struct SmokeEdge {
    half_size_m: f32,
    params: SmokeParams,
    /// The fields: 0 radius, 1 height, 2 band, 3 density.
    waves: [[Wave; WAVES]; 4],
}

impl SmokeEdge {
    /// Makes the wall for a seed. `half_size_m` is half the side of the zone.
    pub fn new(seed: i64, half_size_m: f32, params: SmokeParams) -> Self {
        let s = (seed as u64 ^ ((seed as u64) >> 32)) as u32 ^ 0x5B0C_E000;
        let mut waves = [[Wave {
            lobes: 0.0,
            phase: 0.0,
            speed: 0.0,
            weight: 0.0,
        }; WAVES]; 4];
        let swing_m = params.swing_fraction * half_size_m;
        let slow = TAU / params.period_s.max(1.0);
        for (field, row) in waves.iter_mut().enumerate() {
            let mut total = 0.0;
            for (i, wave) in row.iter_mut().enumerate() {
                let key = (field * WAVES + i) as i32;
                wave.lobes = LOBES[i];
                wave.phase = hash01(key, 2, s) * TAU;
                wave.weight = 1.0 / wave.lobes.sqrt();
                total += wave.weight;
            }
            for (i, wave) in row.iter_mut().enumerate() {
                let key = (field * WAVES + i) as i32;
                wave.weight /= total;
                // Half of the waves turn one way, and half the other way.
                let direction = if hash2(key, 1, s) & 1 == 0 { 1.0 } else { -1.0 };
                let mix = 0.4 + 0.6 * hash01(key, 3, s);
                wave.speed = if field == 0 {
                    // The front is a danger for the bear. Each wave moves it at
                    // `front_speed_mps` at the most, so the bear can walk away.
                    let amplitude_m = (swing_m * FIELD_GAIN * wave.weight).max(1.0);
                    direction * mix * params.front_speed_mps / amplitude_m
                } else {
                    direction * mix * slow
                };
            }
        }
        Self {
            half_size_m,
            params,
            waves,
        }
    }

    /// The half size of the zone that this wall surrounds, in metres.
    pub fn half_size_m(&self) -> f32 {
        self.half_size_m
    }

    /// Gives one field at an angle and a time, from -1 to 1. The sum has
    /// a small range in practice, so it is made stronger and clamped.
    fn field(&self, index: usize, angle: f32, t: f32) -> f32 {
        let mut sum = 0.0;
        for wave in &self.waves[index] {
            let (s, _) = sin_cos(wave.lobes * angle + wave.phase + wave.speed * t);
            sum += wave.weight * s;
        }
        (sum * FIELD_GAIN).clamp(-1.0, 1.0)
    }

    /// Gives the shape of the wall at an angle (radians) and a time (seconds).
    pub fn sample(&self, angle: f32, t: f32) -> EdgeSample {
        let p = &self.params;
        let swing = p.swing_fraction * self.half_size_m;
        let mean = p.front_fraction * self.half_size_m;
        let farthest = (self.half_size_m - p.border_margin_m).max(p.min_front_m);
        let front_m = (mean + swing * self.field(0, angle, t)).clamp(p.min_front_m, farthest);
        let lerp = |a: f32, b: f32, f: f32| a + (b - a) * (f * 0.5 + 0.5);
        EdgeSample {
            front_m,
            height_m: lerp(p.height_min_m, p.height_max_m, self.field(1, angle, t)),
            band_m: lerp(p.band_min_m, p.band_max_m, self.field(2, angle, t)),
            density: lerp(0.4, 1.0, self.field(3, angle, t)),
        }
    }

    /// Gives how far a point is inside the smoke, in metres along the
    /// line from the centre. A point in front of the wall gives a
    /// negative number. `angle` is the angle of the point around the centre.
    pub fn depth_m(&self, x: f32, z: f32, t: f32) -> f32 {
        let r = (x * x + z * z).sqrt();
        let sample = self.sample(z.atan2(x), t);
        r - sample.front_m
    }
}

/// Makes a square grey noise image that repeats at its edges. Each byte
/// is one pixel. The image is for the smoke: puffs and wisps from three
/// layers of soft noise. The same seed gives the same image.
pub fn tile_noise(size: usize, seed: i64) -> Vec<u8> {
    let s = (seed as u64 ^ ((seed as u64) >> 32)) as u32 ^ 0x7A11_0E00;
    // The cells of each layer. Each is a power of two, so the lattice repeats.
    let layers: [(usize, f32); 3] = [(4, 0.55), (8, 0.3), (16, 0.15)];
    let mut out = vec![0u8; size * size];
    for y in 0..size {
        for x in 0..size {
            let mut value = 0.0;
            for (layer, &(cells, amount)) in layers.iter().enumerate() {
                let fx = x as f32 / size as f32 * cells as f32;
                let fy = y as f32 / size as f32 * cells as f32;
                let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
                let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
                let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
                let cell = |cx: i32, cy: i32| {
                    let n = cells as i32;
                    hash01(
                        cx.rem_euclid(n),
                        cy.rem_euclid(n),
                        s.wrapping_add(layer as u32),
                    )
                };
                let top = cell(x0, y0) * (1.0 - tx) + cell(x0 + 1, y0) * tx;
                let bottom = cell(x0, y0 + 1) * (1.0 - tx) + cell(x0 + 1, y0 + 1) * tx;
                value += amount * (top * (1.0 - ty) + bottom * ty);
            }
            out[y * size + x] = (value.clamp(0.0, 1.0) * 255.0).round() as u8;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(seed: i64) -> SmokeEdge {
        SmokeEdge::new(seed, 1024.0, SmokeParams::default())
    }

    #[test]
    fn same_seed_same_shape() {
        let (a, b) = (edge(7), edge(7));
        for i in 0..100 {
            let angle = i as f32 * 0.063;
            let t = i as f32 * 3.7;
            assert_eq!(a.sample(angle, t), b.sample(angle, t));
        }
    }

    #[test]
    fn different_seeds_make_different_shapes() {
        let (a, b) = (edge(1), edge(2));
        let different = (0..64)
            .filter(|i| {
                let angle = *i as f32 * 0.1;
                (a.sample(angle, 10.0).front_m - b.sample(angle, 10.0).front_m).abs() > 1.0
            })
            .count();
        assert!(different > 32, "only {different} of 64 angles differ");
    }

    #[test]
    fn the_wall_closes_with_no_seam() {
        let e = edge(3);
        for t in [0.0, 40.0, 500.0] {
            let a = e.sample(0.0, t);
            let b = e.sample(TAU, t);
            assert!((a.front_m - b.front_m).abs() < 0.01);
            assert!((a.height_m - b.height_m).abs() < 0.01);
        }
    }

    #[test]
    fn the_front_stays_between_the_centre_and_the_border() {
        for seed in 0..20 {
            let e = edge(seed);
            for i in 0..200 {
                for t in [0.0, 17.0, 333.0, 5000.0] {
                    let s = e.sample(i as f32 * 0.0314, t);
                    assert!(s.front_m >= 320.0 && s.front_m <= 984.0, "{s:?}");
                    assert!(s.height_m >= 60.0 && s.height_m <= 320.0, "{s:?}");
                    assert!(s.band_m >= 80.0 && s.band_m <= 260.0, "{s:?}");
                    assert!((0.4..=1.0).contains(&s.density), "{s:?}");
                }
            }
        }
    }

    #[test]
    fn the_start_area_is_always_clear() {
        // The bear starts within 200 m of the centre.
        for seed in 0..50 {
            let e = edge(seed);
            for i in 0..100 {
                for t in [0.0, 60.0, 900.0] {
                    assert!(e.depth_m(200.0, 0.0, t) < 0.0);
                    assert!(e.sample(i as f32 * 0.0628, t).front_m > 300.0);
                }
            }
        }
    }

    #[test]
    fn the_front_moves_with_time() {
        let e = edge(5);
        let a = e.sample(1.0, 0.0).front_m;
        let moved = (1..=60).any(|k| (e.sample(1.0, k as f32 * 10.0).front_m - a).abs() > 20.0);
        assert!(moved, "the front did not move in 10 minutes");
    }

    #[test]
    fn the_front_is_slower_than_a_walking_bear() {
        // Over many seeds, the front never moves faster than 1.4 m/s
        // (a walk), and it is usually much slower.
        let mut fastest = 0.0_f32;
        let mut total = 0.0;
        let mut count = 0;
        for seed in 0..30 {
            let e = edge(seed);
            for i in 0..64 {
                let angle = i as f32 * TAU / 64.0;
                for k in 0..40 {
                    let t = k as f32 * 7.0;
                    let v = (e.sample(angle, t + 0.5).front_m - e.sample(angle, t).front_m) / 0.5;
                    fastest = fastest.max(v.abs());
                    total += v.abs();
                    count += 1;
                }
            }
        }
        let mean = total / count as f32;
        assert!(fastest < 1.4, "fastest {fastest} m/s");
        assert!(mean < 0.5, "mean {mean} m/s");
    }

    #[test]
    fn the_wall_has_real_bays_and_headlands() {
        // Over all angles the front covers a wide range, so it is not a circle.
        let e = edge(11);
        let radii: Vec<f32> = (0..256)
            .map(|i| e.sample(i as f32 * TAU / 256.0, 30.0).front_m)
            .collect();
        let min = radii.iter().cloned().fold(f32::MAX, f32::min);
        let max = radii.iter().cloned().fold(f32::MIN, f32::max);
        assert!(max - min > 60.0, "range {min}..{max}");
    }

    #[test]
    fn depth_is_zero_on_the_front() {
        let e = edge(9);
        let angle = 2.0_f32;
        let front = e.sample(angle, 12.0).front_m;
        let (x, z) = (angle.cos() * front, angle.sin() * front);
        assert!(e.depth_m(x, z, 12.0).abs() < 0.01);
        assert!(e.depth_m(x * 1.05, z * 1.05, 12.0) > 1.0);
        assert!(e.depth_m(x * 0.95, z * 0.95, 12.0) < -1.0);
    }

    #[test]
    fn the_noise_image_repeats_and_is_stable() {
        let a = tile_noise(64, 4);
        assert_eq!(a, tile_noise(64, 4));
        assert_ne!(a, tile_noise(64, 5));
        // The step between the last and the first column is as small as
        // between two neighbours: the image repeats at its edges.
        let step = |x0: usize, x1: usize| -> f32 {
            (0..64)
                .map(|y| (a[y * 64 + x0] as f32 - a[y * 64 + x1] as f32).abs())
                .sum::<f32>()
                / 64.0
        };
        assert!(step(63, 0) < 12.0, "seam step {}", step(63, 0));
        assert!(step(31, 32) < 12.0);
        let mean = a.iter().map(|&v| v as f32).sum::<f32>() / a.len() as f32;
        assert!((90.0..170.0).contains(&mean), "mean {mean}");
    }
}
