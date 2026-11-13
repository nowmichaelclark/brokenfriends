//! Where the water went: basins, flow and dry riverbeds.
//!
//! The world has no water now. But water made its shape. This module
//! finds the paths that rain takes down the terrain. Paths that collect
//! much rain become dry riverbeds.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// The 8 neighbours of a cell, and the distance to each one in cells.
const NEIGHBOURS: [(isize, isize, f32); 8] = [
    (1, 0, 1.0),
    (-1, 0, 1.0),
    (0, 1, 1.0),
    (0, -1, 1.0),
    (1, 1, std::f32::consts::SQRT_2),
    (-1, 1, std::f32::consts::SQRT_2),
    (1, -1, std::f32::consts::SQRT_2),
    (-1, -1, std::f32::consts::SQRT_2),
];

/// The flow of rain across a square grid of heights.
#[derive(Clone, Debug)]
pub struct Flow {
    /// For each cell, the index of the cell that its water goes to.
    /// A cell with no lower neighbour is a sink: it has its own index.
    pub receiver: Vec<u32>,
    /// For each cell, the number of cells that drain through it.
    pub area: Vec<f32>,
    /// All cell indexes, from the highest cell to the lowest cell.
    pub order: Vec<u32>,
}

/// Changes a height into a key that sorts in the same order.
#[inline]
fn sort_key(h: f32) -> i32 {
    let bits = h.to_bits() as i32;
    bits ^ (((bits >> 31) as u32) >> 1) as i32
}

/// Fills each closed basin to its rim and gives the new heights.
///
/// The border cells are the outlets. If `outlets` is given, each cell
/// that is `true` there is an outlet also. This is the priority-flood
/// method by Barnes. `epsilon` is a small rise for each cell in a filled
/// basin, thus water can flow across the basin to its outlet.
pub fn fill_basins(
    heights: &[f32],
    size: usize,
    epsilon: f32,
    outlets: Option<&[bool]>,
) -> Vec<f32> {
    let n = size * size;
    assert_eq!(heights.len(), n);
    let mut filled = heights.to_vec();
    let mut closed = vec![false; n];
    let mut open: BinaryHeap<Reverse<(i32, u32)>> = BinaryHeap::with_capacity(size * 8);

    for i in 0..size {
        for idx in [i, (size - 1) * size + i, i * size, i * size + size - 1] {
            if !closed[idx] {
                closed[idx] = true;
                open.push(Reverse((sort_key(filled[idx]), idx as u32)));
            }
        }
    }

    if let Some(outlets) = outlets {
        for (idx, _) in outlets.iter().enumerate().filter(|(_, &o)| o) {
            if !closed[idx] {
                closed[idx] = true;
                open.push(Reverse((sort_key(filled[idx]), idx as u32)));
            }
        }
    }

    let s = size as isize;
    while let Some(Reverse((_, idx))) = open.pop() {
        let (x, z) = (
            (idx as usize % size) as isize,
            (idx as usize / size) as isize,
        );
        let level = filled[idx as usize];
        for (dx, dz, _) in NEIGHBOURS {
            let (nx, nz) = (x + dx, z + dz);
            if nx < 0 || nz < 0 || nx >= s || nz >= s {
                continue;
            }
            let ni = (nz * s + nx) as usize;
            if closed[ni] {
                continue;
            }
            closed[ni] = true;
            if filled[ni] < level + epsilon {
                filled[ni] = level + epsilon;
            }
            open.push(Reverse((sort_key(filled[ni]), ni as u32)));
        }
    }
    filled
}

/// Calculates where the rain goes on a surface.
///
/// Each cell gives its water to the neighbour with the steepest drop.
/// A cell with no lower neighbour keeps its water (a sink). Border
/// cells are sinks also.
pub fn flow(surface: &[f32], size: usize) -> Flow {
    let n = size * size;
    assert_eq!(surface.len(), n);
    let s = size as isize;
    let mut receiver: Vec<u32> = (0..n as u32).collect();
    for z in 1..s - 1 {
        for x in 1..s - 1 {
            let i = (z * s + x) as usize;
            let mut best = 0.0;
            for (dx, dz, dist) in NEIGHBOURS {
                let ni = ((z + dz) * s + x + dx) as usize;
                let drop = (surface[i] - surface[ni]) / dist;
                if drop > best {
                    best = drop;
                    receiver[i] = ni as u32;
                }
            }
        }
    }

    // Go from the highest cell to the lowest cell, and give the water of
    // each cell to its receiver.
    let mut order: Vec<u32> = (0..n as u32).collect();
    order.sort_unstable_by_key(|&i| (Reverse(sort_key(surface[i as usize])), i));
    let mut area = vec![1.0f32; n];
    for &idx in &order {
        let r = receiver[idx as usize];
        if r != idx {
            area[r as usize] += area[idx as usize];
        }
    }
    Flow {
        receiver,
        area,
        order,
    }
}

/// Fills closed basins in part, and gives the depth of the fill for
/// each cell.
///
/// A basin that is less deep than `shallow` is filled to its rim. A
/// deeper basin keeps its shape: only the lowest part (`floor_part`,
/// from 0 to 1, of its depth) is filled. This flat floor is a dry lake
/// bed. The function also gives a mask that is `true` on lake beds.
pub fn partial_fill(
    heights: &[f32],
    size: usize,
    epsilon: f32,
    shallow: f32,
    floor_part: f32,
) -> (Vec<f32>, Vec<bool>) {
    let n = size * size;
    let mut surface = heights.to_vec();
    let mut lake = vec![false; n];
    let mut label = vec![u32::MAX; n];
    let mut stack: Vec<u32> = Vec::new();
    // A basin can have smaller basins inside it. Each pass finds the
    // basins that the pass before left open. The lake beds of the passes
    // before are outlets, thus a pass does not fill them again.
    for _ in 0..3 {
        let filled = fill_basins(&surface, size, epsilon, Some(&lake));
        label.fill(u32::MAX);
        let mut depths: Vec<f32> = Vec::new();
        for start in 0..n {
            if label[start] != u32::MAX || filled[start] - surface[start] <= 0.02 {
                continue;
            }
            // Find all cells of this basin, and its largest depth.
            let id = depths.len() as u32;
            let mut deepest = 0.0f32;
            label[start] = id;
            stack.push(start as u32);
            while let Some(i) = stack.pop() {
                let i = i as usize;
                deepest = deepest.max(filled[i] - surface[i]);
                let (x, z) = (i % size, i / size);
                let mut visit = |ni: usize| {
                    if label[ni] == u32::MAX && filled[ni] - surface[ni] > 0.02 {
                        label[ni] = id;
                        stack.push(ni as u32);
                    }
                };
                if x > 0 {
                    visit(i - 1);
                }
                if x + 1 < size {
                    visit(i + 1);
                }
                if z > 0 {
                    visit(i - size);
                }
                if z + 1 < size {
                    visit(i + size);
                }
            }
            depths.push(deepest);
        }
        if depths.is_empty() {
            break;
        }
        for i in 0..n {
            if label[i] == u32::MAX {
                // A tiny fill: keep it, it helps the flow.
                surface[i] = filled[i];
                continue;
            }
            let deepest = depths[label[i] as usize];
            if deepest < shallow {
                surface[i] = filled[i];
            } else {
                let level = filled[i] - deepest * (1.0 - floor_part);
                if level > surface[i] {
                    surface[i] = level;
                    lake[i] = true;
                }
            }
        }
    }
    let fill = surface.iter().zip(heights).map(|(s, h)| s - h).collect();
    (fill, lake)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(size: usize, f: impl Fn(f32, f32) -> f32) -> Vec<f32> {
        (0..size * size)
            .map(|i| f((i % size) as f32, (i / size) as f32))
            .collect()
    }

    #[test]
    fn a_pit_is_filled_to_its_rim() {
        // A cone with a deep pit in the centre.
        let size = 33;
        let h = grid(size, |x, z| {
            let d = ((x - 16.0).powi(2) + (z - 16.0).powi(2)).sqrt();
            if d < 3.0 {
                -50.0
            } else {
                40.0 - d
            }
        });
        let filled = fill_basins(&h, size, 1e-3, None);
        let d = flow(&filled, size);
        let centre = 16 * size + 16;
        // The lowest cell of the rim is at a distance of 4.24 cells.
        assert!(filled[centre] > 35.7, "pit level {}", filled[centre]);
        // Each cell that is not on the border has a lower receiver.
        for z in 1..size - 1 {
            for x in 1..size - 1 {
                let i = z * size + x;
                let r = d.receiver[i] as usize;
                assert!(r != i && filled[r] < filled[i]);
            }
        }
    }

    #[test]
    fn a_valley_collects_the_water() {
        // A V-shaped valley that goes down to the side z = 0.
        let size = 64;
        let h = grid(size, |x, z| (x - 32.0).abs() * 2.0 + z * 0.5);
        let d = flow(&h, size);
        let outlet = size + 32;
        assert!(d.area[outlet] > 1500.0, "area {}", d.area[outlet]);
        let total: f32 = (0..size * size)
            .filter(|&i| d.receiver[i] as usize == i)
            .map(|i| d.area[i])
            .sum();
        assert_eq!(total, (size * size) as f32, "no water is lost");
    }

    #[test]
    fn a_deep_basin_gets_a_flat_floor_only() {
        // A bowl of 30 m depth, and a small hole on its outer slope.
        let size = 65;
        let h = grid(size, |x, z| {
            let d = ((x - 32.0).powi(2) + (z - 32.0).powi(2)).sqrt();
            let bowl = if d < 20.0 { 20.0 + d * 1.5 } else { 70.0 - d };
            let hole = if (x - 55.0).abs() < 2.0 && (z - 32.0).abs() < 2.0 {
                4.0
            } else {
                0.0
            };
            bowl - hole
        });
        let (fill, lake) = partial_fill(&h, size, 1e-3, 5.0, 0.25);
        let centre = 32 * size + 32;
        assert!(lake[centre]);
        assert!((fill[centre] - 7.5).abs() < 0.5, "fill {}", fill[centre]);
        // The slope of the bowl is not filled.
        assert_eq!(fill[32 * size + 47], 0.0);
        // The small hole is filled to its rim, and it is not a lake.
        let hole = 32 * size + 55;
        assert!(fill[hole] > 1.5 && !lake[hole], "fill {}", fill[hole]);
    }
}
