//! Writes preview images of generated zones. No Godot is necessary.
//!
//! Run: `cargo run --release -p bf-worldgen --example preview -- --seed 7`
//!
//! Options:
//! - `--seed N`: the seed. Use the option again for more zones.
//! - `--size N`: the number of samples on each side (default 2048).
//! - `--spacing M`: the distance between samples in metres (default 1).
//! - `--out DIR`: the folder for the images (default `preview`).
//!
//! For each seed, the tool writes four images:
//! - `seed_N_height.png`: the heights, 16 bits, black is low.
//! - `seed_N_relief.png`: a map with ground colours and shadows.
//! - `seed_N_air.png`: a view from the air through haze.
//! - `seed_N_ground.png`: a view from the eyes of the bear.

use bf_worldgen::highlands::{generate, HighlandsParams, Zone};
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;

const SUN: [f32; 3] = [0.52, 0.56, 0.64];
const HAZE: [f32; 3] = [0.80, 0.70, 0.52];

fn main() {
    let mut seeds: Vec<u32> = Vec::new();
    let mut size = 2048usize;
    let mut spacing = 1.0f32;
    let mut out = PathBuf::from("preview");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| panic!("{arg} needs a value"));
        match arg.as_str() {
            "--seed" => seeds.push(value().parse().expect("the seed must be a number")),
            "--size" => size = value().parse().expect("the size must be a number"),
            "--spacing" => spacing = value().parse().expect("the spacing must be a number"),
            "--out" => out = PathBuf::from(value()),
            other => panic!("unknown option {other}"),
        }
    }
    if seeds.is_empty() {
        seeds.push(1);
    }
    std::fs::create_dir_all(&out).expect("could not make the output folder");

    let params = HighlandsParams::default();
    for seed in seeds {
        let start = Instant::now();
        let zone = generate(seed, size, spacing, &params);
        let seconds = start.elapsed().as_secs_f32();
        print_numbers(&zone, seconds);

        let colours = zone.ground_colours();
        write_height(&zone, &out.join(format!("seed_{seed}_height.png")));
        write_relief(
            &zone,
            &colours,
            &out.join(format!("seed_{seed}_relief.png")),
        );

        // A camera in the air, near one corner. It looks at the centre.
        let edge = zone.heights.world_size() * 0.5;
        let (lo, hi) = zone.heights.min_max();
        let eye = [-edge * 0.8, lo + (hi - lo) * 1.05, edge * 0.8];
        let target = [0.0, lo + (hi - lo) * 0.3, 0.0];
        write_view(
            &zone,
            &colours,
            eye,
            target,
            60.0,
            &out.join(format!("seed_{seed}_air.png")),
        );

        // A camera at the eyes of the bear, at the lowest point of the
        // zone. It looks at the highest point.
        let (low, high) = lowest_and_highest(&zone);
        let eye = [low[0], low[1] + 0.35, low[2]];
        write_view(
            &zone,
            &colours,
            eye,
            high,
            75.0,
            &out.join(format!("seed_{seed}_ground.png")),
        );
    }
    println!("The images are in {}", out.display());
}

fn print_numbers(zone: &Zone, seconds: f32) {
    let size = zone.heights.size;
    let (lo, hi) = zone.heights.min_max();
    let mut walk = 0usize;
    let mut climb = 0usize;
    for z in 0..size {
        for x in 0..size {
            let (sx, sz) = zone.slope(x, z);
            let steep = (sx * sx + sz * sz).sqrt();
            if steep < 0.7 {
                walk += 1; // less than 35 degrees
            } else if steep > 1.19 {
                climb += 1; // more than 50 degrees
            }
        }
    }
    let n = (size * size) as f32;
    let river = zone.riverbed.iter().filter(|&&r| r > 128).count() as f32;
    let lake = zone.lakebed.iter().filter(|&&r| r > 128).count() as f32;
    println!(
        "seed {}: {:.1} s, heights {:.0} to {:.0} m (relief {:.0} m), walk {:.0}%, climb {:.0}%, \
         riverbed {:.1}%, lake bed {:.1}%, checksum {:016x}",
        zone.seed,
        seconds,
        lo,
        hi,
        hi - lo,
        walk as f32 / n * 100.0,
        climb as f32 / n * 100.0,
        river / n * 100.0,
        lake / n * 100.0,
        zone.heights.checksum()
    );
}

fn lowest_and_highest(zone: &Zone) -> ([f32; 3], [f32; 3]) {
    let size = zone.heights.size;
    let half = size as f32 * 0.5;
    let spacing = zone.heights.spacing;
    let position = |i: usize| {
        [
            ((i % size) as f32 - half) * spacing,
            zone.heights.data[i],
            ((i / size) as f32 - half) * spacing,
        ]
    };
    // Do not use the samples near the border.
    let margin = size / 6;
    let mut low = 0;
    let mut high = 0;
    let mut first = true;
    for z in margin..size - margin {
        for x in margin..size - margin {
            let i = z * size + x;
            if first || zone.heights.data[i] < zone.heights.data[low] {
                low = i;
            }
            if first || zone.heights.data[i] > zone.heights.data[high] {
                high = i;
            }
            first = false;
        }
    }
    (position(low), position(high))
}

fn write_height(zone: &Zone, path: &std::path::Path) {
    let size = zone.heights.size as u32;
    let (lo, hi) = zone.heights.min_max();
    let range = (hi - lo).max(1e-3);
    let pixels: Vec<u16> = zone
        .heights
        .data
        .iter()
        .map(|h| ((h - lo) / range * 65535.0) as u16)
        .collect();
    let image: image::ImageBuffer<image::Luma<u16>, _> =
        image::ImageBuffer::from_raw(size, size, pixels).expect("wrong image size");
    image.save(path).expect("could not write the height image");
}

fn normal_at(zone: &Zone, x: usize, z: usize) -> [f32; 3] {
    let (sx, sz) = zone.slope(x, z);
    let len = (sx * sx + 1.0 + sz * sz).sqrt();
    [-sx / len, 1.0 / len, -sz / len]
}

fn shade(colour: [u8; 3], normal: [f32; 3], lit: f32) -> [f32; 3] {
    let sun = (normal[0] * SUN[0] + normal[1] * SUN[1] + normal[2] * SUN[2]).max(0.0) * lit;
    let sky = 0.35 + 0.15 * normal[1];
    let light = sun * 0.9 + sky;
    [
        f32::from(colour[0]) / 255.0 * light,
        f32::from(colour[1]) / 255.0 * light,
        f32::from(colour[2]) / 255.0 * light,
    ]
}

fn to_bytes(c: [f32; 3]) -> [u8; 3] {
    // A simple gamma curve.
    [
        (c[0].clamp(0.0, 1.0).sqrt() * 255.0) as u8,
        (c[1].clamp(0.0, 1.0).sqrt() * 255.0) as u8,
        (c[2].clamp(0.0, 1.0).sqrt() * 255.0) as u8,
    ]
}

fn write_relief(zone: &Zone, colours: &[[u8; 3]], path: &std::path::Path) {
    let size = zone.heights.size;
    let mut bytes = vec![0u8; size * size * 3];
    bytes
        .par_chunks_mut(size * 3)
        .enumerate()
        .for_each(|(z, row)| {
            for x in 0..size {
                let lit = shade(colours[z * size + x], normal_at(zone, x, z), 1.0);
                // A map has no gamma problem with linear colours: keep it bright.
                let c = to_bytes([lit[0] * lit[0], lit[1] * lit[1], lit[2] * lit[2]]);
                row[x * 3..x * 3 + 3].copy_from_slice(&c);
            }
        });
    image::save_buffer(
        path,
        &bytes,
        size as u32,
        size as u32,
        image::ColorType::Rgb8,
    )
    .expect("could not write the relief image");
}

/// Moves along a ray until it goes into the ground. Gives the distance.
fn march(zone: &Zone, from: [f32; 3], dir: [f32; 3], max: f32) -> Option<f32> {
    let edge = zone.heights.world_size() * 0.5;
    let mut t = 0.05;
    let mut before = 0.0;
    while t < max {
        let p = [
            from[0] + dir[0] * t,
            from[1] + dir[1] * t,
            from[2] + dir[2] * t,
        ];
        if p[0].abs() > edge || p[2].abs() > edge {
            // Outside the zone: stop if the ray moves away.
            if p[0] * dir[0] > 0.0 && p[0].abs() > edge || p[2] * dir[2] > 0.0 && p[2].abs() > edge
            {
                return None;
            }
        } else if p[1] < zone.heights.sample_world(p[0], p[2]) {
            // Find the surface between the last two steps.
            let (mut a, mut b) = (before, t);
            for _ in 0..8 {
                let m = (a + b) * 0.5;
                let q = [
                    from[0] + dir[0] * m,
                    from[1] + dir[1] * m,
                    from[2] + dir[2] * m,
                ];
                if q[1] < zone.heights.sample_world(q[0], q[2]) {
                    b = m;
                } else {
                    a = m;
                }
            }
            return Some(b);
        }
        before = t;
        t += (t * 0.004).max(0.05);
    }
    None
}

fn write_view(
    zone: &Zone,
    colours: &[[u8; 3]],
    eye: [f32; 3],
    target: [f32; 3],
    fov_degrees: f32,
    path: &std::path::Path,
) {
    const W: usize = 1280;
    const H: usize = 720;
    let size = zone.heights.size;
    let half = size as f32 * 0.5;
    let spacing = zone.heights.spacing;

    let norm = |v: [f32; 3]| {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        [v[0] / l, v[1] / l, v[2] / l]
    };
    let forward = norm([target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]]);
    let right = norm([-forward[2], 0.0, forward[0]]);
    let up = [
        right[1] * forward[2] - right[2] * forward[1],
        right[2] * forward[0] - right[0] * forward[2],
        right[0] * forward[1] - right[1] * forward[0],
    ];
    let tan = (fov_degrees.to_radians() * 0.5).tan();

    let mut bytes = vec![0u8; W * H * 3];
    bytes
        .par_chunks_mut(W * 3)
        .enumerate()
        .for_each(|(py, row)| {
            for px in 0..W {
                let u = (px as f32 + 0.5 - W as f32 * 0.5) / (H as f32 * 0.5) * tan;
                let v = -(py as f32 + 0.5 - H as f32 * 0.5) / (H as f32 * 0.5) * tan;
                let dir = norm([
                    forward[0] + right[0] * u + up[0] * v,
                    forward[1] + right[1] * u + up[1] * v,
                    forward[2] + right[2] * u + up[2] * v,
                ]);
                // The sky: haze, a little brighter near the sun.
                let to_sun = (dir[0] * SUN[0] + dir[1] * SUN[1] + dir[2] * SUN[2]).max(0.0);
                let glow = to_sun.powi(24) * 0.25;
                let sky = [HAZE[0] + glow, HAZE[1] + glow, HAZE[2] + glow * 0.8];
                let mut c = sky;
                if let Some(t) = march(zone, eye, dir, 4000.0) {
                    let p = [
                        eye[0] + dir[0] * t,
                        eye[1] + dir[1] * t,
                        eye[2] + dir[2] * t,
                    ];
                    let x = ((p[0] / spacing + half) as usize).min(size - 1);
                    let z = ((p[2] / spacing + half) as usize).min(size - 1);
                    let above = [p[0], p[1] + 0.3, p[2]];
                    let lit = if march(zone, above, SUN, 1500.0).is_some() {
                        0.0
                    } else {
                        1.0
                    };
                    let ground = shade(colours[z * size + x], normal_at(zone, x, z), lit);
                    // Smog hides the far ground. The value 700 m is a guess
                    // for the preview only.
                    let fog = 1.0 - (-t / 700.0).exp();
                    c = [
                        ground[0] + (HAZE[0] - ground[0]) * fog,
                        ground[1] + (HAZE[1] - ground[1]) * fog,
                        ground[2] + (HAZE[2] - ground[2]) * fog,
                    ];
                }
                row[px * 3..px * 3 + 3].copy_from_slice(&to_bytes([
                    c[0] * c[0],
                    c[1] * c[1],
                    c[2] * c[2],
                ]));
            }
        });
    image::save_buffer(path, &bytes, W as u32, H as u32, image::ColorType::Rgb8)
        .expect("could not write the view image");
}
