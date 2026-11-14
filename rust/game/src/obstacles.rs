//! `BfObstacles`: the obstacles of the zone on the screen (R23).
//!
//! The terrain makes the list of obstacles with the zone
//! (`bf_worldgen::obstacles`). `ObstacleSim` (`obstacle_sim.rs`) moves
//! them. This node is the thin part that talks to Godot. It gives the
//! results to the bear and draws the obstacles:
//! - one MultiMesh for all puffs of dust and steam (1 draw call). A
//!   MultiMesh draws many copies of one mesh in one draw call.
//! - one MultiMesh for all rolling stones (1 draw call)
//! - one MultiMesh for all vent mounds (1 draw call)
//! - one mesh and one collision shape for each ash crust near the camera

use crate::bear::{BfBear, BEAR_GROUP, GRIP_META, SOFT_META};
use crate::obstacle_sim::{BearIn, Crust, ObstacleSim, Puff, PuffKind, SimParams};
use crate::terrain::{BfTerrain, TERRAIN_GROUP};
use bf_worldgen::obstacles::{Kind, Lid, Obstacle, KIND_NAMES};
use bf_worldgen::HeightMap;
use glam::Vec3;
use godot::classes::base_material_3d::{
    BillboardMode, CullMode, Flags, ShadingMode, TextureParam, Transparency,
};
use godot::classes::geometry_instance_3d::ShadowCastingSetting;
use godot::classes::image::Format;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::multi_mesh::TransformFormat;
use godot::classes::{
    ArrayMesh, CollisionShape3D, ConcavePolygonShape3D, Image, ImageTexture, InputEvent,
    InputEventKey, Material, MeshInstance3D, MultiMesh, MultiMeshInstance3D, Node3D, QuadMesh,
    SphereMesh, StandardMaterial3D, StaticBody3D,
};
use godot::global::Key;
use godot::prelude::*;

/// The group that holds the `BfObstacles` node.
pub const OBSTACLES_GROUP: &str = "bf_obstacles";

/// The lid of a crust is this much larger than the pit, in metres. It
/// covers the ground that goes down to the pit.
const LID_EXTRA: f32 = 1.0;
/// The number of pieces that a crust breaks into.
const SHARDS: usize = 6;
/// The number of triangle columns in one piece.
const SHARD_SEGMENTS: usize = 3;
/// The number of rings of triangles in a lid.
const LID_RINGS: usize = 4;

fn to_godot(v: Vec3) -> Vector3 {
    Vector3::new(v.x, v.y, v.z)
}

/// One puff of dust or steam on the screen.
struct LivePuff {
    puff: Puff,
    age: f32,
}

/// One piece of a broken crust.
struct Shard {
    node: Gd<MeshInstance3D>,
    /// The place of the piece in the lid.
    from: Vector3,
    /// The place on the floor of the pit where the piece stops.
    to: Vector3,
    axis: Vector3,
    spin: f32,
    /// The time until the piece starts to fall.
    delay: f32,
    /// The part of the fall that is done, from 0 to 1.
    fall: f32,
}

/// The nodes of one ash crust.
struct CrustNodes {
    /// The index of the obstacle in the simulation.
    index: usize,
    body: Gd<StaticBody3D>,
    shape: Gd<CollisionShape3D>,
    mesh: Gd<MeshInstance3D>,
    shards: Vec<Shard>,
}

/// Draws the obstacles of the zone and gives their effects to the bear.
/// Dev key: F6 puts the bear at the next obstacle.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfObstacles {
    /// Turns all obstacles on or off.
    #[export]
    #[init(val = true)]
    enabled: bool,

    /// How long an easy ash crust shakes before it falls, in seconds.
    #[export]
    #[init(val = SimParams::default().crust_shake_easy_s)]
    crust_shake_easy_s: f32,

    /// How long a hard ash crust shakes before it falls, in seconds.
    #[export]
    #[init(val = SimParams::default().crust_shake_hard_s)]
    crust_shake_hard_s: f32,

    /// How long a stone shakes at the top before it rolls, in seconds.
    #[export]
    #[init(val = SimParams::default().stone_tell_s)]
    stone_tell_s: f32,

    /// The time between two stones of one wave, in seconds.
    #[export]
    #[init(val = SimParams::default().stone_gap_s)]
    stone_gap_s: f32,

    /// The gravity on a stone in m/s each second.
    #[export]
    #[init(val = SimParams::default().stone_gravity)]
    stone_gravity: f32,

    /// The rolling friction of a stone. A small value rolls far.
    #[export]
    #[init(val = SimParams::default().stone_friction)]
    stone_friction: f32,

    /// The highest speed of a stone in m/s.
    #[export]
    #[init(val = SimParams::default().stone_max_speed)]
    stone_max_speed: f32,

    /// The part of the speed of a stone that the bear gets in a hit (0 to 1).
    #[export]
    #[init(val = SimParams::default().stone_knock_part)]
    stone_knock_part: f32,

    /// The speed up that the bear gets when a stone hits it, in m/s.
    #[export]
    #[init(val = SimParams::default().stone_knock_up)]
    stone_knock_up: f32,

    /// Stones start only if the bear is nearer than this, in metres.
    #[export]
    #[init(val = SimParams::default().stone_wake_m)]
    stone_wake_m: f32,

    /// The wind speed of an easy gust in m/s. The bear walks at 1.4 m/s.
    #[export]
    #[init(val = SimParams::default().gust_speed_easy)]
    gust_speed_easy: f32,

    /// The wind speed of a hard gust in m/s. The bear jogs at 3.2 m/s.
    #[export]
    #[init(val = SimParams::default().gust_speed_hard)]
    gust_speed_hard: f32,

    /// How long a gust blows in each cycle, in seconds.
    #[export]
    #[init(val = SimParams::default().gust_blow_s)]
    gust_blow_s: f32,

    /// How long dust shows before a gust blows, in seconds.
    #[export]
    #[init(val = SimParams::default().gust_tell_s)]
    gust_tell_s: f32,

    /// The wind on a bear in the air is multiplied by this value.
    #[export]
    #[init(val = SimParams::default().gust_air_factor)]
    gust_air_factor: f32,

    /// How long a vent makes more steam before it blows, in seconds.
    #[export]
    #[init(val = SimParams::default().vent_tell_s)]
    vent_tell_s: f32,

    /// How long a vent blows, in seconds.
    #[export]
    #[init(val = SimParams::default().vent_blast_s)]
    vent_blast_s: f32,

    /// The speed to the side that an easy push vent gives the bear, in m/s.
    #[export]
    #[init(val = SimParams::default().vent_push_easy)]
    vent_push_easy: f32,

    /// The speed to the side that a hard push vent gives the bear, in m/s.
    #[export]
    #[init(val = SimParams::default().vent_push_hard)]
    vent_push_hard: f32,

    /// The speed up that a push vent gives the bear, in m/s.
    #[export]
    #[init(val = SimParams::default().vent_push_up)]
    vent_push_up: f32,

    /// The bear must be this near to a launch vent, in metres.
    #[export]
    #[init(val = SimParams::default().vent_radius)]
    vent_radius: f32,

    /// A launch goes this far above the ledge, in metres.
    #[export]
    #[init(val = SimParams::default().launch_margin_m)]
    launch_margin_m: f32,

    /// Puffs show only for obstacles nearer than this, in metres.
    #[export]
    #[init(val = SimParams::default().puff_range_m)]
    puff_range_m: f32,

    /// The largest number of puffs of dust and steam at one time.
    #[export]
    #[init(val = 400)]
    max_puffs: i32,

    /// The largest number of stones that show at one time.
    #[export]
    #[init(val = 48)]
    max_stones: i32,

    /// An ash crust shows only if the camera is nearer than this, in metres.
    #[export]
    #[init(val = 180.0)]
    crust_draw_range_m: f32,

    sim: Option<ObstacleSim>,
    puffs: Vec<LivePuff>,
    puff_mesh: Option<Gd<MultiMesh>>,
    stone_mesh: Option<Gd<MultiMesh>>,
    /// The parent of all nodes that belong to the current zone.
    zone_nodes: Option<Gd<Node3D>>,
    crusts: Vec<CrustNodes>,
    crust_material: Option<Gd<StandardMaterial3D>>,
    /// The time since the zone showed, for the shake of the crusts.
    time: f32,
    /// The obstacle type that the next push of F6 goes to.
    tour: usize,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfObstacles {
    fn ready(&mut self) {
        self.base_mut().add_to_group(OBSTACLES_GROUP);
        self.make_pools();
        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone_changed");
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        let dt = delta as f32;
        self.time += dt;
        let tree = self.base().get_tree();
        let terrain = tree
            .get_first_node_in_group(TERRAIN_GROUP)
            .and_then(|n| n.try_cast::<BfTerrain>().ok());
        let bear = tree
            .get_first_node_in_group(BEAR_GROUP)
            .and_then(|n| n.try_cast::<BfBear>().ok());
        let (Some(terrain), Some(mut bear)) = (terrain, bear) else {
            return;
        };
        if !self.enabled {
            bear.bind_mut().set_wind(Vector3::ZERO);
            return;
        }
        let params = self.sim_params(&bear);
        let feet = bear.get_global_position();
        let climbing = bear.bind().is_climbing();
        let input = BearIn {
            feet: Vec3::new(feet.x, feet.y, feet.z),
            on_floor: bear.is_on_floor() && !climbing,
            climbing,
        };

        let out = {
            let terrain = terrain.bind();
            let (Some(zone), Some(sim)) = (terrain.zone(), self.sim.as_mut()) else {
                return;
            };
            let out = sim.step(dt, &input, &zone.heights, &params);
            for &index in &out.broke {
                self.break_crust(index, &zone.heights);
            }
            out
        };

        {
            let mut bear = bear.bind_mut();
            bear.set_wind(to_godot(out.wind));
            if let Some(knock) = out.knock {
                bear.knock(to_godot(knock));
            }
            if out.wet {
                bear.make_wet(1.0);
            }
            for event in &out.events {
                bear.report_event(GString::from(event));
            }
        }

        let room = self.max_puffs.max(0) as usize;
        for puff in out.puffs {
            if self.puffs.len() >= room {
                break;
            }
            self.puffs.push(LivePuff { puff, age: 0.0 });
        }
        self.move_puffs(dt);
        self.show_stones();
        self.move_crusts(dt);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        let Ok(key) = event.try_cast::<InputEventKey>() else {
            return;
        };
        if key.is_pressed() && !key.is_echo() && key.get_keycode() == Key::F6 {
            self.go_to_next();
        }
    }
}

#[godot_api]
impl BfObstacles {
    /// Called when the terrain shows a new zone: makes the obstacles of
    /// that zone.
    #[func]
    fn on_zone_changed(&mut self) {
        let tree = self.base().get_tree();
        let terrain = tree
            .get_first_node_in_group(TERRAIN_GROUP)
            .and_then(|n| n.try_cast::<BfTerrain>().ok());
        let Some(terrain) = terrain else {
            return;
        };
        let obstacles = terrain.bind().obstacles().to_vec();
        self.build_zone(&obstacles);
        self.sim = Some(ObstacleSim::new(obstacles));
        self.puffs.clear();
        self.time = 0.0;
        self.tour = 0;
    }

    /// Puts the bear near the next obstacle (dev key: F6). Each push
    /// goes to the next type: crust, stones, gust, push vent, launch
    /// vent. Of that type, it selects the nearest obstacle that is more
    /// than 8 m away. The result tells which obstacle it is: `kind`,
    /// `id`, `pos` and `dir`. It is empty if the zone has no obstacles.
    #[func]
    pub fn go_to_next(&mut self) -> VarDictionary {
        let tree = self.base().get_tree();
        let terrain = tree.get_first_node_in_group(TERRAIN_GROUP);
        let bear = tree
            .get_first_node_in_group(BEAR_GROUP)
            .and_then(|n| n.try_cast::<BfBear>().ok());
        let mut found = VarDictionary::new();
        let (Some(mut terrain), Some(mut bear), Some(sim)) = (terrain, bear, &self.sim) else {
            return found;
        };
        let from = bear.get_global_position();
        for _ in 0..KIND_NAMES.len() {
            let kind = self.tour % KIND_NAMES.len();
            self.tour += 1;
            let distance = |o: &Obstacle| {
                let (dx, dz) = (o.pos[0] - from.x, o.pos[2] - from.z);
                (dx * dx + dz * dz).sqrt()
            };
            let nearest = sim
                .obstacles()
                .iter()
                .filter(|o| o.kind.index() == kind && distance(o) > 8.0)
                .min_by(|a, b| distance(a).total_cmp(&distance(b)));
            let Some(o) = nearest else {
                continue;
            };
            // A place next to the obstacle, where it does not act yet.
            let (dir, side) = (o.dir, [-o.dir[1], o.dir[0]]);
            let (along, across) = match &o.kind {
                Kind::AshCrust { radius, .. } => (-(radius + 1.5), 0.0),
                Kind::RollingStones { .. } => (10.0, 2.5),
                Kind::Gust { half_length, .. } => (-(half_length + 1.5), 0.0),
                Kind::VentPush { .. } => (-1.5, 0.0),
                Kind::VentLaunch { .. } => (-1.5, 0.0),
            };
            let x = o.pos[0] + dir[0] * along + side[0] * across;
            let z = o.pos[2] + dir[1] * along + side[1] * across;
            let ground = terrain
                .call("height_at", &[x.to_variant(), z.to_variant()])
                .to::<f32>();
            bear.bind_mut().go_to(Vector3::new(x, ground + 0.3, z));
            godot_print!(
                "BfObstacles: the bear is at {} #{} (difficulty {:.2})",
                o.kind.name(),
                o.id,
                o.difficulty
            );
            found.set("kind", o.kind.name());
            found.set("id", i64::from(o.id));
            found.set("pos", Vector3::new(o.pos[0], o.pos[1], o.pos[2]));
            found.set("dir", Vector3::new(o.dir[0], 0.0, o.dir[1]));
            break;
        }
        found
    }

    /// Gives one line of text for the debug overlay.
    #[func]
    pub fn debug_text(&self) -> GString {
        let Some(sim) = &self.sim else {
            return GString::from("none");
        };
        let text = format!(
            "{}   stones {}   puffs {}",
            sim.obstacles().len(),
            sim.stones().len(),
            self.puffs.len()
        );
        GString::from(&text)
    }
}

impl BfObstacles {
    /// Gives the tuning values from the exported fields and the bear.
    fn sim_params(&self, bear: &Gd<BfBear>) -> SimParams {
        let (bear_gravity, bear_fall_factor) = bear.bind().gravity();
        SimParams {
            crust_shake_easy_s: self.crust_shake_easy_s,
            crust_shake_hard_s: self.crust_shake_hard_s,
            stone_tell_s: self.stone_tell_s,
            stone_gap_s: self.stone_gap_s,
            stone_gravity: self.stone_gravity,
            stone_friction: self.stone_friction,
            stone_max_speed: self.stone_max_speed,
            stone_knock_part: self.stone_knock_part,
            stone_knock_up: self.stone_knock_up,
            stone_wake_m: self.stone_wake_m,
            gust_speed_easy: self.gust_speed_easy,
            gust_speed_hard: self.gust_speed_hard,
            gust_blow_s: self.gust_blow_s,
            gust_tell_s: self.gust_tell_s,
            gust_air_factor: self.gust_air_factor,
            vent_tell_s: self.vent_tell_s,
            vent_blast_s: self.vent_blast_s,
            vent_push_easy: self.vent_push_easy,
            vent_push_hard: self.vent_push_hard,
            vent_push_up: self.vent_push_up,
            vent_radius: self.vent_radius,
            launch_margin_m: self.launch_margin_m,
            bear_gravity,
            bear_fall_factor,
            puff_range_m: self.puff_range_m,
        }
    }

    /// Makes the MultiMesh of the puffs and the MultiMesh of the stones.
    /// They stay for all zones.
    fn make_pools(&mut self) {
        // The zone is 2 km wide. The instances move, so the nodes get a
        // box that has all of the zone in it.
        let everywhere = Aabb::new(
            Vector3::new(-3000.0, -1000.0, -3000.0),
            Vector3::new(6000.0, 3000.0, 6000.0),
        );

        // Puffs: soft round cards that look at the camera.
        let mut material = StandardMaterial3D::new_gd();
        material.set_shading_mode(ShadingMode::UNSHADED);
        material.set_transparency(Transparency::ALPHA);
        material.set_billboard_mode(BillboardMode::ENABLED);
        material.set_flag(Flags::BILLBOARD_KEEP_SCALE, true);
        material.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);
        material.set_cull_mode(CullMode::DISABLED);
        if let Some(texture) = soft_dot_texture() {
            material.set_texture(TextureParam::ALBEDO, &texture);
        }
        let mut quad = QuadMesh::new_gd();
        quad.set_size(Vector2::new(1.0, 1.0));
        quad.set_material(&material.upcast::<Material>());
        let puffs = multimesh(&quad.upcast(), self.max_puffs.max(1));
        self.add_multimesh("Puffs", &puffs, everywhere);
        self.puff_mesh = Some(puffs);

        // Stones: a low-poly ball with blotches, so the roll shows.
        let mut material = StandardMaterial3D::new_gd();
        material.set_albedo(Color::from_rgb(0.47, 0.36, 0.29));
        material.set_roughness(1.0);
        if let Some(texture) = blotch_texture() {
            material.set_texture(TextureParam::ALBEDO, &texture);
        }
        let mut ball = SphereMesh::new_gd();
        ball.set_radius(1.0);
        ball.set_height(2.0);
        ball.set_radial_segments(8);
        ball.set_rings(4);
        ball.set_material(&material.upcast::<Material>());
        let stones = multimesh(&ball.upcast(), self.max_stones.max(1));
        self.add_multimesh("Stones", &stones, everywhere);
        self.stone_mesh = Some(stones);

        // Ash crust: pale ash with dark cracks.
        let mut material = StandardMaterial3D::new_gd();
        material.set_roughness(1.0);
        if let Some(texture) = crack_texture() {
            material.set_texture(TextureParam::ALBEDO, &texture);
        }
        self.crust_material = Some(material);
    }

    fn add_multimesh(&mut self, name: &str, mesh: &Gd<MultiMesh>, bounds: Aabb) {
        let mut node = MultiMeshInstance3D::new_alloc();
        node.set_name(name);
        node.set_multimesh(mesh);
        node.set_custom_aabb(bounds);
        node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
        self.base_mut().add_child(&node);
    }

    /// Makes the nodes of a new zone: the vent mounds and the crusts.
    fn build_zone(&mut self, obstacles: &[Obstacle]) {
        if let Some(mut old) = self.zone_nodes.take() {
            old.queue_free();
        }
        self.crusts.clear();
        let mut parent = Node3D::new_alloc();
        parent.set_name("Zone");
        self.base_mut().add_child(&parent);

        // The vent mounds: one MultiMesh. A push vent is dark and leans
        // to the side that it blows to. A launch vent is yellow and
        // stands straight.
        let vents: Vec<&Obstacle> = obstacles
            .iter()
            .filter(|o| matches!(o.kind, Kind::VentPush { .. } | Kind::VentLaunch { .. }))
            .collect();
        if !vents.is_empty() {
            let mut material = StandardMaterial3D::new_gd();
            material.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);
            material.set_roughness(1.0);
            let mut mound = mound_mesh();
            mound.surface_set_material(0, &material.upcast::<Material>());
            let mut mesh = multimesh(&mound.upcast(), vents.len() as i32);
            mesh.set_visible_instance_count(vents.len() as i32);
            for (i, o) in vents.iter().enumerate() {
                let push = matches!(o.kind, Kind::VentPush { .. });
                let side = Vector3::new(-o.dir[1], 0.0, o.dir[0]);
                let lean = if push { 0.45 } else { 0.0 };
                let basis = Basis::from_axis_angle(side, -lean);
                let origin = Vector3::new(o.pos[0], o.pos[1] - 0.02, o.pos[2]);
                mesh.set_instance_transform(i as i32, Transform3D::new(basis, origin));
                let colour = if push {
                    Color::from_rgb(0.33, 0.19, 0.14)
                } else {
                    Color::from_rgb(0.86, 0.71, 0.27)
                };
                mesh.set_instance_color(i as i32, colour);
            }
            let mut node = MultiMeshInstance3D::new_alloc();
            node.set_name("Vents");
            node.set_multimesh(&mesh);
            node.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            parent.add_child(&node);
        }

        // The ash crusts: a lid with a collision shape over each pit.
        for (index, o) in obstacles.iter().enumerate() {
            let Kind::AshCrust { radius, lid, .. } = &o.kind else {
                continue;
            };
            let centre = Vec3::from_array(o.pos);
            let triangles = lid_triangles(lid, centre, *radius + LID_EXTRA, 0..SHARDS);
            let mut body = StaticBody3D::new_alloc();
            body.set_name(&format!("Crust{}", o.id));
            body.set_position(to_godot(centre));
            // The crust is soft ash: it holds a little, and the feet sink.
            body.set_meta(GRIP_META, &0.5f32.to_variant());
            body.set_meta(SOFT_META, &0.6f32.to_variant());

            let faces: PackedVector3Array = triangles.iter().flatten().copied().collect();
            let mut concave = ConcavePolygonShape3D::new_gd();
            concave.set_backface_collision_enabled(true);
            concave.set_faces(&faces);
            let mut shape = CollisionShape3D::new_alloc();
            shape.set_shape(&concave);
            body.add_child(&shape);

            let mut mesh = MeshInstance3D::new_alloc();
            mesh.set_mesh(&triangle_mesh(&triangles, centre, Vector3::ZERO));
            if let Some(material) = &self.crust_material {
                mesh.set_material_override(&material.clone().upcast::<Material>());
            }
            mesh.set_visibility_range_end(self.crust_draw_range_m);
            mesh.set_cast_shadows_setting(ShadowCastingSetting::OFF);
            body.add_child(&mesh);
            parent.add_child(&body);
            self.crusts.push(CrustNodes {
                index,
                body,
                shape,
                mesh,
                shards: Vec::new(),
            });
        }
        self.zone_nodes = Some(parent);
    }

    /// A crust falls: its collision goes away, and its lid becomes
    /// pieces that fall into the pit.
    fn break_crust(&mut self, index: usize, heights: &HeightMap) {
        let Some(sim) = &self.sim else {
            return;
        };
        let Some(o) = sim.obstacles().get(index) else {
            return;
        };
        let Kind::AshCrust { radius, lid, .. } = &o.kind else {
            return;
        };
        let Some(crust) = self.crusts.iter_mut().find(|c| c.index == index) else {
            return;
        };
        // The physics engine is in its step now: change the shape later.
        crust.shape.set_deferred("disabled", &true.to_variant());
        crust.mesh.set_visible(false);
        let centre = Vec3::from_array(o.pos);
        for k in 0..SHARDS {
            let triangles = lid_triangles(lid, centre, *radius + LID_EXTRA, k..k + 1);
            let count = (triangles.len() * 3) as f32;
            let middle = triangles
                .iter()
                .flatten()
                .fold(Vector3::ZERO, |sum, &v| sum + v)
                / count;
            let mut node = MeshInstance3D::new_alloc();
            node.set_mesh(&triangle_mesh(&triangles, centre, middle));
            if let Some(material) = &self.crust_material {
                node.set_material_override(&material.clone().upcast::<Material>());
            }
            node.set_visibility_range_end(self.crust_draw_range_m);
            node.set_position(middle);
            crust.body.add_child(&node);
            // The numbers below are for the look only. The piece falls
            // to the floor of the pit, nearer to the middle.
            let r = |salt: u32| bf_worldgen::noise::hash01(k as i32, o.id as i32, salt);
            let mut to = middle * (0.3 + 0.2 * r(6));
            to.y = heights.sample_world(centre.x + to.x, centre.z + to.z) - centre.y + 0.05;
            crust.shards.push(Shard {
                node,
                from: middle,
                to,
                axis: Vector3::new(r(1) - 0.5, (r(2) - 0.5) * 0.3, r(3) - 0.5).normalized(),
                spin: 1.5 + 3.0 * r(4),
                delay: 0.12 * r(5),
                fall: 0.0,
            });
        }
    }

    /// Shakes the crusts that shake, and moves the pieces that fall.
    fn move_crusts(&mut self, dt: f32) {
        let Some(sim) = &self.sim else {
            return;
        };
        let t = self.time;
        for crust in &mut self.crusts {
            if let Crust::Shaking { left, total } = sim.crust(crust.index) {
                // The shake gets stronger until the crust falls.
                let k = 0.006 + 0.016 * (1.0 - left / total.max(0.01));
                let offset = Vector3::new((t * 61.0).sin(), (t * 83.0).sin(), (t * 71.0).cos());
                crust.mesh.set_position(offset * k);
            }
            for shard in &mut crust.shards {
                if shard.fall >= 1.0 {
                    continue;
                }
                if shard.delay > 0.0 {
                    shard.delay -= dt;
                    continue;
                }
                // The piece falls in 0.5 s. It turns, and it breaks into
                // a smaller piece on the way.
                shard.fall = (shard.fall + dt / 0.5).min(1.0);
                let u = shard.fall;
                let mut position = shard.from.lerp(shard.to, u);
                position.y = shard.from.y + (shard.to.y - shard.from.y) * u * u;
                let size = 1.0 - 0.5 * u;
                let turn = Basis::from_axis_angle(shard.axis, shard.spin * u);
                shard
                    .node
                    .set_basis(turn.scaled(Vector3::new(size, size, size)));
                shard.node.set_position(position);
            }
        }
    }

    /// Moves the puffs and gives them to the MultiMesh.
    fn move_puffs(&mut self, dt: f32) {
        self.puffs.retain_mut(|p| {
            p.age += dt;
            let puff = &mut p.puff;
            // The air slows a puff. Steam goes up.
            puff.vel *= 1.0 - (1.8 * dt).min(1.0);
            if puff.kind == PuffKind::Steam {
                puff.vel.y += 0.9 * dt;
            }
            puff.pos += puff.vel * dt;
            p.age < puff.life
        });
        let Some(mesh) = &mut self.puff_mesh else {
            return;
        };
        let count = (self.puffs.len() as i32).min(mesh.get_instance_count());
        for (i, p) in self.puffs.iter().take(count as usize).enumerate() {
            let k = p.age / p.puff.life.max(0.01);
            // A puff grows, comes in fast and goes out slowly.
            let size = p.puff.size * (1.0 + 1.6 * k);
            let alpha = (k * 8.0).min(1.0) * (1.0 - k) * (1.0 - k);
            let colour = match p.puff.kind {
                // Darker than the pale ground, so the tells show.
                PuffKind::Dust => Color::from_rgba(0.52, 0.42, 0.30, 0.6 * alpha),
                PuffKind::Steam => Color::from_rgba(0.40, 0.38, 0.34, 0.75 * alpha),
            };
            let basis = Basis::from_scale(Vector3::new(size, size, size));
            mesh.set_instance_transform(i as i32, Transform3D::new(basis, to_godot(p.puff.pos)));
            mesh.set_instance_color(i as i32, colour);
        }
        mesh.set_visible_instance_count(count);
    }

    /// Gives the stones of the simulation to the MultiMesh.
    fn show_stones(&mut self) {
        let (Some(sim), Some(mesh)) = (&self.sim, &mut self.stone_mesh) else {
            return;
        };
        let stones = sim.stones();
        let count = (stones.len() as i32).min(mesh.get_instance_count());
        for (i, s) in stones.iter().take(count as usize).enumerate() {
            let dir = sim.obstacles()[s.owner].dir;
            let forward = if s.vel.length_squared() > 0.01 {
                Vector3::new(s.vel.x, 0.0, s.vel.y).normalized()
            } else {
                Vector3::new(dir[0], 0.0, dir[1])
            };
            let turn = Basis::from_axis_angle(Vector3::UP.cross(forward).normalized(), s.roll);
            // The stone is not a perfect ball.
            let basis = Basis::from_cols(
                turn.col_a() * s.radius,
                turn.col_b() * s.radius * 0.84,
                turn.col_c() * s.radius * 0.93,
            );
            let mut origin = to_godot(s.pos);
            if s.wait > 0.0 {
                // The tell: the stone shakes before it rolls.
                origin.x += (s.wait * 47.0).sin() * 0.012;
                origin.z += (s.wait * 53.0).cos() * 0.012;
            }
            mesh.set_instance_transform(i as i32, Transform3D::new(basis, origin));
        }
        mesh.set_visible_instance_count(count);
    }
}

/// Makes a MultiMesh for `count` copies of a mesh, with a colour for
/// each copy. No copy shows at the start.
fn multimesh(mesh: &Gd<godot::classes::Mesh>, count: i32) -> Gd<MultiMesh> {
    let mut out = MultiMesh::new_gd();
    out.set_transform_format(TransformFormat::TRANSFORM_3D);
    out.set_use_colors(true);
    out.set_mesh(mesh);
    out.set_instance_count(count);
    out.set_visible_instance_count(0);
    out
}

/// Gives the triangles of the lid of a crust, relative to its centre.
///
/// The lid is a disc of `SHARDS` pieces. `pieces` selects which pieces
/// the result has. Each triangle is clockwise from above, so its front
/// looks up.
fn lid_triangles(
    lid: &Lid,
    centre: Vec3,
    radius: f32,
    pieces: std::ops::Range<usize>,
) -> Vec<[Vector3; 3]> {
    let segments = SHARDS * SHARD_SEGMENTS;
    let point = |ring: usize, segment: usize| {
        let angle = segment as f32 / segments as f32 * std::f32::consts::TAU;
        // The rim is not a perfect circle.
        let rim = 0.93 + 0.12 * bf_worldgen::noise::hash01((segment % segments) as i32, 0, 77);
        let r = radius * rim * ring as f32 / LID_RINGS as f32;
        let (x, z) = (centre.x + r * angle.cos(), centre.z + r * angle.sin());
        // 2 cm above the old ground, so the lid shows on top of the rim.
        Vector3::new(
            x - centre.x,
            lid.height(x, z) + 0.02 - centre.y,
            z - centre.z,
        )
    };
    let mut out = Vec::new();
    for piece in pieces {
        for s in piece * SHARD_SEGMENTS..(piece + 1) * SHARD_SEGMENTS {
            out.push([point(0, s), point(1, s), point(1, s + 1)]);
            for ring in 1..LID_RINGS {
                let (a, b) = (point(ring, s), point(ring, s + 1));
                let (c, d) = (point(ring + 1, s), point(ring + 1, s + 1));
                out.push([a, c, d]);
                out.push([a, d, b]);
            }
        }
    }
    out
}

/// Makes a mesh from triangles. `centre` is the world place of the
/// triangles (for the texture). `origin` moves the mesh: a piece of a
/// crust turns around its middle.
fn triangle_mesh(triangles: &[[Vector3; 3]], centre: Vec3, origin: Vector3) -> Gd<ArrayMesh> {
    let mut positions = PackedVector3Array::new();
    let mut normals = PackedVector3Array::new();
    let mut uvs = PackedVector2Array::new();
    for triangle in triangles {
        // Clockwise from the front: the normal is (c - a) x (b - a).
        let normal = (triangle[2] - triangle[0])
            .cross(triangle[1] - triangle[0])
            .normalized();
        for &v in triangle {
            positions.push(v - origin);
            normals.push(normal);
            uvs.push(Vector2::new(centre.x + v.x, centre.z + v.z) * 0.3);
        }
    }
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    let mut out = ArrayMesh::new_gd();
    out.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    out
}

/// Makes the mesh of a vent: a low mound with a dark hole on top. The
/// colour of each copy comes from the MultiMesh; the hole is dark.
fn mound_mesh() -> Gd<ArrayMesh> {
    const SEGMENTS: usize = 10;
    // (radius, height, brightness) from the foot to the hole.
    const RINGS: [(f32, f32, f32); 4] = [
        (0.36, 0.0, 1.0),
        (0.20, 0.11, 0.85),
        (0.12, 0.12, 0.35),
        (0.07, 0.03, 0.05),
    ];
    let point = |ring: usize, segment: usize| {
        let angle = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
        let (r, y, _) = RINGS[ring];
        Vector3::new(r * angle.cos(), y, r * angle.sin())
    };
    let mut positions = PackedVector3Array::new();
    let mut normals = PackedVector3Array::new();
    let mut colours = PackedColorArray::new();
    let mut add = |corners: [(usize, usize); 3]| {
        let p = corners.map(|(ring, segment)| point(ring, segment));
        let normal = (p[2] - p[0]).cross(p[1] - p[0]).normalized();
        for (v, (ring, _)) in p.into_iter().zip(corners) {
            positions.push(v);
            normals.push(normal);
            let grey = RINGS[ring].2;
            colours.push(Color::from_rgb(grey, grey, grey));
        }
    };
    for s in 0..SEGMENTS {
        for ring in 0..RINGS.len() - 1 {
            add([(ring, s), (ring + 1, s + 1), (ring + 1, s)]);
            add([(ring, s), (ring, s + 1), (ring + 1, s + 1)]);
        }
    }
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::NORMAL.ord() as usize, &normals.to_variant());
    arrays.set(ArrayType::COLOR.ord() as usize, &colours.to_variant());
    let mut out = ArrayMesh::new_gd();
    out.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    out
}

/// Makes a texture from a function that gives red, green, blue and
/// alpha (0 to 1) for each pixel position (0 to 1).
fn texture(size: i32, f: impl Fn(f32, f32) -> [f32; 4]) -> Option<Gd<ImageTexture>> {
    let mut bytes = PackedByteArray::new();
    bytes.resize((size * size * 4) as usize);
    for (i, px) in bytes.as_mut_slice().chunks_exact_mut(4).enumerate() {
        let (x, y) = (i as i32 % size, i as i32 / size);
        let colour = f(
            (x as f32 + 0.5) / size as f32,
            (y as f32 + 0.5) / size as f32,
        );
        for (dst, c) in px.iter_mut().zip(colour) {
            *dst = (c.clamp(0.0, 1.0) * 255.0) as u8;
        }
    }
    let mut image = Image::create_from_data(size, size, false, Format::RGBA8, &bytes)?;
    image.generate_mipmaps();
    ImageTexture::create_from_image(&image)
}

/// A white dot with a soft edge, for the puffs.
fn soft_dot_texture() -> Option<Gd<ImageTexture>> {
    texture(32, |x, y| {
        let d = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt() * 2.0;
        let a = (1.0 - d).clamp(0.0, 1.0);
        [1.0, 1.0, 1.0, a * a * (3.0 - 2.0 * a)]
    })
}

/// Light and dark blotches, for the stones.
fn blotch_texture() -> Option<Gd<ImageTexture>> {
    texture(16, |x, y| {
        let cell = |v: f32| (v * 4.0) as i32;
        let n = bf_worldgen::noise::hash01(cell(x), cell(y), 91);
        let v = 0.6 + 0.8 * n;
        [v, v, v, 1.0]
    })
}

/// Pale ash with dark cracks, for the ash crust. The cracks are the
/// borders between the cells of random points (a Voronoi pattern). The
/// texture repeats without a seam.
fn crack_texture() -> Option<Gd<ImageTexture>> {
    const CELLS: i32 = 5;
    texture(128, |x, y| {
        let (px, py) = (x * CELLS as f32, y * CELLS as f32);
        let (cx, cy) = (px.floor() as i32, py.floor() as i32);
        let (mut nearest, mut second) = (f32::MAX, f32::MAX);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (gx, gy) = (cx + dx, cy + dy);
                let (wx, wy) = (gx.rem_euclid(CELLS), gy.rem_euclid(CELLS));
                let jx = bf_worldgen::noise::hash01(wx, wy, 301);
                let jy = bf_worldgen::noise::hash01(wx, wy, 302);
                let d = ((gx as f32 + jx - px).powi(2) + (gy as f32 + jy - py).powi(2)).sqrt();
                if d < nearest {
                    second = nearest;
                    nearest = d;
                } else if d < second {
                    second = d;
                }
            }
        }
        let edge = ((second - nearest) / 0.09).clamp(0.0, 1.0);
        let grain = bf_worldgen::noise::hash01((x * 128.0) as i32, (y * 128.0) as i32, 303);
        let v = 0.35 + 0.65 * edge;
        let pale = [0.66, 0.61, 0.54];
        [
            pale[0] * v + grain * 0.04,
            pale[1] * v + grain * 0.04,
            pale[2] * v + grain * 0.04,
            1.0,
        ]
    })
}
