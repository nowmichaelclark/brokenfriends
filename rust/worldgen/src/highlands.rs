//! The scorched highlands: the first biome.
//!
//! The steps of the generator:
//! 1. Domain warp bends the positions, so the ranges are not straight.
//! 2. Ridged noise makes the mountain ranges.
//! 3. The erosion filter cuts gullies into the slopes.
//! 4. Fine noise makes the ground a little rough.
//! 5. The drainage step fills closed basins (dry lake beds) and cuts
//!    dry riverbeds where much rain came together.

use crate::detmath::smoothstep;
use crate::drainage::{flow, partial_fill};
use crate::erosion_filter::{erosion_filter, ErosionParams};
use crate::heightmap::HeightMap;
use crate::noise::{fbm, gradient_noise};
use rayon::prelude::*;

/// The settings of the scorched-highlands generator. Lengths are in
/// metres.
#[derive(Clone, Debug, PartialEq)]
pub struct HighlandsParams {
    /// The height of the base terrain, from the lowest to the highest
    /// point, before erosion.
    pub relief_m: f32,
    /// The distance between two mountain ranges.
    pub ridge_wavelength_m: f32,
    /// The number of noise layers in the ranges.
    pub ridge_octaves: u32,
    /// More than 1 makes wide valleys and thin ridges.
    pub ridge_sharpness: f32,
    /// The size of the bends that the domain warp makes.
    pub warp_wavelength_m: f32,
    /// How far the domain warp moves a position.
    pub warp_amount_m: f32,
    /// The size of the high and low parts of the zone.
    pub massif_wavelength_m: f32,
    /// How much lower the low parts are (0 to 1).
    pub massif_amount: f32,
    /// The erosion filter reads the slope of the base terrain across
    /// this distance. A longer distance makes smoother gully directions.
    pub slope_radius_m: f32,
    /// The reference length of the erosion filter. The filter settings
    /// are parts of this length.
    pub erosion_length_m: f32,
    /// The settings of the erosion filter.
    pub erosion: ErosionParams,
    /// The height of the fine roughness of the ground.
    pub detail_height_m: f32,
    /// The size of the fine roughness of the ground.
    pub detail_wavelength_m: f32,
    /// A sample can be this much higher or lower than the mean of its
    /// 4 neighbours. A smaller value removes more thin spikes.
    pub max_bend_m: f32,
    /// The drainage runs on each n-th sample. 2 is 4 times faster than 1.
    pub drainage_step: usize,
    /// A closed basin that is less deep than this is filled to its rim.
    pub shallow_basin_m: f32,
    /// A deeper basin keeps its shape. Only this part of its depth (0 to
    /// 1) is filled. The flat floor is a dry lake bed.
    pub lake_floor_part: f32,
    /// The noise value where a range ends and the valley floor starts
    /// (0.2 makes thin ranges and wide valleys, 1 makes no valley floor).
    pub valley_at: f32,
    /// The height of the low hills on the valley floors.
    pub floor_height_m: f32,
    /// The size of the low hills on the valley floors.
    pub floor_wavelength_m: f32,
    /// How round the tops of the ranges are (0.05 is sharp, 0.5 is
    /// round). The erosion filter makes the sharp ridges.
    pub ridge_rounding: f32,
    /// The rain area that starts a riverbed, in square metres.
    pub river_min_area_m2: f32,
    /// The width of the smallest riverbed.
    pub river_min_width_m: f32,
    /// The width of the largest riverbed.
    pub river_max_width_m: f32,
    /// The depth of a riverbed, as a part of its width.
    pub river_depth_ratio: f32,
}

impl Default for HighlandsParams {
    fn default() -> Self {
        Self {
            relief_m: 430.0,
            ridge_wavelength_m: 1300.0,
            ridge_octaves: 2,
            ridge_sharpness: 1.2,
            warp_wavelength_m: 900.0,
            warp_amount_m: 170.0,
            massif_wavelength_m: 1900.0,
            massif_amount: 0.4,
            slope_radius_m: 16.0,
            erosion_length_m: 2048.0,
            erosion: ErosionParams {
                strength: 0.16,
                // More rounding than the default of the filter. With less
                // rounding, the ridge tops get thin fins.
                rounding: [0.15, 0.03, 4.0, 2.0],
                ..ErosionParams::default()
            },
            detail_height_m: 0.35,
            detail_wavelength_m: 11.0,
            max_bend_m: 0.5,
            drainage_step: 2,
            shallow_basin_m: 6.0,
            lake_floor_part: 0.2,
            valley_at: 0.5,
            floor_height_m: 40.0,
            floor_wavelength_m: 350.0,
            ridge_rounding: 0.3,
            river_min_area_m2: 30_000.0,
            river_min_width_m: 3.0,
            river_max_width_m: 18.0,
            river_depth_ratio: 0.22,
        }
    }
}

/// One generated zone.
#[derive(Clone, Debug)]
pub struct Zone {
    /// The seed that made the zone.
    pub seed: u32,
    /// The ground heights in metres.
    pub heights: HeightMap,
    /// For each sample: 255 in the bed of a dry river, 0 away from it.
    pub riverbed: Vec<u8>,
    /// For each sample: 255 on a dry lake bed, 0 away from it.
    pub lakebed: Vec<u8>,
}

/// The ground types at one place. Each value is a part from 0 to 1.
/// The colours and the grip of the ground come from these values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    /// 0 is soil, 1 is rock.
    pub rock: f32,
    /// The colour band of the rock: 0 is rust rock, 1 is ash rock.
    pub ash: f32,
    /// 1 on a dry lake bed.
    pub lake: f32,
    /// 1 on the sand of a dry riverbed.
    pub sand: f32,
}

impl Ground {
    /// Gives the grip of the ground from the grip of each ground type.
    /// A grip of 1 holds fully. A grip of 0 does not hold.
    pub fn grip(&self, rust_rock: f32, ash_rock: f32, soil: f32, sand: f32) -> f32 {
        let rock = rust_rock + (ash_rock - rust_rock) * self.ash;
        let ground = soil + (rock - soil) * self.rock;
        ground + (sand - ground) * self.sand
    }

    /// Gives the name of the main ground type: "sand", "lake bed",
    /// "ash rock", "rust rock" or "soil".
    pub fn name(&self) -> &'static str {
        if self.sand >= 0.5 {
            "sand"
        } else if self.lake >= 0.5 {
            "lake bed"
        } else if self.rock >= 0.5 {
            if self.ash >= 0.5 {
                "ash rock"
            } else {
                "rust rock"
            }
        } else {
            "soil"
        }
    }
}

/// Ridged noise: the absolute value of noise, turned over. The zero
/// lines of the noise become sharp ridges. The result is from 0 to 1.
fn ridged(x: f32, y: f32, seed: u32, octaves: u32, rounding: f32, valley_at: f32) -> f32 {
    let mut sum = 0.0;
    let mut norm = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut weight = 1.0;
    for i in 0..octaves {
        let n = gradient_noise(
            x * freq,
            y * freq,
            seed.wrapping_add(i.wrapping_mul(0x51ED_270B)),
        );
        // A soft absolute value. The ridge is round on top, thus the
        // slope has no sudden change of direction.
        let n = n.value / valley_at;
        let soft_abs = (n * n + rounding * rounding).sqrt() - rounding;
        let r = 1.0 - soft_abs.min(1.0);
        let r = r * r * weight;
        // A high layer makes the next layer stronger: ridges get detail,
        // valleys stay smooth.
        weight = (r * 2.0).clamp(0.0, 1.0);
        sum += r * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.1;
    }
    sum / norm
}

/// The base terrain before erosion, in metres, at a world position.
fn base_height(wx: f32, wz: f32, seed: u32, p: &HighlandsParams) -> f32 {
    let (qx, qz) = (wx / p.warp_wavelength_m, wz / p.warp_wavelength_m);
    let warp_x = fbm(qx, qz, seed ^ 0x1111_1111, 3, 2.0, 0.5).value;
    let warp_z = fbm(qx + 5.2, qz + 1.3, seed ^ 0x2222_2222, 3, 2.0, 0.5).value;
    let x = (wx + warp_x * p.warp_amount_m) / p.ridge_wavelength_m;
    let z = (wz + warp_z * p.warp_amount_m) / p.ridge_wavelength_m;

    let r = ridged(
        x,
        z,
        seed ^ 0x3333_3333,
        p.ridge_octaves,
        p.ridge_rounding,
        p.valley_at,
    );
    let r = crate::detmath::pow(r, p.ridge_sharpness);

    let m = fbm(
        wx / p.massif_wavelength_m,
        wz / p.massif_wavelength_m,
        seed ^ 0x4444_4444,
        2,
        2.0,
        0.5,
    );
    let massif = 1.0 - p.massif_amount * (0.5 - 0.5 * m.value).clamp(0.0, 1.0);
    let floor = fbm(
        wx / p.floor_wavelength_m,
        wz / p.floor_wavelength_m,
        seed ^ 0xAAAA_AAAA,
        3,
        2.0,
        0.5,
    );
    r * massif * p.relief_m + (floor.value * 0.5 + 0.5) * p.floor_height_m
}

/// Removes thin spikes and thin holes. A sample that is much higher or
/// lower than the mean of its 4 neighbours moves toward that mean.
fn remove_spikes(mut map: HeightMap, max_bend: f32) -> HeightMap {
    let size = map.size;
    for _ in 0..3 {
        let before = map.clone();
        map.data
            .par_chunks_mut(size)
            .enumerate()
            .for_each(|(z, row)| {
                let zi = z as isize;
                for (x, h) in row.iter_mut().enumerate() {
                    let xi = x as isize;
                    let mean = (before.get(xi - 1, zi)
                        + before.get(xi + 1, zi)
                        + before.get(xi, zi - 1)
                        + before.get(xi, zi + 1))
                        * 0.25;
                    *h = h.clamp(mean - max_bend, mean + max_bend);
                }
            });
    }
    map
}

/// Makes a zone of scorched highlands.
///
/// `size` is the number of samples on each side. `spacing` is the
/// distance between samples in metres. The same seed and settings
/// always make the same zone.
pub fn generate(seed: u32, size: usize, spacing: f32, params: &HighlandsParams) -> Zone {
    let half = size as f32 * 0.5;
    let world = |i: usize| (i as f32 - half) * spacing;

    // Steps 1 and 2: the base terrain.
    let base = HeightMap::from_fn(size, spacing, |x, z| {
        base_height(world(x), world(z), seed, params)
    });
    let (lo, hi) = base.min_max();
    let mid = (lo + hi) * 0.5;
    let half_range = ((hi - lo) * 0.5).max(1e-3);

    // Steps 3 and 4: the erosion filter and the fine roughness.
    let length = params.erosion_length_m;
    let reach = ((params.slope_radius_m / spacing).round() as isize).max(1);
    let run = 2.0 * reach as f32 * spacing;
    let eroded = HeightMap::from_fn(size, spacing, |x, z| {
        let (xi, zi) = (x as isize, z as isize);
        let h = base.get(xi, zi);
        let slope = (
            (base.get(xi + reach, zi) - base.get(xi - reach, zi)) / run,
            (base.get(xi, zi + reach) - base.get(xi, zi - reach)) / run,
        );
        let (wx, wz) = (world(x), world(z));
        let fade_target = (h - mid) / half_range;
        let e = erosion_filter(
            (wx / length, wz / length),
            slope,
            fade_target,
            &params.erosion,
            seed ^ 0x5555_5555,
        );
        let detail = fbm(
            wx / params.detail_wavelength_m,
            wz / params.detail_wavelength_m,
            seed ^ 0x6666_6666,
            3,
            2.0,
            0.5,
        );
        h + e.height * length + detail.value * params.detail_height_m
    });

    let eroded = remove_spikes(eroded, params.max_bend_m);

    // Step 5: drainage on a grid with fewer samples.
    let step = params.drainage_step.max(1);
    let csize = size.div_ceil(step);
    let cell_m = spacing * step as f32;
    let coarse: Vec<f32> = (0..csize * csize)
        .map(|i| eroded.get(((i % csize) * step) as isize, ((i / csize) * step) as isize))
        .collect();
    let (fill, lake) = partial_fill(
        &coarse,
        csize,
        0.0005 * cell_m,
        params.shallow_basin_m,
        params.lake_floor_part,
    );
    let surface: Vec<f32> = coarse.iter().zip(&fill).map(|(h, f)| h + f).collect();
    let rain = flow(&surface, csize);

    // Put the fill on the full grid. `fill_at` gives the depth of the fill.
    let cell = |cx: isize, cz: isize| {
        let max = csize as isize - 1;
        cz.clamp(0, max) as usize * csize + cx.clamp(0, max) as usize
    };
    let fill_at = |cx: isize, cz: isize| fill[cell(cx, cz)];
    let lake_at = |cx: isize, cz: isize| if lake[cell(cx, cz)] { 1.0 } else { 0.0 };
    let mut heights = eroded;
    let mut lakebed = vec![0u8; size * size];
    heights
        .data
        .par_chunks_mut(size)
        .zip(lakebed.par_chunks_mut(size))
        .enumerate()
        .for_each(|(z, (row, lake_row))| {
            let fz = z as f32 / step as f32;
            let z0 = fz.floor();
            let tz = fz - z0;
            for (x, (h, lake)) in row.iter_mut().zip(lake_row.iter_mut()).enumerate() {
                let fx = x as f32 / step as f32;
                let x0 = fx.floor();
                let tx = fx - x0;
                let (cx, cz) = (x0 as isize, z0 as isize);
                let top = fill_at(cx, cz) * (1.0 - tx) + fill_at(cx + 1, cz) * tx;
                let bottom = fill_at(cx, cz + 1) * (1.0 - tx) + fill_at(cx + 1, cz + 1) * tx;
                let fill = top * (1.0 - tz) + bottom * tz;
                *h += fill;
                let top = lake_at(cx, cz) * (1.0 - tx) + lake_at(cx + 1, cz) * tx;
                let bottom = lake_at(cx, cz + 1) * (1.0 - tx) + lake_at(cx + 1, cz + 1) * tx;
                *lake = ((top * (1.0 - tz) + bottom * tz) * 255.0) as u8;
            }
        });

    // Cut the riverbeds. Each river cell stamps a disc into the ground.
    // For each sample, the stamp with the most effect wins. If two stamps
    // have the same effect, the lower bed wins. Thus the order of the
    // stamps does not change the result.
    let cell_area = cell_m * cell_m;
    let river_width = |i: usize| {
        let area = rain.area[i] * cell_area;
        // A river stops at the shore of a dry lake.
        if area < params.river_min_area_m2 || lake[i] {
            return 0.0;
        }
        (params.river_min_width_m * (area / params.river_min_area_m2).sqrt())
            .min(params.river_max_width_m)
            // The discs of two river cells must touch.
            .max(cell_m * 1.5)
    };
    // The level of each riverbed. Go from the lowest cell to the highest
    // cell. A bed is never lower than the bed after it, thus a river
    // does not cut a hole where it stops.
    let mut bed_level = surface.clone();
    for &i in rain.order.iter().rev() {
        let i = i as usize;
        let width = river_width(i);
        if width > 0.0 {
            let after = bed_level[rain.receiver[i] as usize];
            bed_level[i] = (surface[i] - width * params.river_depth_ratio).max(after);
        }
    }
    let mut best_f = vec![0.0f32; size * size];
    let mut best_bed = vec![0.0f32; size * size];
    let mut riverbed = vec![0u8; size * size];
    for (i, &bed) in bed_level.iter().enumerate() {
        let width = river_width(i);
        if width <= 0.0 {
            continue;
        }
        let inner = width * 0.5;
        let outer = inner + width * 0.9 + spacing;
        let (cx, cz) = (((i % csize) * step) as isize, ((i / csize) * step) as isize);
        let reach = (outer / spacing).ceil() as isize;
        for dz in -reach..=reach {
            let z = cz + dz;
            if z < 0 || z >= size as isize {
                continue;
            }
            for dx in -reach..=reach {
                let x = cx + dx;
                if x < 0 || x >= size as isize {
                    continue;
                }
                let dist = ((dx * dx + dz * dz) as f32).sqrt() * spacing;
                if dist >= outer {
                    continue;
                }
                let f = 1.0 - smoothstep(inner, outer, dist);
                let idx = z as usize * size + x as usize;
                if f > best_f[idx] || (f == best_f[idx] && bed < best_bed[idx]) {
                    best_f[idx] = f;
                    best_bed[idx] = bed;
                }
                let mask = 1.0 - smoothstep(inner * 0.8, inner * 1.6 + spacing, dist);
                let mask = (mask * 255.0) as u8;
                if mask > riverbed[idx] {
                    riverbed[idx] = mask;
                }
            }
        }
    }
    let max_raise = params.river_min_width_m * params.river_depth_ratio;
    for ((h, &f), &bed) in heights.data.iter_mut().zip(&best_f).zip(&best_bed) {
        if f > 0.0 {
            // The bed is flat. It cuts high ground, and it fills small holes.
            let target = bed.min(*h + max_raise);
            *h += (target - *h) * f;
        }
    }

    Zone {
        seed,
        heights,
        riverbed,
        lakebed,
    }
}

impl Zone {
    /// Gives the slope at a sample, as rise divided by run.
    pub fn slope(&self, x: usize, z: usize) -> (f32, f32) {
        let (xi, zi) = (x as isize, z as isize);
        let k = 1.0 / (2.0 * self.heights.spacing);
        (
            (self.heights.get(xi + 1, zi) - self.heights.get(xi - 1, zi)) * k,
            (self.heights.get(xi, zi + 1) - self.heights.get(xi, zi - 1)) * k,
        )
    }

    /// Gives the ground types at a sample.
    pub fn ground_at_sample(&self, x: usize, z: usize) -> Ground {
        let size = self.heights.size;
        let (x, z) = (x.min(size - 1), z.min(size - 1));
        let half = size as f32 * 0.5;
        let spacing = self.heights.spacing;
        let (sx, sz) = self.slope(x, z);
        let steep = (sx * sx + sz * sz).sqrt();
        let h = self.heights.data[z * size + x];
        let (wx, wz) = ((x as f32 - half) * spacing, (z as f32 - half) * spacing);

        // Rock starts at a slope of about 35 degrees. At 50 degrees
        // all the ground is rock.
        let rock = smoothstep(0.70, 1.19, steep);
        let seed = self.seed;
        let wobble = gradient_noise(wx / 60.0, wz / 60.0, seed ^ 0x8888_8888).value;
        let band = gradient_noise((h + wobble * 12.0) / 9.0, 0.5, seed ^ 0x9999_9999).value;
        Ground {
            rock,
            ash: smoothstep(-0.25, 0.35, band),
            lake: f32::from(self.lakebed[z * size + x]) / 255.0,
            sand: f32::from(self.riverbed[z * size + x]) / 255.0,
        }
    }

    /// Gives the ground types at a world position (the nearest sample).
    pub fn ground_at(&self, wx: f32, wz: f32) -> Ground {
        let half = self.heights.size as f32 * 0.5;
        let index = |w: f32| (w / self.heights.spacing + half).round().max(0.0) as usize;
        self.ground_at_sample(index(wx), index(wz))
    }

    /// Gives the colour of the ground for each sample, as red, green and
    /// blue from 0 to 255. Flat ground is pale ochre. Steep rock is rust
    /// with bands of ash. Riverbeds and lake beds are pale.
    pub fn ground_colours(&self) -> Vec<[u8; 3]> {
        const OCHRE: [f32; 3] = [0.70, 0.56, 0.36];
        const DUST: [f32; 3] = [0.78, 0.68, 0.50];
        const RUST: [f32; 3] = [0.50, 0.31, 0.20];
        const ASH: [f32; 3] = [0.50, 0.42, 0.36];
        const SAND: [f32; 3] = [0.82, 0.76, 0.62];
        const LAKE: [f32; 3] = [0.87, 0.83, 0.72];

        fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        }

        let size = self.heights.size;
        let spacing = self.heights.spacing;
        let half = size as f32 * 0.5;
        let seed = self.seed;
        let mut out = vec![[0u8; 3]; size * size];
        out.par_chunks_mut(size).enumerate().for_each(|(z, row)| {
            for (x, px) in row.iter_mut().enumerate() {
                let g = self.ground_at_sample(x, z);
                let (wx, wz) = ((x as f32 - half) * spacing, (z as f32 - half) * spacing);

                let patches = fbm(wx / 90.0, wz / 90.0, seed ^ 0x7777_7777, 3, 2.0, 0.5).value;
                let mut c = mix(OCHRE, DUST, (patches * 0.9 + 0.5).clamp(0.0, 1.0));
                c = mix(c, mix(RUST, ASH, g.ash), g.rock);
                c = mix(c, LAKE, g.lake * (1.0 - g.rock));
                c = mix(c, SAND, g.sand);

                *px = [
                    (c[0] * 255.0).round() as u8,
                    (c[1] * 255.0).round() as u8,
                    (c[2] * 255.0).round() as u8,
                ];
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn ground_names() {
        let g = |rock, ash, lake, sand| Ground {
            rock,
            ash,
            lake,
            sand,
        };
        assert_eq!(g(0.0, 0.0, 0.0, 0.0).name(), "soil");
        assert_eq!(g(1.0, 0.2, 0.0, 0.0).name(), "rust rock");
        assert_eq!(g(1.0, 0.8, 0.0, 0.0).name(), "ash rock");
        assert_eq!(g(1.0, 0.8, 0.0, 0.9).name(), "sand");
        assert_eq!(g(0.0, 0.0, 1.0, 0.0).name(), "lake bed");
    }

    use super::*;

    #[test]
    fn same_seed_same_zone() {
        let p = HighlandsParams::default();
        let a = generate(7, 256, 8.0, &p);
        let b = generate(7, 256, 8.0, &p);
        let c = generate(8, 256, 8.0, &p);
        assert_eq!(a.heights.checksum(), b.heights.checksum());
        assert_eq!(a.riverbed, b.riverbed);
        assert_ne!(a.heights.checksum(), c.heights.checksum());
    }

    #[test]
    fn grip_mixes_the_ground_types() {
        let g = |rock, ash, sand| Ground {
            rock,
            ash,
            lake: 0.0,
            sand,
        };
        assert!((g(1.0, 0.0, 0.0).grip(1.0, 0.4, 0.8, 0.1) - 1.0).abs() < 1e-6);
        assert!((g(1.0, 1.0, 0.0).grip(1.0, 0.4, 0.8, 0.1) - 0.4).abs() < 1e-6);
        assert!((g(0.0, 1.0, 0.0).grip(1.0, 0.4, 0.8, 0.1) - 0.8).abs() < 1e-6);
        assert!((g(1.0, 0.0, 1.0).grip(1.0, 0.4, 0.8, 0.1) - 0.1).abs() < 1e-6);
        // The lookup at a world position reads the nearest sample.
        let zone = generate(3, 64, 8.0, &HighlandsParams::default());
        assert_eq!(zone.ground_at(-256.0, -256.0), zone.ground_at_sample(0, 0));
        assert_eq!(zone.ground_at(9.0, 0.0), zone.ground_at_sample(33, 32));
    }

    #[test]
    fn the_zone_has_mountains_and_riverbeds() {
        let zone = generate(3, 256, 8.0, &HighlandsParams::default());
        let (lo, hi) = zone.heights.min_max();
        assert!(zone.heights.data.iter().all(|h| h.is_finite()));
        assert!(hi - lo > 300.0, "relief {}", hi - lo);
        assert!(hi - lo < 900.0, "relief {}", hi - lo);
        let river_samples = zone.riverbed.iter().filter(|&&r| r > 128).count();
        assert!(river_samples > 50, "river samples {river_samples}");
    }

    #[test]
    fn riverbeds_have_no_deep_pools() {
        // If a riverbed has a hole, the basin fill shows it. The dry
        // lakes are outlets, because a river can stop there.
        use crate::drainage::fill_basins;
        let zone = generate(5, 512, 2.0, &HighlandsParams::default());
        let size = zone.heights.size;
        let lakes: Vec<bool> = zone.lakebed.iter().map(|&l| l > 0).collect();
        let filled = fill_basins(&zone.heights.data, size, 0.001, Some(&lakes));
        let mut worst = 0.0f32;
        let mut pools = 0;
        for (i, (level, ground)) in filled.iter().zip(&zone.heights.data).enumerate() {
            if zone.riverbed[i] == 255 && zone.lakebed[i] == 0 {
                let depth = level - ground;
                worst = worst.max(depth);
                pools += usize::from(depth > 2.5);
            }
        }
        assert!(
            pools == 0,
            "{pools} riverbed samples are in pools, the deepest is {worst} m"
        );
    }
}
