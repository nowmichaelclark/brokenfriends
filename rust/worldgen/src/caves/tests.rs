//! The tests of the cave layer. They use zones of 512 × 512 samples
//! with 4 m spacing: the full zone size (2048 m) and a fast build.

use super::*;
use crate::highlands::{generate, HighlandsParams};
use std::sync::OnceLock;
use std::time::Instant;

const SEEDS: [u32; 3] = [1, 7, 42];

fn zone(seed: u32) -> &'static (Zone, Caves) {
    static ZONES: OnceLock<Vec<(Zone, Caves)>> = OnceLock::new();
    let all = ZONES.get_or_init(|| {
        SEEDS
            .iter()
            .map(|&s| {
                let zone = generate(s, 512, 4.0, &HighlandsParams::default());
                let caves = Caves::generate(&zone, &SmokeParams::default(), &CaveParams::default());
                (zone, caves)
            })
            .collect()
    });
    &all[SEEDS.iter().position(|&s| s == seed).unwrap()]
}

/// Points spread over the cave air and the rock near it.
fn probe_points(caves: &Caves, count: usize) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    let mut i = 0u32;
    for sys in &caves.systems {
        for e in &sys.edges {
            for (q, r) in e.path.iter().zip(&e.radii) {
                for k in 0..4 {
                    i += 1;
                    let j = |salt: u32| crate::noise::hash01(i as i32, salt as i32, 99) * 2.0 - 1.0;
                    let s = r * 2.5 + 4.0;
                    out.push([q[0] + j(1) * s, q[1] + j(2) * s, q[2] + j(3) * s + k as f32]);
                }
            }
        }
        for n in &sys.nodes {
            if n.kind == NodeKind::Chamber {
                for _ in 0..200 {
                    i += 1;
                    let j = |salt: u32| crate::noise::hash01(i as i32, salt as i32, 98) * 2.0 - 1.0;
                    out.push([
                        n.pos[0] + j(1) * n.radius_m * 1.2,
                        n.pos[1] + j(2) * n.half_height_m * 1.3,
                        n.pos[2] + j(3) * n.radius_m * 1.2,
                    ]);
                }
            }
        }
    }
    out.truncate(count);
    out
}

#[test]
fn same_seed_gives_same_caves() {
    let zone7 = generate(7, 512, 4.0, &HighlandsParams::default());
    let a = Caves::generate(&zone7, &SmokeParams::default(), &CaveParams::default());
    let b = Caves::generate(&zone7, &SmokeParams::default(), &CaveParams::default());
    assert_eq!(a.checksum(), b.checksum());
    assert_eq!(a.systems, b.systems);
    assert_ne!(a.checksum(), zone(1).1.checksum());
    assert_ne!(zone(1).1.checksum(), zone(42).1.checksum());
}

#[test]
fn sizes_follow_the_spec() {
    let p = CaveParams::default();
    for seed in SEEDS {
        let (_, caves) = zone(seed);
        let n = caves.systems.len();
        println!("seed {seed}: {n} systems, {} shapes", caves.shape_count());
        assert!((2..=5).contains(&n), "seed {seed}: {n} systems");
        for sys in &caves.systems {
            assert_eq!(sys.mouths[0].kind, MouthKind::Hillside);
            let depth = sys.highest_mouth_y() - sys.lowest_y;
            let mut widest: f32 = 0.0;
            for i in sys.chambers() {
                let c = &sys.nodes[i];
                let (w, h) = (c.radius_m * 2.0, c.half_height_m * 2.0);
                widest = widest.max(w);
                assert!(
                    (40.0..=250.0).contains(&w),
                    "seed {seed}: chamber {w} m wide"
                );
                assert!(
                    (20.0..=120.0).contains(&h),
                    "seed {seed}: chamber {h} m high"
                );
            }
            for e in &sys.edges {
                if e.kind == EdgeKind::Inside {
                    continue;
                }
                for r in &e.radii {
                    let w = r * 2.0;
                    assert!((3.0..=25.0).contains(&w), "seed {seed}: tunnel {w} m wide");
                }
            }
            let walk = &sys.edges[1];
            let (a, b) = (walk.path[0], walk.path[walk.path.len() - 1]);
            let flat = ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            let walk_slope =
                ((a[1] - walk.radii[0]) - (b[1] - walk.radii[walk.radii.len() - 1])) / flat;
            assert!(
                walk_slope <= p.steep_slope,
                "seed {seed}: the way to the hall is too steep"
            );
            println!(
                "  system {}: way to the hall: slope {walk_slope:.2} over {flat:.0} m; {} chambers (widest {widest:.0} m), {} edges, {} mouths, depth {depth:.0} m",
                sys.index,
                sys.chambers().count(),
                sys.edges.len(),
                sys.mouths.len()
            );
            assert!(widest >= p.hall_width_min_m, "seed {seed}: no hall");
            assert!(
                (p.depth_min_m..=p.depth_max_m).contains(&depth),
                "seed {seed}: system {} is {depth} m deep",
                sys.index
            );
        }
    }
}

#[test]
fn caves_stay_in_the_safe_part() {
    for seed in SEEDS {
        let (_, caves) = zone(seed);
        let safe = caves.safe_radius_m;
        assert!(safe > 400.0, "the safe radius is {safe} m");
        for sys in &caves.systems {
            for n in &sys.nodes {
                let r = (n.pos[0].powi(2) + n.pos[2].powi(2)).sqrt();
                let extent = if n.kind == NodeKind::Chamber {
                    n.radius_m
                } else {
                    0.0
                };
                assert!(r + extent <= safe, "seed {seed}: a node at {r} m");
            }
            for e in &sys.edges {
                for (q, rad) in e.path.iter().zip(&e.radii) {
                    let r = (q[0].powi(2) + q[2].powi(2)).sqrt();
                    assert!(r + rad <= safe, "seed {seed}: a tunnel at {r} m");
                }
            }
        }
    }
}

/// Gives the columns (on a 2 m grid) with cave air less than
/// `roof_min_m` below the ground, away from the mouths, and the number
/// of columns that the check read.
fn thin_roofs(zone: &Zone, caves: &Caves) -> (Vec<(f32, f32, f32)>, usize) {
    let safe = caves.safe_radius_m;
    let mut columns = 0;
    let mut bad = Vec::new();
    let step = 2.0;
    let n = (safe / step) as i32;
    for iz in -n..=n {
        for ix in -n..=n {
            let (x, z) = (ix as f32 * step, iz as f32 * step);
            if x * x + z * z > safe * safe {
                continue;
            }
            let near_mouth = caves.mouths().any(|m| {
                ((m.pos[0] - x).powi(2) + (m.pos[2] - z).powi(2)).sqrt() <= m.exempt_radius_m
            });
            if near_mouth {
                continue;
            }
            columns += 1;
            let h = zone.heights.sample_world(x, z);
            let mut d = 0.0;
            while d <= caves.params.roof_min_m {
                if caves.cave_value([x, h - d, z]) < 0.0 {
                    bad.push((x, z, d));
                    break;
                }
                d += 0.5;
            }
        }
    }
    (bad, columns)
}

#[test]
fn the_roof_is_thick_except_at_mouths() {
    for seed in SEEDS {
        let (zone, caves) = zone(seed);
        assert_eq!(caves.roof_faults, 0, "seed {seed}: roof faults");
        let (bad, columns) = thin_roofs(zone, caves);
        println!(
            "seed {seed}: {columns} columns, {} with a thin roof",
            bad.len()
        );
        assert!(
            bad.is_empty(),
            "seed {seed}: thin roof at {:?}",
            &bad[..bad.len().min(5)]
        );
    }
}

/// The zones of the game: 2048 samples with 1 m spacing. Run by hand:
/// `cargo test --release -p bf-worldgen full_size -- --ignored --nocapture`.
#[test]
#[ignore]
fn full_size_zones_keep_the_rules() {
    for seed in SEEDS {
        let zone = generate(seed, 2048, 1.0, &HighlandsParams::default());
        let start = Instant::now();
        let caves = Caves::generate(&zone, &SmokeParams::default(), &CaveParams::default());
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let (bad, columns) = thin_roofs(&zone, &caves);
        let holes = caves
            .hole_mask(&zone.heights, 2)
            .iter()
            .filter(|&&m| m == 2)
            .count();
        println!(
            "seed {seed}: {} systems, {} mouths, built in {ms:.0} ms, roof faults {}, {columns} columns with {} thin roofs, {holes} hole samples",
            caves.systems.len(),
            caves.mouths().count(),
            caves.roof_faults,
            bad.len()
        );
        assert_eq!(caves.roof_faults, 0);
        assert!(
            bad.is_empty(),
            "seed {seed}: thin roof at {:?}",
            &bad[..bad.len().min(5)]
        );
    }
}

#[test]
fn every_node_has_a_path_to_a_mouth() {
    for seed in SEEDS {
        for sys in &zone(seed).1.systems {
            for (i, n) in sys.nodes.iter().enumerate() {
                assert!(
                    n.mouth_path_m.is_finite(),
                    "seed {seed}: node {i} has no path"
                );
            }
        }
    }
}

#[test]
fn the_grid_gives_the_same_bits_as_all_shapes() {
    for seed in SEEDS {
        let (_, caves) = zone(seed);
        let points = probe_points(caves, 20_000);
        let mut air = 0;
        for p in &points {
            let fast = caves.cave_value(*p);
            let slow = caves.cave_value_slow(*p);
            assert_eq!(fast.to_bits(), slow.to_bits(), "seed {seed} at {p:?}");
            if fast < 0.0 {
                air += 1;
            }
        }
        assert!(
            air > points.len() / 10,
            "seed {seed}: only {air} points in cave air"
        );
    }
}

/// The corner of the 32 m chunk that holds a point.
fn chunk_of(p: [f32; 3]) -> [f32; 3] {
    [
        (p[0] / 32.0).floor() * 32.0,
        (p[1] / 32.0).floor() * 32.0,
        (p[2] / 32.0).floor() * 32.0,
    ]
}

#[test]
fn a_chunk_fill_gives_the_same_bits_as_point_queries() {
    let (zone, caves) = zone(7);
    let hall = &caves.systems[0].nodes[caves.systems[0].chambers().next().unwrap()];
    let wall = [hall.pos[0] + hall.radius_m, hall.pos[1], hall.pos[2]];
    for corner in [chunk_of(wall), chunk_of(caves.systems[0].mouths[0].pos)] {
        let mut cave = Vec::new();
        let mut air = Vec::new();
        caves.fill_cave(corner, 1.0, [34, 34, 34], &mut cave);
        caves.fill_air(&zone.heights, corner, 1.0, [34, 34, 34], &mut air);
        let mut signs = [0, 0];
        for iz in 0..34 {
            for iy in 0..34 {
                for ix in 0..34 {
                    let i = ix + 34 * (iy + 34 * iz);
                    let p = [
                        corner[0] + ix as f32,
                        corner[1] + iy as f32,
                        corner[2] + iz as f32,
                    ];
                    assert_eq!(cave[i].to_bits(), caves.cave_value(p).to_bits());
                    assert_eq!(
                        air[i].to_bits(),
                        caves.air_value(&zone.heights, p).to_bits()
                    );
                    signs[usize::from(air[i] > 0.0)] += 1;
                }
            }
        }
        assert!(
            signs[0] > 0 && signs[1] > 0,
            "the chunk must have a wall: {signs:?}"
        );
    }
}

#[test]
fn the_bound_check_never_skips_a_wall() {
    for seed in SEEDS {
        let (_, caves) = zone(seed);
        let mut counts = [0usize; 3];
        let mut walls = 0;
        let low = caves
            .systems
            .iter()
            .map(|s| s.lowest_y)
            .fold(f32::INFINITY, f32::min);
        let high = caves
            .systems
            .iter()
            .map(|s| s.highest_y)
            .fold(f32::NEG_INFINITY, f32::max);
        let lo = chunk_of([-caves.safe_radius_m, low - 40.0, -caves.safe_radius_m]);
        let hi = chunk_of([caves.safe_radius_m, high + 40.0, caves.safe_radius_m]);
        let mut z = lo[2];
        while z <= hi[2] {
            let mut x = lo[0];
            while x <= hi[0] {
                let mut y = lo[1];
                while y <= hi[1] {
                    let block = caves.classify_block([x, y, z], 32.0);
                    counts[block as usize] += 1;
                    let mut signs = [false; 2];
                    for i in 0..=8 {
                        for j in 0..=8 {
                            for k in 0..=8 {
                                let p =
                                    [x + i as f32 * 4.0, y + j as f32 * 4.0, z + k as f32 * 4.0];
                                let v = caves.cave_value(p);
                                signs[usize::from(v < 0.0)] = true;
                                match block {
                                    Block::NoCave => assert!(
                                        v >= 0.0,
                                        "seed {seed}: air in a NoCave block at {p:?}"
                                    ),
                                    Block::CaveAir => assert!(
                                        v < 0.0,
                                        "seed {seed}: rock in a CaveAir block at {p:?}"
                                    ),
                                    Block::Mixed => {}
                                }
                            }
                        }
                    }
                    if block == Block::Mixed && signs[0] && signs[1] {
                        walls += 1;
                    }
                    y += 32.0;
                }
                x += 32.0;
            }
            z += 32.0;
        }
        println!(
            "seed {seed}: blocks of 32 m: {} with no cave, {} only cave air, {} to mesh ({walls} of them with a wall on a 4 m grid)",
            counts[0], counts[1], counts[2]
        );
        // A 4 m grid misses small walls, so the true part is higher.
        assert!(
            walls * 3 >= counts[2],
            "seed {seed}: the check keeps too many empty blocks"
        );
    }
}

#[test]
fn the_queries_answer_without_a_mesh() {
    for seed in SEEDS {
        let (zone, caves) = zone(seed);
        let ground = &zone.heights;
        let sys = &caves.systems[0];
        let hall_i = sys.chambers().next().unwrap();
        let hall = sys.nodes[hall_i];
        let inside = [
            hall.pos[0],
            hall.floor_y + hall.half_height_m * 0.5,
            hall.pos[2],
        ];
        assert!(
            caves.in_cave(ground, inside),
            "seed {seed}: the hall is not a cave"
        );
        assert_eq!(
            caves.chamber_at(inside),
            Some(ChamberRef {
                system: 0,
                node: hall_i
            })
        );
        let floor = caves.floor_below(ground, inside, 200.0).expect("a floor");
        let bumps = caves.params.noise_max_m + caves.params.smooth_m;
        assert!(
            (floor - hall.floor_y).abs() <= bumps,
            "seed {seed}: floor at {floor}, the plan says {}",
            hall.floor_y
        );
        let rock = [
            hall.pos[0],
            hall.pos[1] + hall.half_height_m + 30.0,
            hall.pos[2],
        ];
        if rock[1] < ground.sample_world(rock[0], rock[2]) {
            assert!(!caves.in_cave(ground, rock));
            assert_eq!(caves.sky_visibility(ground, rock), 0.0);
            assert_eq!(caves.floor_below(ground, rock, 10.0), None);
        }
        let sky = [
            inside[0],
            ground.sample_world(inside[0], inside[2]) + 2.0,
            inside[2],
        ];
        assert_eq!(caves.sky_visibility(ground, sky), 1.0);
        assert!(!caves.in_cave(ground, sky));

        // The sky visibility is high near the mouth and low deep down.
        let m = &sys.mouths[0];
        let entry = sys.edges.iter().find(|e| e.a == m.node).unwrap();
        let near = entry.path[1];
        let vis_near = caves.sky_visibility(ground, near);
        let deepest = sys
            .chambers()
            .map(|i| sys.nodes[i])
            .min_by(|a, b| a.floor_y.total_cmp(&b.floor_y))
            .unwrap();
        let deep = [deepest.pos[0], deepest.floor_y + 3.0, deepest.pos[2]];
        let vis_deep = caves.sky_visibility(ground, deep);
        println!(
            "seed {seed}: sky visibility {vis_near:.2} near the mouth, {:.3} in the hall, {vis_deep:.4} in the deepest chamber",
            caves.sky_visibility(ground, inside)
        );
        assert!(vis_near > 0.4, "seed {seed}: {vis_near}");
        assert!(vis_deep < 0.02, "seed {seed}: {vis_deep}");
    }
}

#[test]
fn each_mouth_makes_a_hole_in_the_ground() {
    for seed in SEEDS {
        let (zone, caves) = zone(seed);
        let mask = caves.hole_mask(&zone.heights, 1);
        let holes = mask.iter().filter(|&&m| m == 2).count();
        let band = mask.iter().filter(|&&m| m == 1).count();
        println!("seed {seed}: {holes} hole samples and {band} band samples (4 m samples)");
        let size = zone.heights.size;
        let half = size as f32 * 0.5;
        let sp = zone.heights.spacing;
        for m in caves.mouths() {
            let (cx, cz) = (m.pos[0] / sp + half, m.pos[2] / sp + half);
            let reach = (m.exempt_radius_m / sp).ceil() as isize + 1;
            let mut here = 0;
            for dz in -reach..=reach {
                for dx in -reach..=reach {
                    let (x, z) = (cx as isize + dx, cz as isize + dz);
                    if x >= 0
                        && z >= 0
                        && (x as usize) < size
                        && (z as usize) < size
                        && mask[z as usize * size + x as usize] == 2
                    {
                        here += 1;
                    }
                }
            }
            assert!(
                here >= 1,
                "seed {seed}: the {:?} mouth at {:?} has no hole",
                m.kind,
                m.pos
            );
        }
        assert!(band >= holes, "the band surrounds the holes");
    }
}

#[test]
fn the_costs_of_the_cave_layer() {
    // Pure-Rust costs only. The numbers come from the machine that runs
    // the tests (a busy laptop or a cloud container).
    let zone = generate(7, 512, 4.0, &HighlandsParams::default());
    let start = Instant::now();
    let caves = Caves::generate(&zone, &SmokeParams::default(), &CaveParams::default());
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    let points = probe_points(&caves, 50_000);
    let start = Instant::now();
    let mut sum = 0.0;
    for p in &points {
        sum += caves.cave_value(*p);
    }
    let ns = start.elapsed().as_secs_f64() * 1e9 / points.len() as f64;
    let hall = &caves.systems[0].nodes[caves.systems[0].chambers().next().unwrap()];
    let corner = chunk_of([hall.pos[0] + hall.radius_m, hall.floor_y, hall.pos[2]]);
    let mut out = Vec::new();
    let start = Instant::now();
    for _ in 0..5 {
        caves.fill_air(&zone.heights, corner, 1.0, [34, 34, 34], &mut out);
    }
    let chunk_ms = start.elapsed().as_secs_f64() * 1000.0 / 5.0;
    let start = Instant::now();
    let mask = caves.hole_mask(&zone.heights, 2);
    let mask_ms = start.elapsed().as_secs_f64() * 1000.0;
    println!(
        "cave costs: graph and grid {build_ms:.1} ms, {ns:.0} ns for each SDF sample, \
         {chunk_ms:.2} ms for one 34^3 chunk (one thread), hole mask {mask_ms:.1} ms ({} samples), checksum {sum:.1}",
        mask.len()
    );
    assert!(build_ms < 2000.0, "the cave build takes {build_ms} ms");
}

/// Run by hand: `cargo test --release -p bf-worldgen survey -- --ignored --nocapture`.
#[test]
#[ignore]
fn survey_seeds_1_to_50() {
    let mut near = 0;
    let mut systems = 0;
    let mut faults = 0;
    for seed in 1..=50 {
        let zone = generate(seed, 512, 4.0, &HighlandsParams::default());
        let caves = Caves::generate(&zone, &SmokeParams::default(), &CaveParams::default());
        let (sx, sz) = zone.start_point();
        let nearest = caves
            .mouths()
            .map(|m| ((m.pos[0] - sx).powi(2) + (m.pos[2] - sz).powi(2)).sqrt())
            .fold(f32::INFINITY, f32::min);
        if nearest <= 600.0 {
            near += 1;
        }
        systems += caves.systems.len();
        faults += caves.roof_faults;
        println!(
            "seed {seed}: {} systems, {} mouths, nearest mouth {nearest:.0} m, roof faults {}",
            caves.systems.len(),
            caves.mouths().count(),
            caves.roof_faults
        );
    }
    println!("{near} of 50 zones have a mouth within 600 m of the start; {systems} systems; {faults} roof faults");
}
