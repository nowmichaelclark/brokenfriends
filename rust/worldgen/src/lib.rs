//! World generation for Broken Friends.
//!
//! This crate does not use Godot. All functions are pure: the same seed
//! always gives the same result. The code uses no `sin`, `cos` or `powf`
//! in generation, so the results are the same on all computers.

pub mod heightmap;
pub mod noise;

pub use heightmap::HeightMap;

/// Makes a simple hill terrain from layered noise.
///
/// This is the test terrain of S-101. The biome generators replace it.
/// `size` is the number of samples on each side. `spacing` is the distance
/// between samples in metres. `relief` is the approximate height range in
/// metres.
pub fn test_hills(seed: u32, size: usize, spacing: f32, relief: f32) -> HeightMap {
    let half = size as f32 * 0.5;
    HeightMap::from_fn(size, spacing, |x, z| {
        let wx = (x as f32 - half) * spacing;
        let wz = (z as f32 - half) * spacing;
        let n = noise::fbm(wx / 400.0, wz / 400.0, seed, 6, 2.0, 0.5);
        (n.value * 0.5 + 0.5) * relief
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hills_is_deterministic_and_in_range() {
        let a = test_hills(7, 128, 1.0, 60.0);
        let b = test_hills(7, 128, 1.0, 60.0);
        let c = test_hills(8, 128, 1.0, 60.0);
        assert_eq!(a.checksum(), b.checksum());
        assert_ne!(a.checksum(), c.checksum());
        let (lo, hi) = a.min_max();
        assert!(lo >= -1.0 && hi <= 61.0, "{lo}..{hi}");
    }
}
