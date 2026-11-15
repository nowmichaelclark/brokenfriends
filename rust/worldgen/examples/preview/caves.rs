//! The cave images of the preview tool (S-161).

use bf_worldgen::caves::{CaveParams, CaveSystem, Caves, EdgeKind, MouthKind, NodeKind};
use bf_worldgen::highlands::Zone;
use bf_worldgen::smoke::SmokeParams;
use rayon::prelude::*;
use std::path::Path;
use std::time::Instant;

const ROCK: [f32; 3] = [0.36, 0.25, 0.18];
const OPEN: [f32; 3] = [0.86, 0.80, 0.68];
const DARK: [f32; 3] = [0.05, 0.08, 0.20];
const LIT: [f32; 3] = [1.00, 0.86, 0.35];

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn bytes(c: [f32; 3]) -> [u8; 3] {
    [
        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

/// The colour of one point: rock, open air, or cave air with its sky
/// visibility.
fn colour_at(zone: &Zone, caves: &Caves, p: [f32; 3]) -> [f32; 3] {
    let ground = &zone.heights;
    let h = ground.sample_world(p[0], p[2]);
    if caves.cave_value(p) < 0.0 {
        if p[1] >= h {
            return mix(OPEN, LIT, 0.3);
        }
        // A square root makes the dim parts easier to see.
        let sky = caves.sky_visibility(ground, p).sqrt();
        return mix(DARK, LIT, sky);
    }
    if p[1] >= h {
        OPEN
    } else {
        ROCK
    }
}

fn save(path: &Path, w: usize, h: usize, pixels: &[u8]) {
    image::save_buffer(path, pixels, w as u32, h as u32, image::ColorType::Rgb8)
        .expect("could not write a cave image");
}

/// Makes the caves of a zone, prints the numbers and writes the images.
pub fn write_all(zone: &Zone, out: &Path) {
    let start = Instant::now();
    let caves = Caves::generate(zone, &SmokeParams::default(), &CaveParams::default());
    let ms = start.elapsed().as_secs_f32() * 1000.0;
    let (sx, sz) = zone.start_point();
    let nearest = caves
        .mouths()
        .map(|m| ((m.pos[0] - sx).powi(2) + (m.pos[2] - sz).powi(2)).sqrt())
        .fold(f32::INFINITY, f32::min);
    println!(
        "  caves: {} systems, {} chambers, {} mouths, {} shapes in {ms:.0} ms, nearest mouth {nearest:.0} m \
         from the start, roof faults {}, checksum {:016x}",
        caves.systems.len(),
        caves
            .systems
            .iter()
            .map(|s| s.chambers().count())
            .sum::<usize>(),
        caves.mouths().count(),
        caves.shape_count(),
        caves.roof_faults,
        caves.checksum()
    );
    for sys in &caves.systems {
        let widest = sys
            .chambers()
            .map(|i| sys.nodes[i].radius_m * 2.0)
            .fold(0.0, f32::max);
        println!(
            "    system {}: {} chambers (widest {widest:.0} m), {} mouths, {:.0} m deep, from y {:.0} to {:.0}",
            sys.index,
            sys.chambers().count(),
            sys.mouths.len(),
            sys.highest_mouth_y() - sys.lowest_y,
            sys.lowest_y,
            sys.highest_y
        );
    }
    let start = Instant::now();
    let mask = caves.hole_mask(&zone.heights, 2);
    println!(
        "    hole mask: {} hole samples, {} band samples in {:.0} ms",
        mask.iter().filter(|&&m| m == 2).count(),
        mask.iter().filter(|&&m| m == 1).count(),
        start.elapsed().as_secs_f32() * 1000.0
    );

    let seed = zone.seed;
    write_map(
        zone,
        &caves,
        &out.join(format!("seed_{seed}_caves_map.png")),
    );
    // Slices at the middle height of four chambers, from high to low.
    let mut heights: Vec<f32> = caves
        .systems
        .iter()
        .flat_map(|s| s.chambers().map(|i| s.nodes[i]))
        .map(|c| (c.floor_y + c.half_height_m * 0.6).round())
        .collect();
    heights.sort_by(|a, b| b.total_cmp(a));
    let mut picked: Vec<f32> = Vec::new();
    for i in 0..4 {
        let y = heights[(heights.len() - 1) * i / 3];
        if picked.iter().all(|p| (p - y).abs() > 20.0) {
            picked.push(y);
        }
    }
    for y in picked {
        write_slice(
            zone,
            &caves,
            y,
            &out.join(format!("seed_{seed}_caves_y{y:.0}.png")),
        );
    }
    for sys in &caves.systems {
        let s = sys.index;
        write_path(
            zone,
            &caves,
            sys,
            &out.join(format!("seed_{seed}_cave_{s}_path.png")),
        );
        write_hall(
            zone,
            &caves,
            sys,
            &out.join(format!("seed_{seed}_cave_{s}_hall.png")),
        );
    }
}

/// The cave graph from above.
fn write_map(zone: &Zone, caves: &Caves, path: &Path) {
    let world = zone.heights.world_size();
    let px = 2.0;
    let n = (world / px) as usize;
    let half = world * 0.5;
    let (lo, hi) = zone.heights.min_max();
    let deepest = caves
        .systems
        .iter()
        .map(|s| s.lowest_y)
        .fold(f32::INFINITY, f32::min);
    let (sx, sz) = zone.start_point();
    let mut pixels = vec![0u8; n * n * 3];
    pixels
        .par_chunks_mut(n * 3)
        .enumerate()
        .for_each(|(iz, row)| {
            let z = iz as f32 * px - half + px * 0.5;
            for ix in 0..n {
                let x = ix as f32 * px - half + px * 0.5;
                let h = zone.heights.sample_world(x, z);
                let grey = 0.25 + 0.45 * (h - lo) / (hi - lo).max(1.0);
                let mut c = [grey, grey, grey * 0.95];
                // The highest cave air in this column (from the graph).
                let mut top: Option<f32> = None;
                for sys in &caves.systems {
                    for node in &sys.nodes {
                        if node.kind != NodeKind::Chamber {
                            continue;
                        }
                        let (dx, dz) = (x - node.pos[0], z - node.pos[2]);
                        if dx * dx + dz * dz <= node.radius_m * node.radius_m {
                            let y = node.pos[1] + node.half_height_m;
                            top = Some(top.map_or(y, |t: f32| t.max(y)));
                        }
                    }
                    for e in &sys.edges {
                        if e.kind == EdgeKind::Inside {
                            continue;
                        }
                        for j in 0..e.path.len() - 1 {
                            let (a, b) = (e.path[j], e.path[j + 1]);
                            let r = e.radii[j].max(e.radii[j + 1]);
                            let (abx, abz) = (b[0] - a[0], b[2] - a[2]);
                            let l2 = (abx * abx + abz * abz).max(1e-6);
                            let t = (((x - a[0]) * abx + (z - a[2]) * abz) / l2).clamp(0.0, 1.0);
                            let (qx, qz) = (a[0] + abx * t, a[2] + abz * t);
                            if (x - qx).powi(2) + (z - qz).powi(2) <= r * r {
                                let y = a[1] + (b[1] - a[1]) * t + r;
                                top = Some(top.map_or(y, |v: f32| v.max(y)));
                            }
                        }
                    }
                }
                if let Some(y) = top {
                    let t = ((h - y) / (h - deepest).max(1.0)).clamp(0.0, 1.0);
                    c = mix([1.0, 0.85, 0.4], [0.25, 0.05, 0.45], t);
                }
                let r = (x * x + z * z).sqrt();
                if (r - caves.safe_radius_m).abs() < px {
                    c = [0.6, 0.6, 0.6];
                }
                if ((x - sx).abs() < px * 0.6 && (z - sz).abs() < 12.0)
                    || ((z - sz).abs() < px * 0.6 && (x - sx).abs() < 12.0)
                {
                    c = [1.0, 1.0, 1.0];
                }
                for m in caves.mouths() {
                    if (x - m.pos[0]).powi(2) + (z - m.pos[2]).powi(2) <= 64.0 {
                        c = match m.kind {
                            MouthKind::Hillside => [0.95, 0.1, 0.1],
                            _ => [0.1, 0.9, 0.95],
                        };
                    }
                }
                row[ix * 3..ix * 3 + 3].copy_from_slice(&bytes(c));
            }
        });
    save(path, n, n, &pixels);
}

/// A horizontal slice of the safe part of the zone at height `y`.
fn write_slice(zone: &Zone, caves: &Caves, y: f32, path: &Path) {
    let r = caves.safe_radius_m.ceil();
    let n = (2.0 * r) as usize;
    let mut pixels = vec![0u8; n * n * 3];
    pixels
        .par_chunks_mut(n * 3)
        .enumerate()
        .for_each(|(iz, row)| {
            let z = iz as f32 - r + 0.5;
            for ix in 0..n {
                let x = ix as f32 - r + 0.5;
                let c = colour_at(zone, caves, [x, y, z]);
                row[ix * 3..ix * 3 + 3].copy_from_slice(&bytes(c));
            }
        });
    save(path, n, n, &pixels);
}

/// The way from the first mouth to the deepest chamber, as a list of
/// points (x, y, z).
fn main_way(sys: &CaveSystem) -> Vec<[f32; 3]> {
    let goal = sys
        .chambers()
        .min_by(|&a, &b| sys.nodes[a].floor_y.total_cmp(&sys.nodes[b].floor_y))
        .unwrap_or(0);
    // Breadth-first search from the first mouth node.
    let n = sys.nodes.len();
    let mut from: Vec<Option<(usize, usize)>> = vec![None; n];
    let start = sys.mouths[0].node;
    let mut queue = std::collections::VecDeque::from([start]);
    let mut seen = vec![false; n];
    seen[start] = true;
    while let Some(i) = queue.pop_front() {
        for (ei, e) in sys.edges.iter().enumerate() {
            if e.kind == EdgeKind::Shaft {
                continue;
            }
            let other = if e.a == i {
                e.b
            } else if e.b == i {
                e.a
            } else {
                continue;
            };
            if !seen[other] {
                seen[other] = true;
                from[other] = Some((i, ei));
                queue.push_back(other);
            }
        }
    }
    let mut steps = Vec::new();
    let mut at = goal;
    while let Some((prev, ei)) = from[at] {
        steps.push((prev, ei));
        at = prev;
    }
    steps.reverse();
    let mut way = Vec::new();
    for (prev, ei) in steps {
        let e = &sys.edges[ei];
        let mut part = e.path.clone();
        if e.a != prev {
            part.reverse();
        }
        way.extend(part);
    }
    way
}

/// A vertical slice of a system along its main way, straightened.
fn write_path(zone: &Zone, caves: &Caves, sys: &CaveSystem, path: &Path) {
    let way = main_way(sys);
    if way.len() < 2 {
        return;
    }
    // One column for each metre along the way.
    let mut columns = Vec::new();
    for w in way.windows(2) {
        let (a, b) = (w[0], w[1]);
        let flat = ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let steps = flat.ceil() as usize;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            columns.push([a[0] + (b[0] - a[0]) * t, a[2] + (b[2] - a[2]) * t]);
        }
    }
    let ground_top = columns
        .iter()
        .map(|c| zone.heights.sample_world(c[0], c[1]))
        .fold(f32::NEG_INFINITY, f32::max);
    let y_top = ground_top.max(sys.highest_y) + 20.0;
    let y_low = sys.lowest_y - 20.0;
    write_vertical(zone, caves, &columns, y_low, y_top, path);
}

/// A vertical slice from west to east through the hall of a system.
fn write_hall(zone: &Zone, caves: &Caves, sys: &CaveSystem, path: &Path) {
    let Some(hall) = sys.chambers().next().map(|i| sys.nodes[i]) else {
        return;
    };
    let reach = hall.radius_m + 60.0;
    let columns: Vec<[f32; 2]> = (0..(2.0 * reach) as usize)
        .map(|i| [hall.pos[0] - reach + i as f32, hall.pos[2]])
        .collect();
    let ground_top = columns
        .iter()
        .map(|c| zone.heights.sample_world(c[0], c[1]))
        .fold(f32::NEG_INFINITY, f32::max);
    let y_low = hall.floor_y - 30.0;
    let y_top = ground_top + 20.0;
    write_vertical(zone, caves, &columns, y_low, y_top, path);
}

fn write_vertical(
    zone: &Zone,
    caves: &Caves,
    columns: &[[f32; 2]],
    y_low: f32,
    y_top: f32,
    path: &Path,
) {
    let w = columns.len();
    let h = (y_top - y_low).ceil().max(1.0) as usize;
    let mut pixels = vec![0u8; w * h * 3];
    pixels
        .par_chunks_mut(w * 3)
        .enumerate()
        .for_each(|(iy, row)| {
            let y = y_top - iy as f32 - 0.5;
            for (ix, c) in columns.iter().enumerate() {
                let colour = colour_at(zone, caves, [c[0], y, c[1]]);
                row[ix * 3..ix * 3 + 3].copy_from_slice(&bytes(colour));
            }
        });
    save(path, w, h, &pixels);
}
