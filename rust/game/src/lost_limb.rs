//! A limb that came off the bear (R12). It is a Jolt rigid body that
//! falls and rolls. When it is at rest, it freezes and becomes static,
//! so it stays in the world at almost no cost. The bear sews it on
//! again when it touches it.

use crate::bear_shape::PartId;
use crate::tumble::{BodyDef, Collider};
use godot::classes::rigid_body_3d::FreezeMode;
use godot::classes::{
    CapsuleShape3D, CollisionShape3D, IRigidBody3D, Material, Mesh, MeshInstance3D,
    PhysicsMaterial, RigidBody3D, Shape3D, SphereShape3D,
};
use godot::prelude::*;

/// The group of all lost limbs. The bear finds them there.
pub const LOST_LIMB_GROUP: &str = "bf_lost_limb";
/// The physics layer of the parts of the bear: the ragdoll and the lost
/// limbs (layer 5). The world is on layer 1.
pub const BEAR_PARTS_LAYER: u32 = 1 << 4;
/// The turn damping of a lost limb (1/s). It stops the limb from rolling
/// down a long slope.
const LOST_ANGULAR_DAMP: f32 = 6.0;
/// The speed damping of a lost limb (1/s).
const LOST_LINEAR_DAMP: f32 = 0.6;
/// The physics layer of the world.
pub const WORLD_LAYER: u32 = 1;

/// A limb on the ground. `BfBear` makes it.
#[derive(GodotClass)]
#[class(base = RigidBody3D, init)]
pub struct BfLostLimb {
    /// The limb, in the order of `LIMBS` (0 left arm to 3 right leg).
    limb: usize,
    /// The time that the limb is at rest now.
    rest: f32,
    /// The time since the limb came off.
    age: f32,
    base: Base<RigidBody3D>,
}

#[godot_api]
impl IRigidBody3D for BfLostLimb {
    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;
        self.age += dt;
        let speed = self.base().get_linear_velocity().length();
        let spin = self.base().get_angular_velocity().length();
        if speed < 0.05 && spin < 0.5 {
            self.rest += dt;
        } else {
            self.rest = 0.0;
        }
        // At rest, or after a long time: the limb becomes static.
        if self.rest > 0.6 || self.age > 20.0 {
            self.base_mut().set_freeze_mode(FreezeMode::STATIC);
            self.base_mut().set_freeze_enabled(true);
            self.base_mut().set_physics_process(false);
        }
    }
}

#[godot_api]
impl BfLostLimb {
    /// The name of the limb, for example "LeftArm".
    #[func]
    pub fn limb_name(&self) -> GString {
        GString::from(crate::tear::Seams::limb_name(self.limb))
    }

    /// Tells if the limb is at rest and static.
    #[func]
    pub fn is_frozen(&self) -> bool {
        self.base().is_freeze_enabled()
    }
}

impl BfLostLimb {
    pub fn limb(&self) -> usize {
        self.limb
    }
}

/// The look of a part: its mesh, its material, and the turn and scale
/// of the mesh relative to its body.
pub struct PartLook {
    pub mesh: Gd<Mesh>,
    pub material: Gd<Material>,
    pub local: Transform3D,
}

/// Makes the collision shape of a body.
pub fn collision_shape(collider: &Collider) -> Gd<CollisionShape3D> {
    let mut node = CollisionShape3D::new_alloc();
    let v = |p: glam::Vec3| Vector3::new(p.x, p.y, p.z);
    match *collider {
        Collider::Capsule { a, b, radius } => {
            let mut capsule = CapsuleShape3D::new_gd();
            capsule.set_radius(radius);
            capsule.set_height(a.distance(b) + 2.0 * radius);
            node.set_shape(&capsule.upcast::<Shape3D>());
            // A capsule is along its Y axis.
            let dir = (b - a).normalize_or(glam::Vec3::Y);
            let turn = glam::Quat::from_rotation_arc(glam::Vec3::Y, dir);
            let basis = Basis::from_quaternion(Quaternion::new(turn.x, turn.y, turn.z, turn.w));
            node.set_transform(Transform3D::new(basis, v((a + b) * 0.5)));
        }
        Collider::Sphere { centre, radius } => {
            let mut sphere = SphereShape3D::new_gd();
            sphere.set_radius(radius);
            node.set_shape(&sphere.upcast::<Shape3D>());
            node.set_position(v(centre));
        }
    }
    node
}

/// Gives a rigid body the settings of a part of the bear: the mass,
/// the layers, the material and continuous collision detection (the
/// parts are small and fast).
pub fn setup_body(body: &mut Gd<RigidBody3D>, def: &BodyDef, gravity_scale: f32) {
    body.set_mass(def.mass);
    body.set_gravity_scale(gravity_scale);
    body.set_collision_layer(BEAR_PARTS_LAYER);
    body.set_collision_mask(WORLD_LAYER);
    body.set_use_continuous_collision_detection(true);
    body.set_linear_damp(0.1);
    body.set_angular_damp(1.0);
    let mut material = PhysicsMaterial::new_gd();
    // Plush: much friction and almost no bounce.
    material.set_friction(0.9);
    material.set_bounce(0.05);
    body.set_physics_material_override(&material);
    body.add_child(&collision_shape(&def.collider));
}

/// Adds the meshes of a part to a body.
pub fn add_looks(body: &mut Gd<RigidBody3D>, looks: &[PartLook]) {
    for look in looks {
        let mut mesh = MeshInstance3D::new_alloc();
        mesh.set_mesh(&look.mesh);
        mesh.set_material_override(&look.material);
        mesh.set_transform(look.local);
        body.add_child(&mesh);
    }
}

/// What a new lost limb needs.
pub struct LostLimbSpec {
    pub limb: usize,
    pub id: PartId,
    pub def: BodyDef,
    pub looks: Vec<PartLook>,
    /// The place and turn of the body (no scale).
    pub transform: Transform3D,
    pub linear: Vector3,
    pub angular: Vector3,
    pub gravity_scale: f32,
}

/// Makes a lost limb in `parent` (the world). The limb ignores the bear
/// `owner`. If there are more than `max_count` lost limbs, the oldest go.
pub fn spawn(
    parent: &mut Gd<Node>,
    owner: &Gd<godot::classes::PhysicsBody3D>,
    spec: LostLimbSpec,
    max_count: usize,
) -> Gd<BfLostLimb> {
    let tree = parent.get_tree();
    let old = tree.get_nodes_in_group(LOST_LIMB_GROUP);
    let extra = (old.len() + 1).saturating_sub(max_count.max(1));
    for mut node in old.iter_shared().take(extra) {
        node.queue_free();
    }

    let mut limb = BfLostLimb::new_alloc();
    limb.bind_mut().limb = spec.limb;
    limb.set_name(&format!("Lost{}", spec.id.name()));
    {
        let mut body = limb.clone().upcast::<RigidBody3D>();
        setup_body(&mut body, &spec.def, spec.gravity_scale);
        // A stuffed limb does not roll far.
        body.set_angular_damp(LOST_ANGULAR_DAMP);
        body.set_linear_damp(LOST_LINEAR_DAMP);
        add_looks(&mut body, &spec.looks);
        body.add_collision_exception_with(owner);
    }
    limb.add_to_group(LOST_LIMB_GROUP);
    parent.add_child(&limb);
    limb.set_global_transform(spec.transform);
    limb.set_linear_velocity(spec.linear);
    limb.set_angular_velocity(spec.angular);
    limb
}
