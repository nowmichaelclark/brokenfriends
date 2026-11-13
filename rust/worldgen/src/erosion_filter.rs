// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// This file is a port of the "Advanced Terrain Erosion Filter" by
// Rune Skovbo Johansen (2025), https://www.shadertoy.com/view/wXcfWn,
// through the CPU code of the `bevy_erosion_filter` crate (MPL-2.0).
// Earlier work: Fewes, "Terrain Erosion Noise" (2023), and Clay John,
// "Eroded Terrain Noise" (2020). See THIRD-PARTY-NOTICES.md.
//
// Changes for Broken Friends: the cell hash has a seed, the maths
// functions are from `detmath` (same bits on all computers), cells with
// no weight are skipped, and the code uses no vector library.

//! The erosion filter: gullies and ridges without a simulation.
//!
//! The filter adds layers of stripes that go down the slope. Each layer
//! bends the slope for the next layer. The result looks like terrain
//! that water has cut. The filter is a pure function of the position,
//! thus each sample can run on a different thread.
//!
//! This file has the MPL-2.0 licence. The other files of the crate have
//! the MIT licence.

use crate::detmath;
use crate::noise::hash2;

const TAU: f32 = std::f32::consts::TAU;

/// The settings of the filter. Lengths are in filter units: the caller
/// divides world positions and heights by one reference length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErosionParams {
    /// The size of the largest gullies.
    pub scale: f32,
    /// The depth of the gullies, as a part of `scale`.
    pub strength: f32,
    /// How much each layer bends the slope for the next layer.
    pub gully_weight: f32,
    /// How fast small gullies appear on the sides of large gullies.
    pub detail: f32,
    /// Rounding: at ridges, at creases, for the input, and the growth
    /// for each layer.
    pub rounding: [f32; 4],
    /// Onset: how fast the effect starts with the slope. The values are
    /// for the input, for each layer, and two values for the ridge map.
    pub onset: [f32; 4],
    /// The slope that the filter assumes, and how much it uses this
    /// value in place of the real slope (0 to 1).
    pub assumed_slope: [f32; 2],
    /// The size of the noise cells in relation to the stripes.
    pub cell_scale: f32,
    /// How much the stripes keep full height where cells disagree.
    pub normalization: f32,
    /// The number of layers.
    pub octaves: u32,
    /// The frequency factor between two layers.
    pub lacunarity: f32,
    /// The strength factor between two layers.
    pub gain: f32,
}

impl Default for ErosionParams {
    fn default() -> Self {
        Self {
            scale: 0.15,
            strength: 0.22,
            gully_weight: 0.5,
            detail: 1.5,
            rounding: [0.1, 0.0, 0.1, 2.0],
            onset: [1.25, 1.25, 2.8, 1.5],
            assumed_slope: [0.7, 1.0],
            cell_scale: 0.7,
            normalization: 0.5,
            octaves: 5,
            lacunarity: 2.0,
            gain: 0.5,
        }
    }
}

/// The result of the filter for one point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ErosionResult {
    /// The change of the height.
    pub height: f32,
    /// The change of the slope on x and y.
    pub slope: (f32, f32),
    /// The largest height change that is possible.
    pub magnitude: f32,
    /// A map of ridges (positive) and creases (negative).
    pub ridge_map: f32,
}

#[inline]
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn clamp01(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

#[inline]
fn pow_inv(t: f32, power: f32) -> f32 {
    1.0 - detmath::pow(1.0 - clamp01(t), power)
}

#[inline]
fn ease_out(t: f32) -> f32 {
    let v = 1.0 - clamp01(t);
    1.0 - v * v
}

#[inline]
fn smooth_start(t: f32, smoothing: f32) -> f32 {
    if t >= smoothing {
        return t - 0.5 * smoothing;
    }
    0.5 * t * t / smoothing
}

#[inline]
fn sign(v: f32) -> f32 {
    if v > 0.0 {
        1.0
    } else if v < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Phacelle noise by Rune Skovbo Johansen: stripes with a direction,
/// blended between random cell points.
///
/// Returns the cosine wave, the sine wave, and the side direction
/// multiplied by `freq * TAU`.
fn phacelle(
    px: f32,
    py: f32,
    dir: (f32, f32),
    freq: f32,
    offset_cycles: f32,
    normalization: f32,
    seed: u32,
) -> (f32, f32, f32, f32) {
    let side = (-dir.1 * freq * TAU, dir.0 * freq * TAU);
    let offset = offset_cycles * TAU;

    let ix = px.floor();
    let iy = py.floor();
    let fx = px - ix;
    let fy = py - iy;
    let (cx, cy) = (ix as i32, iy as i32);

    let mut sum_c = 0.0;
    let mut sum_s = 0.0;
    let mut weight_sum = 0.0;
    for j in -1..=2 {
        for i in -1..=2 {
            let h = hash2(cx.wrapping_add(i), cy.wrapping_add(j), seed);
            // A random offset between -0.5 and 0.5 on each axis.
            let rx = (h & 0xffff) as f32 * (1.0 / 65_536.0) - 0.5;
            let ry = (h >> 16) as f32 * (1.0 / 65_536.0) - 0.5;
            let vx = fx - i as f32 - rx;
            let vy = fy - j as f32 - ry;
            let sqr_dist = vx * vx + vy * vy;
            // The weight is 0 at this distance. Do not calculate the wave.
            if sqr_dist >= 2.25 {
                continue;
            }
            let weight = (detmath::exp(-sqr_dist * 2.0) - 0.011_11).max(0.0);
            weight_sum += weight;
            let (s, c) = detmath::sin_cos(vx * side.0 + vy * side.1 + offset);
            sum_c += c * weight;
            sum_s += s * weight;
        }
    }
    if weight_sum <= 0.0 {
        return (0.0, 0.0, side.0, side.1);
    }
    let c = sum_c / weight_sum;
    let s = sum_s / weight_sum;
    let magnitude = (1.0 - normalization).max((c * c + s * s).sqrt());
    (c / magnitude, s / magnitude, side.0, side.1)
}

/// Runs the filter for one point.
///
/// `p` is the position. `slope` is the slope of the terrain before the
/// filter. `fade_target` is -1 in the valleys and 1 on the peaks.
pub fn erosion_filter(
    p: (f32, f32),
    slope: (f32, f32),
    fade_target: f32,
    params: &ErosionParams,
    seed: u32,
) -> ErosionResult {
    let mut strength = params.strength * params.scale;
    let mut fade_target = fade_target.clamp(-1.0, 1.0);

    let mut dh = 0.0;
    let mut ds = (0.0, 0.0);

    let mut freq = 1.0 / (params.scale * params.cell_scale);
    let slope_length = (slope.0 * slope.0 + slope.1 * slope.1).sqrt().max(1e-10);
    let mut magnitude = 0.0;
    let mut rounding_mult = 1.0;

    let rounding_for_input = mix(
        params.rounding[1],
        params.rounding[0],
        clamp01(fade_target + 0.5),
    ) * params.rounding[2];
    let mut combi_mask = ease_out(smooth_start(
        slope_length * params.onset[0],
        rounding_for_input * params.onset[0],
    ));

    let mut ridge_combi_mask = ease_out(slope_length * params.onset[2]);
    let mut ridge_fade_target = fade_target;

    let assumed = params.assumed_slope[0] / slope_length;
    let mut gully = (
        mix(slope.0, slope.0 * assumed, params.assumed_slope[1]),
        mix(slope.1, slope.1 * assumed, params.assumed_slope[1]),
    );

    for octave in 0..params.octaves {
        let len = (gully.0 * gully.0 + gully.1 * gully.1).sqrt();
        let dir = if len > 1e-10 {
            (gully.0 / len, gully.1 / len)
        } else {
            gully
        };
        let (wave_c, wave_s, side_x, side_y) = phacelle(
            p.0 * freq,
            p.1 * freq,
            dir,
            params.cell_scale,
            0.25,
            params.normalization,
            seed.wrapping_add(octave.wrapping_mul(0x9E37_79B9)),
        );
        let zx = side_x * -freq;
        let zy = side_y * -freq;
        let sloping = wave_s.abs();

        let bend = sign(wave_s) * strength * params.gully_weight;
        gully.0 += bend * zx;
        gully.1 += bend * zy;

        let octave_h = wave_c;
        let octave_s = (wave_s * zx, wave_s * zy);
        let faded_h = mix(fade_target, octave_h * params.gully_weight, combi_mask);
        let faded_s = (
            octave_s.0 * params.gully_weight * combi_mask,
            octave_s.1 * params.gully_weight * combi_mask,
        );
        dh += faded_h * strength;
        ds.0 += faded_s.0 * strength;
        ds.1 += faded_s.1 * strength;
        magnitude += strength;

        fade_target = faded_h;

        let rounding_for_octave = mix(
            params.rounding[1],
            params.rounding[0],
            clamp01(wave_c + 0.5),
        ) * rounding_mult;
        let new_mask = ease_out(smooth_start(
            sloping * params.onset[1],
            rounding_for_octave * params.onset[1],
        ));
        combi_mask = pow_inv(combi_mask, params.detail) * new_mask;

        ridge_fade_target = mix(ridge_fade_target, octave_h, ridge_combi_mask);
        ridge_combi_mask *= ease_out(sloping * params.onset[3]);

        strength *= params.gain;
        freq *= params.lacunarity;
        rounding_mult *= params.rounding[3];
    }

    ErosionResult {
        height: dh,
        slope: ds,
        magnitude,
        ridge_map: ridge_fade_target * (1.0 - ridge_combi_mask),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_is_finite_and_inside_the_magnitude() {
        let params = ErosionParams::default();
        for i in 0..2000 {
            let p = (i as f32 * 0.0173, 1.0 - i as f32 * 0.0091);
            let slope = ((i % 13) as f32 * 0.1 - 0.6, (i % 7) as f32 * 0.2 - 0.6);
            let r = erosion_filter(p, slope, (i % 5) as f32 * 0.5 - 1.0, &params, 3);
            assert!(r.height.is_finite() && r.ridge_map.is_finite());
            assert!(
                r.height.abs() <= r.magnitude * 1.001,
                "{} > {}",
                r.height,
                r.magnitude
            );
        }
    }

    #[test]
    fn magnitude_is_the_sum_of_the_layer_strengths() {
        let params = ErosionParams::default();
        let r = erosion_filter((0.42, 0.31), (0.1, -0.2), 0.0, &params, 1);
        let mut expected = 0.0;
        let mut strength = params.strength * params.scale;
        for _ in 0..params.octaves {
            expected += strength;
            strength *= params.gain;
        }
        assert!((r.magnitude - expected).abs() < 1e-6);
    }

    #[test]
    fn flat_input_is_finite_and_seed_changes_the_result() {
        let params = ErosionParams::default();
        let flat = erosion_filter((0.25, 0.75), (0.0, 0.0), 0.0, &params, 1);
        assert!(flat.height.is_finite());
        let a = erosion_filter((0.25, 0.75), (0.3, 0.2), 0.0, &params, 1);
        let b = erosion_filter((0.25, 0.75), (0.3, 0.2), 0.0, &params, 2);
        assert_ne!(a.height, b.height);
    }
}
