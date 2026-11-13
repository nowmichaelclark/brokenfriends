//! A square grid of ground heights.

use rayon::prelude::*;

/// A square heightmap. A heightmap is a grid of ground heights in metres.
///
/// The samples are in rows. The index of sample (`x`, `z`) is
/// `z * size + x`. The centre of the grid is the world origin, thus the
/// world position of sample (`x`, `z`) is `(x - size / 2) * spacing` on
/// each axis.
#[derive(Clone, Debug, PartialEq)]
pub struct HeightMap {
    /// The number of samples on each side.
    pub size: usize,
    /// The distance between two samples in metres.
    pub spacing: f32,
    /// The heights in metres, `size * size` values.
    pub data: Vec<f32>,
}

impl HeightMap {
    /// Makes a flat heightmap with height 0.
    pub fn new(size: usize, spacing: f32) -> Self {
        Self {
            size,
            spacing,
            data: vec![0.0; size * size],
        }
    }

    /// Makes a heightmap from a function of the sample position.
    /// The function runs on many threads, so it must be pure.
    pub fn from_fn(size: usize, spacing: f32, f: impl Fn(usize, usize) -> f32 + Sync) -> Self {
        let mut data = vec![0.0f32; size * size];
        data.par_chunks_mut(size.max(1))
            .enumerate()
            .for_each(|(z, row)| {
                for (x, h) in row.iter_mut().enumerate() {
                    *h = f(x, z);
                }
            });
        Self {
            size,
            spacing,
            data,
        }
    }

    /// Gives the height of one sample. The position is clamped to the grid.
    #[inline]
    pub fn get(&self, x: isize, z: isize) -> f32 {
        let max = self.size as isize - 1;
        let x = x.clamp(0, max) as usize;
        let z = z.clamp(0, max) as usize;
        self.data[z * self.size + x]
    }

    /// Gives the height at a world position in metres (bilinear filter).
    pub fn sample_world(&self, wx: f32, wz: f32) -> f32 {
        let half = self.size as f32 * 0.5;
        let fx = wx / self.spacing + half;
        let fz = wz / self.spacing + half;
        let x0 = fx.floor();
        let z0 = fz.floor();
        let tx = fx - x0;
        let tz = fz - z0;
        let (x0, z0) = (x0 as isize, z0 as isize);
        let a = self.get(x0, z0);
        let b = self.get(x0 + 1, z0);
        let c = self.get(x0, z0 + 1);
        let d = self.get(x0 + 1, z0 + 1);
        let top = a + (b - a) * tx;
        let bottom = c + (d - c) * tx;
        top + (bottom - top) * tz
    }

    /// Gives the lowest and the highest height.
    pub fn min_max(&self) -> (f32, f32) {
        self.data
            .iter()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), &h| {
                (lo.min(h), hi.max(h))
            })
    }

    /// Gives the length of one side in metres.
    pub fn world_size(&self) -> f32 {
        self.size as f32 * self.spacing
    }

    /// Gives a hash of all heights. Use it to compare two worlds.
    pub fn checksum(&self) -> u64 {
        // FNV-1a on the bits of each height.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for v in &self.data {
            for b in v.to_bits().to_le_bytes() {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_fn_puts_samples_in_rows() {
        let m = HeightMap::from_fn(4, 1.0, |x, z| (z * 10 + x) as f32);
        assert_eq!(m.get(3, 0), 3.0);
        assert_eq!(m.get(0, 2), 20.0);
        assert_eq!(
            m.get(-5, 99),
            30.0,
            "positions outside the grid are clamped"
        );
    }

    #[test]
    fn sample_world_has_origin_at_centre() {
        let m = HeightMap::from_fn(8, 2.0, |x, _| x as f32);
        assert_eq!(m.sample_world(0.0, 0.0), 4.0);
        assert_eq!(m.sample_world(1.0, 0.0), 4.5);
        assert_eq!(m.sample_world(-8.0, 3.0), 0.0);
    }

    #[test]
    fn min_max_and_checksum() {
        let a = HeightMap::from_fn(16, 1.0, |x, z| x as f32 - z as f32);
        assert_eq!(a.min_max(), (-15.0, 15.0));
        let mut b = a.clone();
        assert_eq!(a.checksum(), b.checksum());
        b.data[7] += 0.001;
        assert_ne!(a.checksum(), b.checksum());
    }
}
