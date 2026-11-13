//! The shape of the teddy bear, made in code from signed distance fields.
//!
//! A signed distance field (SDF) gives the distance from a point to the
//! nearest surface. The value is negative inside the shape. Each part
//! of the bear (body, head, ears, arms, legs) is a union of simple SDF
//! shapes: ellipsoids and round cones. `fast-surface-nets` changes each
//! group of shapes into a low-poly mesh. Seam rings and the thread of
//! the mouth are small tubes that this module makes directly.
//!
//! This module does not use Godot. All positions are in metres. The
//! origin is between the feet, Y is up and the bear looks along +Z.
//! The left side of the bear is +X.

use bf_worldgen::detmath;
use bf_worldgen::noise::hash01;
use fast_surface_nets::ndshape::{RuntimeShape, Shape as _};
use fast_surface_nets::{surface_nets, SurfaceNetsBuffer};
use glam::{Mat3, Vec3};

/// The parts of the bear. Each part is a separate mesh that can come off.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartId {
    Body,
    Head,
    LeftEar,
    RightEar,
    LeftArm,
    RightArm,
    LeftLeg,
    RightLeg,
}

impl PartId {
    /// All parts. A parent comes before its children.
    pub const ALL: [PartId; 8] = [
        PartId::Body,
        PartId::Head,
        PartId::LeftEar,
        PartId::RightEar,
        PartId::LeftArm,
        PartId::RightArm,
        PartId::LeftLeg,
        PartId::RightLeg,
    ];

    /// The name of the part. The Godot node of the part has this name.
    pub fn name(self) -> &'static str {
        match self {
            PartId::Body => "Body",
            PartId::Head => "Head",
            PartId::LeftEar => "LeftEar",
            PartId::RightEar => "RightEar",
            PartId::LeftArm => "LeftArm",
            PartId::RightArm => "RightArm",
            PartId::LeftLeg => "LeftLeg",
            PartId::RightLeg => "RightLeg",
        }
    }

    /// The part that this part is sewn onto.
    pub fn parent(self) -> Option<PartId> {
        match self {
            PartId::Body => None,
            PartId::LeftEar | PartId::RightEar => Some(PartId::Head),
            _ => Some(PartId::Body),
        }
    }

    /// Finds a part from its name.
    pub fn from_name(name: &str) -> Option<PartId> {
        PartId::ALL.into_iter().find(|p| p.name() == name)
    }
}

/// The look of a part of the surface. The mesh stores it in the vertex
/// colour, and the fur shader reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// 1 is mohair fur, 0 is a smooth surface.
    pub fur: f32,
    /// 1 is the pale colour of the felt pads.
    pub pale: f32,
    /// 1 is the dark colour of the thread and the buttons.
    pub dark: f32,
    /// 1 is a glossy surface (the button eyes).
    pub gloss: f32,
}

impl Look {
    pub const FUR: Look = Look::new(1.0, 0.0, 0.0, 0.0);
    /// The snout has shorter, paler mohair.
    pub const SNOUT: Look = Look::new(1.0, 0.3, 0.0, 0.0);
    pub const FELT: Look = Look::new(0.0, 1.0, 0.0, 0.0);
    pub const BUTTON: Look = Look::new(0.0, 0.0, 1.0, 1.0);
    pub const THREAD: Look = Look::new(0.0, 0.0, 1.0, 0.25);

    pub const fn new(fur: f32, pale: f32, dark: f32, gloss: f32) -> Look {
        Look {
            fur,
            pale,
            dark,
            gloss,
        }
    }

    /// The look as four numbers (red, green, blue, alpha of the vertex colour).
    pub fn to_array(self) -> [f32; 4] {
        [self.fur, self.pale, self.dark, self.gloss]
    }
}

/// One simple SDF shape.
#[derive(Clone, Copy, Debug)]
pub enum Sdf {
    /// An ellipsoid. The columns of `basis` are its axes.
    Ellipsoid {
        centre: Vec3,
        radii: Vec3,
        basis: Mat3,
    },
    /// A capsule whose radius changes from `ra` at `a` to `rb` at `b`.
    Cone { a: Vec3, b: Vec3, ra: f32, rb: f32 },
}

impl Sdf {
    pub fn ellipsoid(centre: Vec3, radii: Vec3) -> Sdf {
        Sdf::Ellipsoid {
            centre,
            radii,
            basis: Mat3::IDENTITY,
        }
    }

    /// An ellipsoid whose Z axis points along `facing`.
    pub fn ellipsoid_facing(centre: Vec3, radii: Vec3, facing: Vec3) -> Sdf {
        Sdf::Ellipsoid {
            centre,
            radii,
            basis: basis_facing(facing),
        }
    }

    pub fn cone(a: Vec3, b: Vec3, ra: f32, rb: f32) -> Sdf {
        Sdf::Cone { a, b, ra, rb }
    }

    /// The distance from `p` to the surface. Negative inside.
    pub fn distance(&self, p: Vec3) -> f32 {
        match *self {
            Sdf::Ellipsoid {
                centre,
                radii,
                basis,
            } => {
                // Inigo Quilez's ellipsoid bound.
                let q = basis.transpose() * (p - centre);
                let k0 = (q / radii).length();
                let k1 = (q / (radii * radii)).length();
                if k1 < 1e-9 {
                    -radii.min_element()
                } else {
                    k0 * (k0 - 1.0) / k1
                }
            }
            Sdf::Cone { a, b, ra, rb } => round_cone(p, a, b, ra, rb),
        }
    }

    /// The smallest radius. The mesh cell must be smaller than this.
    fn smallest_radius(&self) -> f32 {
        match *self {
            Sdf::Ellipsoid { radii, .. } => radii.min_element(),
            Sdf::Cone { ra, rb, .. } => ra.min(rb),
        }
    }

    /// A box that holds the shape: (min, max).
    fn bounds(&self) -> (Vec3, Vec3) {
        match *self {
            Sdf::Ellipsoid { centre, radii, .. } => {
                let r = Vec3::splat(radii.max_element());
                (centre - r, centre + r)
            }
            Sdf::Cone { a, b, ra, rb } => {
                let r = Vec3::splat(ra.max(rb));
                (a.min(b) - r, a.max(b) + r)
            }
        }
    }
}

/// Inigo Quilez's exact round cone.
fn round_cone(p: Vec3, a: Vec3, b: Vec3, r1: f32, r2: f32) -> f32 {
    let sign = |x: f32| {
        if x > 0.0 {
            1.0
        } else if x < 0.0 {
            -1.0
        } else {
            0.0
        }
    };
    let ba = b - a;
    let l2 = ba.dot(ba);
    if l2 < 1e-12 {
        return (p - a).length() - r1.max(r2);
    }
    let rr = r1 - r2;
    let a2 = l2 - rr * rr;
    let il2 = 1.0 / l2;
    let pa = p - a;
    let y = pa.dot(ba);
    let z = y - l2;
    let x2 = (pa * l2 - ba * y).length_squared();
    let y2 = y * y * l2;
    let z2 = z * z * l2;
    let k = sign(rr) * rr * rr * x2;
    if sign(z) * a2 * z2 > k {
        return (x2 + z2).sqrt() * il2 - r2;
    }
    if sign(y) * a2 * y2 < k {
        return (x2 + y2).sqrt() * il2 - r1;
    }
    ((x2 * a2 * il2).sqrt() + y * rr) * il2 - r1
}

/// A smooth union of two distances. `k` is the size of the blend in metres.
fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    if k <= 0.0 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// A smooth cut: the shape `a` without the shape `b`.
fn smooth_cut(a: f32, b: f32, k: f32) -> f32 {
    -smooth_min(-a, b, k)
}

/// An orthonormal basis whose Z axis is `facing`.
fn basis_facing(facing: Vec3) -> Mat3 {
    let z = facing.normalize_or(Vec3::Z);
    let helper = if z.y.abs() > 0.9 { Vec3::X } else { Vec3::Y };
    let x = helper.cross(z).normalize();
    let y = z.cross(x);
    Mat3::from_cols(x, y, z)
}

/// A group of shapes that becomes one closed mesh.
#[derive(Clone, Debug)]
pub struct Piece {
    /// The shapes and their looks. The piece is their smooth union.
    pub shapes: Vec<(Sdf, Look)>,
    /// Shapes that are cut out of the union.
    pub cuts: Vec<Sdf>,
    /// The size of the smooth union and the cuts, in metres.
    pub blend: f32,
}

impl Piece {
    fn new(blend: f32) -> Piece {
        Piece {
            shapes: Vec::new(),
            cuts: Vec::new(),
            blend,
        }
    }

    fn with(mut self, sdf: Sdf, look: Look) -> Piece {
        self.shapes.push((sdf, look));
        self
    }

    /// The distance from `p` to the surface of the piece.
    pub fn distance(&self, p: Vec3) -> f32 {
        let mut d = f32::MAX;
        for (sdf, _) in &self.shapes {
            d = smooth_min(d, sdf.distance(p), self.blend);
        }
        for cut in &self.cuts {
            d = smooth_cut(d, cut.distance(p), self.blend * 0.5);
        }
        d
    }

    /// The look at `p`: the look of the nearest shape.
    fn look_at(&self, p: Vec3) -> Look {
        let mut best = (f32::MAX, Look::FUR);
        for (sdf, look) in &self.shapes {
            let d = sdf.distance(p);
            if d < best.0 {
                best = (d, *look);
            }
        }
        best.1
    }
}

/// A tube along a path of points, for thread and seams.
#[derive(Clone, Debug)]
pub struct Tube {
    /// The centre line. The tube is closed if `ring` is true.
    pub points: Vec<Vec3>,
    pub radius: f32,
    pub ring: bool,
    /// The look at each point. The seam stitches alternate.
    pub looks: Vec<Look>,
}

/// The ring where a part is sewn onto its parent. A tear starts here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seam {
    /// The centre of the ring, in the space of the part.
    pub centre: Vec3,
    /// The axis of the ring, away from the parent.
    pub axis: Vec3,
    /// The mean radius of the ring in metres.
    pub radius: f32,
}

/// The sizes of a bear. Later friends can use other values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recipe {
    /// The height from the soles to the top of the ears, in metres.
    pub height: f32,
    /// The size of the head (1 is the classic size).
    pub head: f32,
    /// The length of the snout in front of the head, in metres (at 0.40 m).
    pub snout_length: f32,
    /// The length of an arm from the shoulder to the paw, in metres (at 0.40 m).
    pub arm_length: f32,
    /// The length of a leg from the hip to the ankle, in metres (at 0.40 m).
    pub leg_length: f32,
    /// The size of the hump on the back (1 is the classic size).
    pub hump: f32,
    /// The size of the mesh cells on the fur parts, in metres.
    pub cell: f32,
}

impl Default for Recipe {
    fn default() -> Recipe {
        Recipe {
            height: 0.40,
            head: 1.0,
            snout_length: 0.05,
            arm_length: 0.18,
            leg_length: 0.118,
            hump: 1.0,
            cell: 0.011,
        }
    }
}

/// The definition of one part, before meshing.
#[derive(Clone, Debug)]
pub struct PartDef {
    pub id: PartId,
    /// The joint of the part, in bear space. The part turns around it.
    pub joint: Vec3,
    pub pieces: Vec<Piece>,
    pub tubes: Vec<Tube>,
    /// The seam ring, in the space of the part (relative to the joint).
    pub seam: Option<Seam>,
}

/// The mesh of one part, ready for Godot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PartMesh {
    /// Positions relative to the joint of the part.
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    /// The look of each vertex: fur, pale, dark, gloss.
    pub looks: Vec<[f32; 4]>,
    /// Three indices for each triangle, clockwise seen from outside
    /// (the front face in Godot).
    pub indices: Vec<u32>,
}

impl PartMesh {
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    fn append(&mut self, other: PartMesh) {
        let offset = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.looks.extend(other.looks);
        self.indices
            .extend(other.indices.iter().map(|i| i + offset));
    }
}

/// A finished part: its joint, its mesh and its seam.
#[derive(Clone, Debug)]
pub struct Part {
    pub id: PartId,
    /// The joint in bear space.
    pub joint: Vec3,
    pub mesh: PartMesh,
    pub seam: Option<Seam>,
}

/// Finds the surface along a ray that starts inside the shape.
fn surface_along(distance: &dyn Fn(Vec3) -> f32, from: Vec3, dir: Vec3) -> Vec3 {
    let dir = dir.normalize_or(Vec3::Y);
    let (mut inside, mut outside) = (0.0_f32, 0.25_f32);
    for _ in 0..40 {
        let mid = 0.5 * (inside + outside);
        if distance(from + dir * mid) < 0.0 {
            inside = mid;
        } else {
            outside = mid;
        }
    }
    from + dir * outside
}

/// The outward normal of a field at `p`, from central differences.
fn gradient(distance: &dyn Fn(Vec3) -> f32, p: Vec3) -> Vec3 {
    let e = 1e-4;
    Vec3::new(
        distance(p + Vec3::X * e) - distance(p - Vec3::X * e),
        distance(p + Vec3::Y * e) - distance(p - Vec3::Y * e),
        distance(p + Vec3::Z * e) - distance(p - Vec3::Z * e),
    )
    .normalize_or(Vec3::Y)
}

/// Moves a point onto the surface of a field.
fn project(distance: &dyn Fn(Vec3) -> f32, mut p: Vec3) -> Vec3 {
    for _ in 0..6 {
        p -= gradient(distance, p) * distance(p);
    }
    p
}

/// Makes the definitions of all parts from a recipe.
pub fn define(recipe: &Recipe) -> Vec<PartDef> {
    let r = recipe;
    let v = Vec3::new;
    let head = r.head;

    // Heights at the classic size of 0.40 m. All are scaled at the end.
    let ankle_y = 0.026;
    let hip_y = ankle_y + r.leg_length;
    let neck = v(0.0, hip_y + 0.142, 0.004);
    let head_c = neck + v(0.0, 0.046 * head, 0.006 * head);

    let mut parts = Vec::new();

    // The body: a pear-shaped belly, a long chest and a hump on the back.
    let body_joint = v(0.0, hip_y + 0.06, 0.0);
    let body = Piece::new(0.04)
        .with(
            Sdf::ellipsoid(v(0.0, hip_y + 0.022, 0.006), v(0.070, 0.066, 0.064)),
            Look::FUR,
        )
        .with(
            Sdf::ellipsoid(v(0.0, hip_y + 0.088, 0.0), v(0.060, 0.068, 0.054)),
            Look::FUR,
        )
        .with(
            Sdf::ellipsoid(
                v(0.0, hip_y + 0.100, -0.030),
                v(0.050, 0.048, 0.044) * r.hump,
            ),
            Look::FUR,
        );
    parts.push(PartDef {
        id: PartId::Body,
        joint: body_joint,
        pieces: vec![body],
        tubes: Vec::new(),
        seam: None,
    });

    // The head with a long snout, a stitched nose and mouth, and button eyes.
    let snout_a = head_c + v(0.0, -0.012, 0.024) * head;
    let snout_b = head_c + v(0.0, -0.017, 0.024 + r.snout_length) * head;
    let snout_rb = 0.019 * head;
    let skull = Piece::new(0.022 * head)
        .with(
            Sdf::ellipsoid(head_c, v(0.056, 0.051, 0.051) * head),
            Look::FUR,
        )
        .with(
            Sdf::cone(snout_a, snout_b, 0.031 * head, snout_rb),
            Look::SNOUT,
        );
    let skull_d = |p: Vec3| skull.distance(p);
    let mut head_pieces = Vec::new();
    for side in [-1.0_f32, 1.0] {
        let at = surface_along(&skull_d, head_c, v(0.43 * side, 0.30, 0.85));
        let normal = gradient(&skull_d, at);
        let eye = Sdf::ellipsoid_facing(
            at - normal * 0.0015,
            v(0.0065, 0.0065, 0.0042) * head,
            normal,
        );
        head_pieces.push(Piece::new(0.0).with(eye, Look::BUTTON));
    }
    let tip = surface_along(&skull_d, snout_b, v(0.0, 0.25, 1.0));
    let tip_normal = gradient(&skull_d, tip);
    let nose = Sdf::ellipsoid_facing(
        tip - tip_normal * 0.0025,
        v(0.0135, 0.0095, 0.0065) * head,
        tip_normal,
    );
    head_pieces.push(Piece::new(0.0).with(nose, Look::THREAD));

    // The mouth: an upside-down Y of thread under the nose.
    let rb = snout_rb;
    let mouth_mid = snout_b + v(0.0, -0.95 * rb, 0.35 * rb);
    let nose_bottom = snout_b + v(0.0, -0.40 * rb, 0.92 * rb);
    let mut tubes = Vec::new();
    let thread = |points: &[Vec3]| {
        let mut path = Vec::new();
        for pair in points.windows(2) {
            for i in 0..4 {
                path.push(pair[0].lerp(pair[1], i as f32 / 4.0));
            }
        }
        path.push(*points.last().unwrap_or(&Vec3::ZERO));
        let path: Vec<Vec3> = path
            .into_iter()
            .map(|p| {
                let on = project(&skull_d, p);
                on + gradient(&skull_d, on) * 0.0006
            })
            .collect();
        Tube {
            looks: vec![Look::THREAD; path.len()],
            points: path,
            radius: 0.0014 * head,
            ring: false,
        }
    };
    tubes.push(thread(&[nose_bottom, mouth_mid]));
    for side in [-1.0_f32, 1.0] {
        let corner = snout_b + v(0.62 * rb * side, -0.80 * rb, -0.30 * rb);
        tubes.push(thread(&[mouth_mid, corner]));
    }
    let head_seam_centre = neck + v(0.0, 0.010, 0.0);
    let head_seam = seam_ring(&skull_d, head_seam_centre, Vec3::Y, 0.0019);
    tubes.push(head_seam.0);
    head_pieces.insert(0, skull.clone());
    parts.push(PartDef {
        id: PartId::Head,
        joint: neck,
        pieces: head_pieces,
        tubes,
        seam: Some(Seam {
            centre: head_seam_centre - neck,
            ..head_seam.1
        }),
    });

    // The ears: round, a little cupped, sewn on at the top of the head.
    for (id, side) in [(PartId::LeftEar, 1.0_f32), (PartId::RightEar, -1.0)] {
        let base = surface_along(&skull_d, head_c, v(0.70 * side, 0.70, -0.12));
        let out = gradient(&skull_d, base);
        let centre = base + out * (0.012 * head) + v(0.0, 0.004, 0.0) * head;
        let mut ear = Piece::new(0.006 * head).with(
            Sdf::ellipsoid_facing(
                centre,
                v(0.025, 0.023, 0.012) * head,
                v(0.3 * side, 0.1, 1.0),
            ),
            Look::FUR,
        );
        ear.cuts.push(Sdf::ellipsoid(
            centre + v(0.005 * side, 0.004, 0.022) * head,
            Vec3::splat(0.019) * head,
        ));
        parts.push(PartDef {
            id,
            joint: base,
            pieces: vec![ear],
            tubes: Vec::new(),
            seam: Some(Seam {
                centre: Vec3::ZERO,
                axis: out,
                radius: 0.018 * head,
            }),
        });
    }

    // The long arms, with paws that curve forward and felt pads.
    let arm_k = r.arm_length / 0.15;
    for (id, side) in [(PartId::LeftArm, 1.0_f32), (PartId::RightArm, -1.0)] {
        let shoulder = v(0.056 * side, hip_y + 0.117, 0.0);
        let s = |x: f32, y: f32, z: f32| shoulder + v(x * side, y * arm_k, z);
        let elbow = s(0.016, -0.072, 0.010);
        let wrist = s(0.022, -0.118, 0.026);
        let paw_c = s(0.023, -0.124, 0.031);
        let arm = Piece::new(0.016)
            .with(
                Sdf::cone(s(0.006, -0.004, 0.0), elbow, 0.024, 0.020),
                Look::FUR,
            )
            .with(Sdf::cone(elbow, wrist, 0.020, 0.019), Look::FUR)
            .with(
                Sdf::ellipsoid_facing(paw_c, v(0.020, 0.019, 0.027), v(0.0, 1.0, -0.3)),
                Look::FUR,
            );
        let arm_d = |p: Vec3| arm.distance(p);
        let pad_dir = v(-0.75 * side, -0.25, 0.62);
        let pad_at = surface_along(&arm_d, paw_c, pad_dir);
        let pad_normal = gradient(&arm_d, pad_at);
        let pad_radii = v(0.014, 0.019, 0.008);
        let pad = Piece::new(0.0).with(
            Sdf::ellipsoid_facing(
                pad_at - pad_normal * (pad_radii.z - 0.0022),
                pad_radii,
                pad_normal,
            ),
            Look::FELT,
        );
        let axis = (shoulder - elbow).normalize();
        let seam_centre = shoulder + (elbow - shoulder) * 0.22 + v(0.006 * side, 0.0, 0.0);
        let (ring, seam) = seam_ring(&arm_d, seam_centre, axis, 0.0019);
        parts.push(PartDef {
            id,
            joint: shoulder,
            pieces: vec![arm, pad],
            tubes: vec![ring],
            seam: Some(Seam {
                centre: seam_centre - shoulder,
                ..seam
            }),
        });
    }

    // The long, thin legs with big feet and felt soles.
    for (id, side) in [(PartId::LeftLeg, 1.0_f32), (PartId::RightLeg, -1.0)] {
        let hip = v(0.040 * side, hip_y, 0.0);
        let ankle = v(0.042 * side, ankle_y + 0.006, 0.0);
        let foot_c = v(0.042 * side, ankle_y - 0.004, 0.024);
        let foot_radii = v(0.026, 0.022, 0.054);
        let leg = Piece::new(0.018)
            .with(
                Sdf::cone(hip + v(0.0, 0.010, 0.0), ankle, 0.026, 0.020),
                Look::FUR,
            )
            .with(Sdf::ellipsoid(foot_c, foot_radii), Look::FUR);
        let leg_d = |p: Vec3| leg.distance(p);
        let sole = surface_along(&leg_d, foot_c, v(0.0, -1.0, 0.0));
        let pad_radii = v(0.019, 0.043, 0.008);
        let pad = Piece::new(0.0).with(
            Sdf::Ellipsoid {
                centre: sole + Vec3::Y * (pad_radii.z - 0.0022),
                radii: pad_radii,
                // The thin Z axis of the pad points down.
                basis: Mat3::from_cols(Vec3::X, Vec3::Z, -Vec3::Y),
            },
            Look::FELT,
        );
        let seam_centre = hip + v(0.0, -0.030, 0.0);
        let (ring, seam) = seam_ring(&leg_d, seam_centre, -Vec3::Y, 0.0019);
        parts.push(PartDef {
            id,
            joint: hip,
            pieces: vec![leg, pad],
            tubes: vec![ring],
            seam: Some(Seam {
                centre: seam_centre - hip,
                ..seam
            }),
        });
    }

    // Scale all parts to the height of the recipe.
    let k = r.height / 0.40;
    if (k - 1.0).abs() > 1e-6 {
        for part in &mut parts {
            scale_part(part, k);
        }
    }
    parts
}

/// The number of segments of a seam ring.
const SEAM_SEGMENTS: usize = 24;

/// Makes a seam ring that follows the surface of a field. The ring is
/// in the plane through `centre` at right angles to `axis`.
fn seam_ring(
    distance: &dyn Fn(Vec3) -> f32,
    centre: Vec3,
    axis: Vec3,
    radius: f32,
) -> (Tube, Seam) {
    let basis = basis_facing(axis);
    let mut points = Vec::with_capacity(SEAM_SEGMENTS);
    let mut looks = Vec::with_capacity(SEAM_SEGMENTS);
    let mut mean = 0.0;
    for i in 0..SEAM_SEGMENTS {
        let angle = i as f32 / SEAM_SEGMENTS as f32 * std::f32::consts::TAU;
        let (sin, cos) = detmath::sin_cos(angle);
        let dir = basis.x_axis * cos + basis.y_axis * sin;
        let on = surface_along(distance, centre, dir);
        mean += (on - centre).length();
        points.push(on + dir * (radius * 0.4));
        // The stitches: light and dark thread one after the other.
        let dark = if i % 2 == 0 { 0.45 } else { 0.7 };
        looks.push(Look::new(0.0, 0.0, dark, 0.1));
    }
    let tube = Tube {
        points,
        radius,
        ring: true,
        looks,
    };
    let seam = Seam {
        centre,
        axis: basis.z_axis,
        radius: mean / SEAM_SEGMENTS as f32,
    };
    (tube, seam)
}

fn scale_part(part: &mut PartDef, k: f32) {
    part.joint *= k;
    for piece in &mut part.pieces {
        piece.blend *= k;
        for (sdf, _) in &mut piece.shapes {
            scale_sdf(sdf, k);
        }
        for sdf in &mut piece.cuts {
            scale_sdf(sdf, k);
        }
    }
    for tube in &mut part.tubes {
        tube.radius *= k;
        for p in &mut tube.points {
            *p *= k;
        }
    }
    if let Some(seam) = &mut part.seam {
        seam.centre *= k;
        seam.radius *= k;
    }
}

fn scale_sdf(sdf: &mut Sdf, k: f32) {
    match sdf {
        Sdf::Ellipsoid { centre, radii, .. } => {
            *centre *= k;
            *radii *= k;
        }
        Sdf::Cone { a, b, ra, rb } => {
            *a *= k;
            *b *= k;
            *ra *= k;
            *rb *= k;
        }
    }
}

/// The largest grid side of one piece. It limits the time and the triangles.
const MAX_GRID: u32 = 96;

/// Meshes one piece with Surface Nets. The positions are in bear space.
fn mesh_piece(piece: &Piece, base_cell: f32) -> PartMesh {
    let smallest = piece
        .shapes
        .iter()
        .map(|(s, _)| s.smallest_radius())
        .fold(f32::MAX, f32::min);
    // At least 1.8 cells in the smallest radius, so the shape does not vanish.
    let mut cell = base_cell.min(smallest / 1.8).max(0.0008);
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    for (sdf, _) in &piece.shapes {
        let (a, b) = sdf.bounds();
        lo = lo.min(a);
        hi = hi.max(b);
    }
    let size = hi - lo;
    if size.max_element() / cell > (MAX_GRID - 4) as f32 {
        cell = size.max_element() / (MAX_GRID - 4) as f32;
    }
    // Two empty cells on each side keep the mesh closed.
    let lo = lo - Vec3::splat(cell * 2.0);
    let dims = ((size / cell).ceil() + 5.0)
        .as_uvec3()
        .min(glam::UVec3::splat(MAX_GRID));
    let shape = RuntimeShape::<u32, 3>::new([dims.x, dims.y, dims.z]);
    let mut field = vec![1.0_f32; shape.usize()];
    for (i, value) in field.iter_mut().enumerate() {
        let [x, y, z] = shape.delinearize(i as u32);
        let p = lo + Vec3::new(x as f32, y as f32, z as f32) * cell;
        // Surface Nets works in cells, so the field is in cells too.
        *value = piece.distance(p) / cell;
    }
    let mut buffer = SurfaceNetsBuffer::default();
    surface_nets(
        &field,
        &shape,
        [0; 3],
        [dims.x - 1, dims.y - 1, dims.z - 1],
        &mut buffer,
    );

    let mut mesh = PartMesh::default();
    for (p, n) in buffer.positions.iter().zip(&buffer.normals) {
        let world = lo + Vec3::from_array(*p) * cell;
        mesh.positions.push(world.to_array());
        mesh.normals
            .push(Vec3::from_array(*n).normalize_or(Vec3::Y).to_array());
        mesh.looks.push(piece.look_at(world).to_array());
    }
    // Surface Nets gives counter-clockwise triangles. Godot wants
    // clockwise front faces, so swap two corners of each triangle.
    for tri in buffer.indices.chunks_exact(3) {
        mesh.indices.extend([tri[0], tri[2], tri[1]]);
    }
    mesh
}

/// The number of sides of a tube.
const TUBE_SIDES: usize = 5;

/// Makes the mesh of a tube. The positions are in bear space.
fn mesh_tube(tube: &Tube) -> PartMesh {
    let mut mesh = PartMesh::default();
    let n = tube.points.len();
    if n < 2 {
        return mesh;
    }
    let mut ring_starts = Vec::with_capacity(n);
    for i in 0..n {
        let prev = if i > 0 {
            tube.points[i - 1]
        } else if tube.ring {
            tube.points[n - 1]
        } else {
            tube.points[i]
        };
        let next = if i + 1 < n {
            tube.points[i + 1]
        } else if tube.ring {
            tube.points[0]
        } else {
            tube.points[i]
        };
        let basis = basis_facing(next - prev);
        ring_starts.push(mesh.positions.len() as u32);
        for s in 0..TUBE_SIDES {
            let angle = s as f32 / TUBE_SIDES as f32 * std::f32::consts::TAU;
            let (sin, cos) = detmath::sin_cos(angle);
            let dir = basis.x_axis * cos + basis.y_axis * sin;
            mesh.positions
                .push((tube.points[i] + dir * tube.radius).to_array());
            mesh.normals.push(dir.to_array());
            mesh.looks.push(tube.looks[i].to_array());
        }
    }
    let segments = if tube.ring { n } else { n - 1 };
    let sides = TUBE_SIDES as u32;
    for i in 0..segments {
        let a = ring_starts[i];
        let b = ring_starts[(i + 1) % n];
        for s in 0..sides {
            let t = (s + 1) % sides;
            // Clockwise seen from outside.
            mesh.indices
                .extend([a + s, b + s, a + t, b + s, b + t, a + t]);
        }
    }
    if !tube.ring {
        // Close the two ends with a fan.
        for (start, flip) in [(ring_starts[0], true), (ring_starts[n - 1], false)] {
            for s in 1..sides - 1 {
                if flip {
                    mesh.indices.extend([start, start + s, start + s + 1]);
                } else {
                    mesh.indices.extend([start, start + s + 1, start + s]);
                }
            }
        }
    }
    mesh
}

/// Removes the triangles that are deep inside an other piece of the
/// same part. Nobody sees them. Also removes the unused vertices.
fn cull_hidden(mesh: PartMesh, others: &[&Piece], margin: f32) -> PartMesh {
    if others.is_empty() {
        return mesh;
    }
    let hidden: Vec<bool> = mesh
        .positions
        .iter()
        .map(|p| {
            let p = Vec3::from_array(*p);
            others.iter().any(|o| o.distance(p) < -margin)
        })
        .collect();
    let mut out = PartMesh::default();
    let mut remap = vec![u32::MAX; mesh.positions.len()];
    for tri in mesh.indices.chunks_exact(3) {
        if tri.iter().all(|&i| hidden[i as usize]) {
            continue;
        }
        for &i in tri {
            let i = i as usize;
            if remap[i] == u32::MAX {
                remap[i] = out.positions.len() as u32;
                out.positions.push(mesh.positions[i]);
                out.normals.push(mesh.normals[i]);
                out.looks.push(mesh.looks[i]);
            }
            out.indices.push(remap[i]);
        }
    }
    out
}

/// Meshes one part. The positions of the result are relative to the joint.
pub fn mesh_part(def: &PartDef, cell: f32) -> Part {
    let mut mesh = PartMesh::default();
    for (i, piece) in def.pieces.iter().enumerate() {
        let others: Vec<&Piece> = def
            .pieces
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != i)
            .map(|(_, p)| p)
            .collect();
        let piece_mesh = mesh_piece(piece, cell);
        mesh.append(cull_hidden(piece_mesh, &others, 0.0015));
    }
    for tube in &def.tubes {
        mesh.append(mesh_tube(tube));
    }
    for p in &mut mesh.positions {
        *p = (Vec3::from_array(*p) - def.joint).to_array();
    }
    Part {
        id: def.id,
        joint: def.joint,
        mesh,
        seam: def.seam,
    }
}

/// Makes all parts of a bear.
pub fn build(recipe: &Recipe) -> Vec<Part> {
    let cell = recipe.cell * recipe.height / 0.40;
    define(recipe)
        .iter()
        .map(|def| mesh_part(def, cell))
        .collect()
}

/// Makes the fur texture: short, uneven mohair fibres that tile.
///
/// The result has `size * size` pixels with four bytes each:
/// - red: the brightness of the fibres
/// - green: tufts, a little larger than the fibres
/// - blue: worn places, where the fur is thin
/// - alpha: the gaps between the fibres (dark is a gap)
pub fn fur_texture(size: usize, seed: u32) -> Vec<u8> {
    let n = size.max(8);
    let mut fibres = vec![0.35_f32; n * n];
    let mut gaps = vec![0.0_f32; n * n];
    let wrap = |x: i32| x.rem_euclid(n as i32) as usize;
    let count = n * n / 10;
    for f in 0..count as i32 {
        let rand = |k: i32| hash01(f, k, seed);
        let (x0, y0) = (rand(0) * n as f32, rand(1) * n as f32);
        // The fibres lie mostly in one direction, as on a worn bear.
        let angle = 1.1 + (rand(2) - 0.5) * 1.3;
        let (sin, cos) = detmath::sin_cos(angle);
        let length = 4.0 + rand(3) * 8.0;
        let bright = 0.55 + rand(4) * 0.45;
        let steps = length as i32 * 2;
        for s in 0..=steps {
            let t = s as f32 / steps as f32;
            let (x, y) = (x0 + cos * length * t, y0 + sin * length * t);
            let i = wrap(y as i32) * n + wrap(x as i32);
            // The fibre is a little darker at its root.
            let v = bright * (0.75 + 0.25 * t);
            fibres[i] = fibres[i].max(v);
            gaps[i] = 1.0;
        }
    }
    // Tileable value noise for the tufts and the worn places.
    let value_noise = |x: usize, y: usize, cells: usize, salt: u32| {
        let c = (n / cells).max(1) as f32;
        let (fx, fy) = (x as f32 / c, y as f32 / c);
        let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
        let (tx, ty) = (fx - ix as f32, fy - iy as f32);
        let (sx, sy) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let m = cells as i32;
        let h = |a: i32, b: i32| hash01(a.rem_euclid(m), b.rem_euclid(m), seed ^ salt);
        let top = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
        let bottom = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
        top + (bottom - top) * sy
    };
    let mut out = Vec::with_capacity(n * n * 4);
    for y in 0..n {
        for x in 0..n {
            let i = y * n + x;
            let tuft = value_noise(x, y, 16, 0x7f4a_7c15);
            let worn = detmath::smoothstep(0.55, 0.8, value_noise(x, y, 4, 0x2c1b_3c6d));
            let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            out.extend([
                byte(fibres[i]),
                byte(tuft),
                byte(worn),
                byte(0.4 + 0.6 * gaps[i]),
            ]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds(parts: &[Part]) -> (Vec3, Vec3) {
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for part in parts {
            for p in &part.mesh.positions {
                let p = Vec3::from_array(*p) + part.joint;
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        (lo, hi)
    }

    #[test]
    fn the_bear_is_about_40_cm_tall() {
        let parts = build(&Recipe::default());
        let (lo, hi) = bounds(&parts);
        let height = hi.y - lo.y;
        assert!((0.37..=0.42).contains(&height), "height {height}");
        assert!(lo.y > -0.01, "the soles are near the ground: {}", lo.y);
        // The bear is symmetric from left to right.
        assert!((lo.x + hi.x).abs() < 0.005, "{lo} {hi}");
    }

    #[test]
    fn each_part_has_a_mesh_and_a_small_triangle_count() {
        let parts = build(&Recipe::default());
        assert_eq!(parts.len(), PartId::ALL.len());
        let mut total = 0;
        for part in &parts {
            let m = &part.mesh;
            assert!(m.triangles() > 20, "{:?}", part.id);
            assert_eq!(m.positions.len(), m.normals.len());
            assert_eq!(m.positions.len(), m.looks.len());
            assert!(m.indices.iter().all(|&i| (i as usize) < m.positions.len()));
            total += m.triangles();
        }
        // The R15 detail budget: a few thousand triangles for the player.
        assert!(total < 12_000, "{total} triangles");
    }

    #[test]
    fn the_triangles_face_outward_in_godot_order() {
        let parts = build(&Recipe::default());
        for part in &parts {
            let m = &part.mesh;
            let mut agree = 0;
            for tri in m.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|k| Vec3::from_array(m.positions[tri[k] as usize]));
                let n = Vec3::from_array(m.normals[tri[0] as usize]);
                // Clockwise seen from outside: the cross product points in.
                if (b - a).cross(c - a).dot(n) < 0.0 {
                    agree += 1;
                }
            }
            let part_agree = agree as f32 / m.triangles() as f32;
            assert!(part_agree > 0.97, "{:?}: {part_agree}", part.id);
        }
    }

    #[test]
    fn the_arms_reach_past_the_hips() {
        let parts = build(&Recipe::default());
        let lowest = |id: PartId| {
            let part = parts.iter().find(|p| p.id == id).unwrap();
            part.mesh
                .positions
                .iter()
                .map(|p| p[1] + part.joint.y)
                .fold(f32::MAX, f32::min)
        };
        let hip = parts
            .iter()
            .find(|p| p.id == PartId::LeftLeg)
            .unwrap()
            .joint
            .y;
        assert!(lowest(PartId::LeftArm) < hip - 0.01);
        assert!(lowest(PartId::RightArm) < hip - 0.01);
    }

    #[test]
    fn the_same_recipe_gives_the_same_bear() {
        let a = build(&Recipe::default());
        let b = build(&Recipe::default());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.mesh, b.mesh);
        }
    }

    #[test]
    fn a_smaller_recipe_scales_the_bear() {
        let small = Recipe {
            height: 0.2,
            ..Recipe::default()
        };
        let (lo, hi) = bounds(&build(&small));
        let height = hi.y - lo.y;
        assert!((0.18..=0.21).contains(&height), "height {height}");
    }

    #[test]
    fn the_seams_sit_on_the_surface() {
        let defs = define(&Recipe::default());
        for def in &defs {
            let Some(seam) = def.seam else {
                assert_eq!(def.id, PartId::Body);
                continue;
            };
            assert!((seam.axis.length() - 1.0).abs() < 1e-4);
            assert!((0.01..0.05).contains(&seam.radius), "{:?}", def.id);
        }
    }

    #[test]
    fn the_round_cone_is_a_capsule_with_equal_radii() {
        let (a, b) = (Vec3::ZERO, Vec3::new(0.0, 1.0, 0.0));
        let d = round_cone(Vec3::new(0.5, 0.5, 0.0), a, b, 0.2, 0.2);
        assert!((d - 0.3).abs() < 1e-5);
        let d = round_cone(Vec3::new(0.0, 1.5, 0.0), a, b, 0.2, 0.2);
        assert!((d - 0.3).abs() < 1e-5);
    }

    #[test]
    fn the_fur_texture_tiles_and_repeats() {
        let a = fur_texture(64, 3);
        assert_eq!(a.len(), 64 * 64 * 4);
        assert_eq!(a, fur_texture(64, 3));
        assert_ne!(a, fur_texture(64, 4));
        let mean = a.iter().step_by(4).map(|&v| v as f32).sum::<f32>() / (64.0 * 64.0);
        assert!((90.0..220.0).contains(&mean), "mean {mean}");
    }

    #[test]
    fn part_names_round_trip() {
        for id in PartId::ALL {
            assert_eq!(PartId::from_name(id.name()), Some(id));
            if let Some(parent) = id.parent() {
                let order = |p| PartId::ALL.iter().position(|&q| q == p);
                assert!(order(parent) < order(id));
            }
        }
    }
}
