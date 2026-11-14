//! The ragdoll of a tumbling bear (R11): one Jolt rigid body for the
//! body, the head and each limb (6 bodies), with joints between them.
//! The ears stay on the head. The rules are in `tumble.rs`.
//!
//! `BfBear` makes the ragdoll when a tumble starts and removes it when
//! the bear gets up. While it exists, the visible bear is hidden and
//! the ragdoll shows the same meshes.

use crate::anim::LIMBS;
use crate::bear_shape::PartId;
use crate::lost_limb::{add_looks, setup_body, PartLook};
use crate::tumble::{joint_limits, BodyDef};
use godot::classes::cone_twist_joint_3d::Param;
use godot::classes::{
    ConeTwistJoint3D, Joint3D, MeshInstance3D, Node3D, PhysicsBody3D, PinJoint3D, RigidBody3D,
};
use godot::prelude::*;

/// One part for a new ragdoll.
pub struct RagdollPart {
    pub id: PartId,
    pub def: BodyDef,
    /// The place and the turn of the part in the world. It can have a
    /// scale; the body gets none, and the mesh keeps it.
    pub global: Transform3D,
    /// The meshes of the part, relative to the transform of the part.
    pub looks: Vec<PartLook>,
    /// How loose the seam is (0 to 1). For the body, 0.
    pub looseness: f32,
    /// A limb that dangles on a thread: its socket on the body, in the
    /// world. The joint is a free pin there.
    pub dangling_socket: Option<Vector3>,
}

struct Body {
    id: PartId,
    node: Gd<RigidBody3D>,
    joint: Option<Gd<Joint3D>>,
    last_velocity: Vector3,
}

/// The bodies and the joints of a tumble.
pub struct Ragdoll {
    root: Gd<Node3D>,
    bodies: Vec<Body>,
}

/// Gives an orthonormal basis with its X axis along `dir`.
fn basis_along_x(dir: Vector3) -> Basis {
    let x = dir.try_normalized().unwrap_or(Vector3::RIGHT);
    let helper = if x.y.abs() > 0.9 {
        Vector3::FORWARD
    } else {
        Vector3::UP
    };
    let z = x.cross(helper).normalized();
    let y = z.cross(x);
    Basis::from_cols(x, y, z)
}

impl Ragdoll {
    /// Makes the ragdoll as a child of `owner` (the bear). Each body
    /// gets `velocity` and turns at `spin` around the body.
    pub fn build(
        owner: &mut Gd<PhysicsBody3D>,
        parts: Vec<RagdollPart>,
        velocity: Vector3,
        spin: Vector3,
        gravity_scale: f32,
    ) -> Ragdoll {
        // The root does not move with the bear.
        let mut root = Node3D::new_alloc();
        root.set_name("Ragdoll");
        root.set_as_top_level(true);
        owner.add_child(&root);
        root.set_global_transform(Transform3D::IDENTITY);

        let centre = parts
            .iter()
            .find(|p| p.id == PartId::Body)
            .map_or(Vector3::ZERO, |p| p.global.origin);
        let mut bodies = Vec::new();
        for part in &parts {
            let mut node = RigidBody3D::new_alloc();
            node.set_name(part.id.name());
            setup_body(&mut node, &part.def, gravity_scale);
            // The mesh keeps the scale of the part; the body has none.
            let basis = part.global.basis.orthonormalized();
            let scale = basis.inverse() * part.global.basis;
            let looks: Vec<PartLook> = part
                .looks
                .iter()
                .map(|l| PartLook {
                    mesh: l.mesh.clone(),
                    material: l.material.clone(),
                    local: Transform3D::new(scale, Vector3::ZERO) * l.local,
                })
                .collect();
            add_looks(&mut node, &looks);
            node.add_collision_exception_with(&*owner);
            root.add_child(&node);
            node.set_global_transform(Transform3D::new(basis, part.global.origin));
            let v = velocity + spin.cross(part.global.origin - centre);
            node.set_linear_velocity(v);
            node.set_angular_velocity(spin);
            bodies.push(Body {
                id: part.id,
                node,
                joint: None,
                last_velocity: v,
            });
        }

        // The joints: each part on the body.
        let mut ragdoll = Ragdoll { root, bodies };
        for part in &parts {
            if part.id == PartId::Body {
                continue;
            }
            let joint = match part.dangling_socket {
                Some(socket) => {
                    let mut pin = PinJoint3D::new_alloc();
                    ragdoll.root.add_child(&pin);
                    pin.set_global_position(socket);
                    pin.upcast::<Joint3D>()
                }
                None => {
                    let mut cone = ConeTwistJoint3D::new_alloc();
                    ragdoll.root.add_child(&cone);
                    // The twist axis is the X axis of the joint: along
                    // the limb, or up the neck for the head.
                    let rest = match LIMBS.iter().position(|&l| l == part.id) {
                        Some(_) => part.def_tip_dir(),
                        None => Vector3::UP,
                    };
                    let dir = part.global.basis * rest;
                    let basis = basis_along_x(dir);
                    cone.set_global_transform(Transform3D::new(basis, part.global.origin));
                    let (swing, twist) = joint_limits(part.id, part.looseness);
                    cone.set_param(Param::SWING_SPAN, swing);
                    cone.set_param(Param::TWIST_SPAN, twist);
                    cone.upcast::<Joint3D>()
                }
            };
            let mut joint = joint;
            joint.set_name(&format!("Joint{}", part.id.name()));
            // The bodies must be in place before the joint reads them.
            joint.set_node_a(&NodePath::from("../Body"));
            joint.set_node_b(&NodePath::from(format!("../{}", part.id.name()).as_str()));
            if let Some(body) = ragdoll.bodies.iter_mut().find(|b| b.id == part.id) {
                body.joint = Some(joint);
            }
        }
        ragdoll
    }

    fn body(&self, id: PartId) -> Option<&Body> {
        self.bodies.iter().find(|b| b.id == id)
    }

    /// The body of the bear, if it is there.
    pub fn torso(&self) -> Option<Gd<RigidBody3D>> {
        self.body(PartId::Body).map(|b| b.node.clone())
    }

    /// One physics step. Gives the hits on the limbs: the limb (in the
    /// order of `LIMBS`) and the speed of the hit in m/s. A hit is a
    /// change of velocity that gravity did not make.
    pub fn step(&mut self, dt: f32, gravity: Vector3) -> Vec<(usize, f32)> {
        let mut hits = Vec::new();
        for body in &mut self.bodies {
            let v = body.node.get_linear_velocity();
            let change = v - body.last_velocity - gravity * dt;
            body.last_velocity = v;
            if let Some(limb) = LIMBS.iter().position(|&l| l == body.id) {
                hits.push((limb, change.length()));
            }
        }
        hits
    }

    /// Adds a velocity to all bodies (an obstacle hits the bear).
    pub fn push(&mut self, velocity: Vector3) {
        for body in &mut self.bodies {
            let v = body.node.get_linear_velocity() + velocity;
            body.node.set_linear_velocity(v);
            body.last_velocity = v;
        }
    }

    /// The wind carries the body: a force toward the wind speed.
    pub fn wind(&mut self, wind: Vector3, drag: f32) {
        if wind.length_squared() < 1e-6 {
            return;
        }
        for body in &mut self.bodies {
            let v = body.node.get_linear_velocity();
            let mass = body.node.get_mass();
            body.node.apply_central_force((wind - v) * drag * mass);
        }
    }

    /// Makes the joint of a limb looser (0 to 1).
    pub fn loosen(&mut self, limb: usize, looseness: f32) {
        let Some(&id) = LIMBS.get(limb) else {
            return;
        };
        let Some(body) = self.bodies.iter_mut().find(|b| b.id == id) else {
            return;
        };
        if let Some(Ok(mut cone)) = body.joint.clone().map(|j| j.try_cast::<ConeTwistJoint3D>()) {
            let (swing, twist) = joint_limits(id, looseness);
            cone.set_param(Param::SWING_SPAN, swing);
            cone.set_param(Param::TWIST_SPAN, twist);
        }
    }

    /// Takes a limb off the ragdoll. Gives its transform (no scale),
    /// its velocity and its turn rate, for a lost limb.
    pub fn detach(&mut self, limb: usize) -> Option<(Transform3D, Vector3, Vector3)> {
        let id = *LIMBS.get(limb)?;
        let index = self.bodies.iter().position(|b| b.id == id)?;
        let mut body = self.bodies.remove(index);
        if let Some(mut joint) = body.joint.take() {
            joint.queue_free();
        }
        let state = (
            body.node.get_global_transform(),
            body.node.get_linear_velocity(),
            body.node.get_angular_velocity(),
        );
        body.node.queue_free();
        Some(state)
    }

    /// The transform of each part's mesh in the world (with its scale),
    /// for the move back to the animation.
    pub fn transforms(&self) -> Vec<(PartId, Transform3D)> {
        self.bodies
            .iter()
            .filter_map(|b| {
                // The first mesh is the part; the ears of the head come after.
                let mesh = b
                    .node
                    .get_children()
                    .iter_shared()
                    .find_map(|c| c.try_cast::<MeshInstance3D>().ok())?;
                Some((b.id, mesh.get_global_transform()))
            })
            .collect()
    }

    /// The place of the joint of a limb in the world.
    pub fn limb_position(&self, limb: usize) -> Option<Vector3> {
        let id = *LIMBS.get(limb)?;
        self.body(id).map(|b| b.node.get_global_position())
    }

    /// Removes the ragdoll.
    pub fn free(mut self) {
        self.root.queue_free();
    }
}

impl RagdollPart {
    /// The direction from the joint to the tip of a limb, in the space
    /// of the part.
    fn def_tip_dir(&self) -> Vector3 {
        use crate::tumble::Collider;
        let dir = match self.def.collider {
            Collider::Capsule { a, b, .. } => (b - a).normalize_or(glam::Vec3::NEG_Y),
            Collider::Sphere { centre, .. } => centre.normalize_or(glam::Vec3::NEG_Y),
        };
        Vector3::new(dir.x, dir.y, dir.z)
    }
}
