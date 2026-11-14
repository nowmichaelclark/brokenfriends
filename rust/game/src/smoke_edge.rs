//! The smoke wall at the edge of the zone (R14, R16).
//!
//! A wall of thick, dark smoke surrounds the zone. The seed and the time
//! set its shape (`bf_worldgen::smoke`). The wall moves in and out, and
//! it is sometimes high and thick, sometimes low and thin. If the bear
//! stays in the smoke for too long, or goes too far in, it dies and
//! starts again at the start point (`smoke_dose.rs`).
//!
//! The smoke is cheap (R15). The Mobile renderer has no volumetric fog,
//! so this node draws three things, with 3 draw calls in total:
//! - an inner sheet at the front of the wall,
//! - an outer sheet behind it, and
//! - a set of billboard cards (textured quads that face the camera)
//!   between the two sheets.
//!
//! The shape goes to the GPU as a small float texture (256 x 2). Rust
//! writes the texture 20 times each second. A vertex shader moves the
//! sheets and the cards with it. Thus the picture and the rule use the
//! same numbers, and no mesh is made again while the game runs.

use crate::atmosphere::ATMOSPHERE_GROUP;
use crate::bear::BEAR_GROUP;
use crate::smoke_dose::SmokeDose;
use crate::terrain::TERRAIN_GROUP;
use bf_worldgen::noise::hash01;
use bf_worldgen::smoke::{tile_noise, SmokeEdge, SmokeParams};
use godot::builtin::Aabb;
use godot::classes::geometry_instance_3d::ShadowCastingSetting as Shadow;
use godot::classes::image::Format;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::multi_mesh::TransformFormat;
use godot::classes::{
    ArrayMesh, INode3D, Image, ImageTexture, Material, MeshInstance3D, MultiMesh,
    MultiMeshInstance3D, Node3D, QuadMesh, Shader, ShaderMaterial, Texture2D,
};
use godot::prelude::*;
use std::f32::consts::TAU;

/// The group that holds the `BfSmokeEdge` node.
pub const SMOKE_GROUP: &str = "bf_smoke_edge";

/// The number of angles around the wall. The texture is this wide.
const SAMPLES: usize = 256;
/// The size of the smoke noise image in pixels.
const NOISE_SIZE: usize = 128;
/// How far below the lowest ground of a place the sheets reach, in metres.
const BOTTOM_M: f32 = 250.0;

/// The shader for the two sheets. The vertex shader reads the shape of
/// the wall from `edge_data`: row 0 has the front radius, the height,
/// the band and the ground height. Row 1 has the density.
const SHEET_SHADER: &str = "
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, fog_disabled;

uniform sampler2D edge_data : filter_nearest, repeat_disable;
uniform sampler2D puffs : filter_linear_mipmap, repeat_enable;
uniform float samples = 256.0;
uniform float outer = 0.0;
uniform vec4 haze_color : source_color = vec4(0.74, 0.60, 0.42, 1.0);
uniform float haze_density = 0.0025;
uniform float bottom_m = 250.0;
uniform vec4 smoke_color : source_color = vec4(0.16, 0.12, 0.09, 1.0);
uniform float opacity = 0.92;
uniform float scroll = 0.02;
uniform float fade_distance = 25.0;

varying float v_h;
varying float v_density;
varying vec2 v_uv;

void vertex() {
    int idx = int(UV.x * samples + 0.5) % int(samples);
    vec4 d = texelFetch(edge_data, ivec2(idx, 0), 0);
    v_density = texelFetch(edge_data, ivec2(idx, 1), 0).x;
    float a = UV.x * 6.2831853;
    float r = d.x + outer * d.z;
    float y = d.w - bottom_m + UV.y * (d.y + bottom_m);
    VERTEX = vec3(cos(a) * r, y, sin(a) * r);
    v_h = (y - d.w) / d.y;
    v_uv = vec2(UV.x * 48.0, y * 0.012);
}

void fragment() {
    vec2 uv1 = v_uv + vec2(TIME * scroll, -TIME * scroll * 1.7);
    vec2 uv2 = v_uv * 1.9 + vec2(-TIME * scroll * 0.7, -TIME * scroll * 2.3);
    float n = texture(puffs, uv1).r * 0.6 + texture(puffs, uv2).r * 0.4;
    float body = smoothstep(0.2, 0.5, n + (v_density - 0.7) * 0.4);
    float top = 1.0 - smoothstep(0.55, 1.0, v_h);
    float dist = length(VERTEX);
    float near = smoothstep(2.0, fade_distance, dist);
    float haze = 1.0 - exp(-dist * haze_density);
    ALBEDO = mix(smoke_color.rgb * (0.7 + 0.5 * n), haze_color.rgb, haze);
    ALPHA = clamp(body * top * near * opacity, 0.0, 1.0);
}
";

/// The shader for the cards. Each card has custom data: the angle (as a
/// part of the circle), the place between the front and the end of the
/// band, the height as a part of the wall, and a size factor.
const CARD_SHADER: &str = "
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, fog_disabled;

uniform sampler2D edge_data : filter_nearest, repeat_disable;
uniform sampler2D puffs : filter_linear_mipmap, repeat_enable;
uniform float samples = 256.0;
uniform vec4 haze_color : source_color = vec4(0.74, 0.60, 0.42, 1.0);
uniform float haze_density = 0.0025;
uniform vec4 smoke_color : source_color = vec4(0.16, 0.12, 0.09, 1.0);
uniform float opacity = 0.8;
uniform float scroll = 0.02;
uniform float card_size = 160.0;
uniform float fade_distance = 25.0;

varying float v_dist;
varying float v_density;
varying float v_seed;

void vertex() {
    float idx_f = floor(INSTANCE_CUSTOM.r * samples + 0.5);
    int idx = int(idx_f) % int(samples);
    vec4 d = texelFetch(edge_data, ivec2(idx, 0), 0);
    v_density = texelFetch(edge_data, ivec2(idx, 1), 0).x;
    float a = idx_f / samples * 6.2831853;
    float r = d.x + INSTANCE_CUSTOM.g * d.z - 30.0;
    float y = d.w + INSTANCE_CUSTOM.b * d.y;
    vec3 centre = vec3(cos(a) * r, y, sin(a) * r);
    vec3 right = INV_VIEW_MATRIX[0].xyz;
    vec3 up = INV_VIEW_MATRIX[1].xyz;
    float size = card_size * INSTANCE_CUSTOM.a;
    vec3 world = centre + (right * VERTEX.x + up * VERTEX.y) * size;
    POSITION = PROJECTION_MATRIX * VIEW_MATRIX * vec4(world, 1.0);
    v_dist = length((VIEW_MATRIX * vec4(centre, 1.0)).xyz);
    v_seed = INSTANCE_CUSTOM.r * 17.0 + INSTANCE_CUSTOM.g * 5.0;
}

void fragment() {
    vec2 centred = UV * 2.0 - 1.0;
    float round_mask = 1.0 - smoothstep(0.3, 1.0, length(centred));
    vec2 p = UV * 0.5 + vec2(v_seed, v_seed * 0.37) + vec2(TIME * scroll, TIME * scroll * 0.6);
    float n = texture(puffs, p).r;
    float shape = smoothstep(0.3, 0.65, n + (v_density - 0.7) * 0.3) * round_mask;
    float near = smoothstep(10.0, fade_distance * 4.0, v_dist);
    float haze = 1.0 - exp(-v_dist * haze_density);
    ALBEDO = mix(smoke_color.rgb * (0.75 + 0.45 * n), haze_color.rgb, haze);
    ALPHA = clamp(shape * near * opacity, 0.0, 1.0);
}
";

/// Makes the smoke wall at the edge of the zone. It kills a bear that
/// stays in the smoke for too long.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfSmokeEdge {
    /// The bear dies when it has been this long in the smoke, in
    /// seconds. The time goes down twice as fast outside the smoke.
    #[export]
    #[init(val = 12.0)]
    max_seconds: f32,

    /// The bear dies when it is this far in the smoke, in metres.
    #[export]
    #[init(val = 45.0)]
    max_depth_m: f32,

    /// How many times faster than real time the timer goes down outside
    /// the smoke.
    #[export]
    #[init(val = 2.0)]
    timer_recovery: f32,

    /// The mean radius of the front of the wall, as a part of half the
    /// zone size. At 0.74, the front is 760 m from the centre of a zone of 2 km.
    #[export]
    #[init(val = SmokeParams::default().front_fraction)]
    front_fraction: f32,

    /// How far the front moves in and out from its mean, as a part of
    /// half the zone size.
    #[export]
    #[init(val = SmokeParams::default().swing_fraction)]
    swing_fraction: f32,

    /// The most that one wave moves the front, in metres per second. The
    /// waves add up, so the front can be about 3 times faster for a
    /// short time. A bear walks at 1.4 m/s.
    #[export]
    #[init(val = SmokeParams::default().front_speed_mps)]
    front_speed_mps: f32,

    /// The time in seconds that a change of the height, the band or the
    /// density needs for one cycle. A short time changes the look faster.
    #[export]
    #[init(val = SmokeParams::default().period_s)]
    period_s: f32,

    /// The lowest height of the wall above the ground, in metres.
    #[export]
    #[init(val = SmokeParams::default().height_min_m)]
    height_min_m: f32,

    /// The highest height of the wall above the ground, in metres. A bear
    /// that stands higher than the top of the wall is not in the smoke.
    #[export]
    #[init(val = SmokeParams::default().height_max_m)]
    height_max_m: f32,

    /// The smallest width of the band of smoke behind the front, in metres.
    #[export]
    #[init(val = SmokeParams::default().band_min_m)]
    band_min_m: f32,

    /// The largest width of the band, in metres.
    #[export]
    #[init(val = SmokeParams::default().band_max_m)]
    band_max_m: f32,

    /// The colour of the smoke in the wall. It is dark brown, not black.
    #[export]
    #[init(val = Color::from_rgb(0.16, 0.12, 0.09))]
    smoke_color: Color,

    /// The colour that far parts of the wall fade to. It should be the
    /// smog colour of `BfAtmosphere`. The wall does not use the fog of
    /// the environment, so it can show from farther away.
    #[export]
    #[init(val = Color::from_rgb(0.74, 0.60, 0.42))]
    haze_color: Color,

    /// How fast the wall fades to the haze colour with distance. At
    /// 0.0025, a part 300 m away is 53% haze. The smog of the zone has 0.006.
    #[export]
    #[init(val = 0.0025)]
    haze_density: f32,

    /// How solid the sheets of the wall are (0 to 1).
    #[export]
    #[init(val = 0.92)]
    wall_opacity: f32,

    /// How solid the cards in the wall are (0 to 1).
    #[export]
    #[init(val = 0.8)]
    card_opacity: f32,

    /// How fast the smoke texture moves. This is not the speed of the
    /// wall itself, only of the wisps in it.
    #[export]
    #[init(val = 0.02)]
    scroll_speed: f32,

    /// The number of cards. A card is a big soft puff. Zero turns them off.
    /// This is a cost: each card is overdraw.
    #[export]
    #[init(val = 96)]
    card_count: i32,

    /// The size of a card in metres.
    #[export]
    #[init(val = 160.0)]
    card_size_m: f32,

    /// A sheet fades out this near to the camera, in metres. This stops
    /// a hard edge on the screen when the bear walks through the wall.
    #[export]
    #[init(val = 25.0)]
    fade_distance_m: f32,

    /// How many times each second the shape goes to the GPU.
    #[export]
    #[init(val = 20.0)]
    texture_hz: f32,

    /// If this is off, the wall is not drawn. The rule for the bear stays.
    #[export]
    #[init(val = true)]
    show_wall: bool,

    /// If this is off, the smoke does not kill the bear. The picture stays.
    #[export]
    #[init(val = true)]
    kills: bool,

    /// The shape of the wall for the current zone.
    edge: Option<SmokeEdge>,
    /// The smoke that the bear has taken.
    dose: SmokeDose,
    /// The time in seconds. It counts the physics steps.
    clock: f64,
    /// The time since the shape went to the GPU, in seconds.
    since_texture: f32,
    /// The height of the ground near the wall for each angle, in metres.
    ground_y: Vec<f32>,
    /// The smoke in the fog, from 0 to 1 (smoothed).
    fog_amount: f32,
    /// The bear is above the wall now.
    above_wall: bool,
    /// The nodes and resources that the node made.
    sheets: Vec<Gd<MeshInstance3D>>,
    cards: Option<Gd<MultiMeshInstance3D>>,
    materials: Vec<Gd<ShaderMaterial>>,
    data_texture: Option<Gd<ImageTexture>>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfSmokeEdge {
    fn ready(&mut self) {
        self.base_mut().add_to_group(SMOKE_GROUP);
        self.build();
        // When a new zone shows, make the wall for its seed.
        let tree = self.base().get_tree();
        if let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) {
            let on_zone = self.base().callable("on_zone");
            terrain.connect("zone_changed", &on_zone);
        }
    }

    fn physics_process(&mut self, delta: f64) {
        self.clock += delta;
        if self.edge.is_none() {
            // The first zone may have shown before this node was ready.
            let tree = self.base().get_tree();
            let ready = tree
                .get_first_node_in_group(TERRAIN_GROUP)
                .is_some_and(|mut t| {
                    !t.call("is_generating", &[]).booleanize()
                        && t.call("generation_seconds", &[]).to::<f32>() > 0.0
                });
            if ready {
                self.on_zone();
            }
            return;
        }
        self.step_dose(delta as f32);
    }

    fn process(&mut self, delta: f64) {
        if self.edge.is_none() {
            return;
        }
        self.since_texture += delta as f32;
        if self.since_texture >= 1.0 / self.texture_hz.max(1.0) {
            self.since_texture = 0.0;
            self.write_texture();
        }
        let show = self.show_wall;
        for sheet in &mut self.sheets {
            sheet.set_visible(show);
        }
        if let Some(cards) = &mut self.cards {
            cards.set_visible(show && self.card_count > 0);
        }
        self.apply_look();
    }
}

#[godot_api]
impl BfSmokeEdge {
    /// Makes the wall for the seed and the size of the current zone. The
    /// node calls this when the terrain sends `zone_changed`.
    #[func]
    pub fn on_zone(&mut self) {
        let tree = self.base().get_tree();
        let Some(mut terrain) = tree.get_first_node_in_group(TERRAIN_GROUP) else {
            return;
        };
        let seed = terrain.call("current_seed", &[]).to::<i64>();
        let half = terrain.get("zone_size_m").to::<f32>() * 0.5;
        let params = self.params();
        let edge = SmokeEdge::new(seed, half, params);
        // The ground height for each angle: the highest of 5 places
        // along the range that the front can reach.
        let mut ground = Vec::with_capacity(SAMPLES);
        for i in 0..SAMPLES {
            let angle = i as f32 * TAU / SAMPLES as f32;
            let (s, c) = angle.sin_cos();
            let mut top = f32::MIN;
            for k in 0..5 {
                let f = params.front_fraction + params.swing_fraction * (k as f32 - 2.0) * 0.5;
                let r = (f * half).clamp(params.min_front_m, half - params.border_margin_m);
                let h = terrain
                    .call("height_at", &[(c * r).to_variant(), (s * r).to_variant()])
                    .to::<f32>();
                top = top.max(h);
            }
            ground.push(top);
        }
        self.ground_y = ground;
        self.edge = Some(edge);
        self.dose.reset();
        self.fog_amount = 0.0;
        self.set_fog(0.0);
        self.write_texture();
    }

    /// Gives how far the bear is in the smoke in metres. Zero means
    /// that the bear is not in the smoke.
    #[func]
    pub fn depth_m(&self) -> f32 {
        self.dose.depth_m()
    }

    /// Gives the seconds in the smoke that count now.
    #[func]
    pub fn smoke_seconds(&self) -> f32 {
        self.dose.seconds()
    }

    /// Gives how near the bear is to death by smoke, 0 to 1.
    #[func]
    pub fn danger(&self) -> f32 {
        self.dose.danger(self.max_seconds, self.max_depth_m)
    }

    /// Gives the radius of the front of the wall at an angle (radians) at
    /// the time now, in metres. It gives 0 if there is no wall yet.
    #[func]
    pub fn front_m_at(&self, angle: f32) -> f32 {
        self.edge
            .as_ref()
            .map_or(0.0, |e| e.sample(angle, self.clock as f32).front_m)
    }

    /// Gives one line of text for the debug overlay.
    #[func]
    pub fn debug_text(&self) -> GString {
        if self.edge.is_none() {
            return GString::from("no wall yet");
        }
        let text = if self.dose.is_inside() {
            format!(
                "IN SMOKE {:.0} m deep   {:.1} of {:.0} s",
                self.dose.depth_m(),
                self.dose.seconds(),
                self.max_seconds
            )
        } else if self.above_wall {
            String::from("above the wall")
        } else {
            let (x, z) = self.bear_xz().unwrap_or((0.0, 0.0));
            let distance = self
                .edge
                .as_ref()
                .map_or(0.0, |e| -e.depth_m(x, z, self.clock as f32));
            format!(
                "{distance:.0} m to the wall   timer {:.1} s",
                self.dose.seconds()
            )
        };
        GString::from(&text)
    }
}

impl BfSmokeEdge {
    fn params(&self) -> SmokeParams {
        let d = SmokeParams::default();
        SmokeParams {
            front_fraction: self.front_fraction,
            swing_fraction: self.swing_fraction,
            front_speed_mps: self.front_speed_mps,
            period_s: self.period_s,
            height_min_m: self.height_min_m,
            height_max_m: self.height_max_m.max(self.height_min_m),
            band_min_m: self.band_min_m,
            band_max_m: self.band_max_m.max(self.band_min_m),
            ..d
        }
    }

    fn bear_node(&self) -> Option<Gd<Node3D>> {
        self.base()
            .get_tree()
            .get_first_node_in_group(BEAR_GROUP)
            .and_then(|n| n.try_cast::<Node3D>().ok())
    }

    fn bear_xz(&self) -> Option<(f32, f32)> {
        let p = self.bear_node()?.get_global_position();
        Some((p.x, p.z))
    }

    /// One physics step of the rule: find the depth, count the time, kill.
    fn step_dose(&mut self, delta: f32) {
        let (Some(edge), Some(bear)) = (&self.edge, self.bear_node()) else {
            return;
        };
        let pos = bear.get_global_position();
        let t = self.clock as f32;
        let angle = pos.z.atan2(pos.x);
        let sample = edge.sample(angle, t);
        let r = (pos.x * pos.x + pos.z * pos.z).sqrt();
        let mut depth = r - sample.front_m;
        // A bear that stands higher than the top of the wall is out of the smoke.
        let index = ((angle.rem_euclid(TAU) / TAU * SAMPLES as f32).round() as usize) % SAMPLES;
        let ground = self.ground_y.get(index).copied().unwrap_or(0.0);
        self.above_wall = depth > 0.0 && pos.y > ground + sample.height_m;
        if self.above_wall {
            depth = -1.0;
        }
        let step = self.dose.step(
            delta,
            depth,
            self.max_seconds,
            self.max_depth_m,
            self.timer_recovery,
        );

        let mut bear = bear;
        if step.entered {
            bear.call("report_event", &[GString::from("smoke_enter").to_variant()]);
        }
        if step.left {
            bear.call("report_event", &[GString::from("smoke_leave").to_variant()]);
        }
        if let (Some(cause), true) = (step.died, self.kills) {
            let name = format!("smoke:{}", cause.name());
            bear.call("die", &[GString::from(&name).to_variant()]);
            self.dose.reset();
            self.fog_amount = 0.0;
            self.set_fog(0.0);
            return;
        }

        // The fog gets thick when the bear goes in, and thicker with the
        // danger. The change is smooth, so there is no jump on the screen.
        let danger = self.dose.danger(self.max_seconds, self.max_depth_m);
        let target = if self.dose.is_inside() {
            0.3 + 0.7 * danger
        } else {
            danger
        };
        self.fog_amount += (target - self.fog_amount) * (delta * 3.0).min(1.0);
        self.set_fog(self.fog_amount);
    }

    fn set_fog(&self, amount: f32) {
        if let Some(mut atmosphere) = self
            .base()
            .get_tree()
            .get_first_node_in_group(ATMOSPHERE_GROUP)
        {
            atmosphere.call("set_smoke_amount", &[amount.to_variant()]);
        }
    }

    /// Makes the two sheets, the cards, the noise image and the data
    /// texture. The wall is hidden until a zone shows.
    fn build(&mut self) {
        let Some(noise) = noise_texture(11) else {
            godot_error!("BfSmokeEdge: no noise texture.");
            return;
        };
        let Some(mut data_image) = Image::create_empty(SAMPLES as i32, 2, false, Format::RGBAF)
        else {
            godot_error!("BfSmokeEdge: no data image.");
            return;
        };
        data_image.fill(Color::from_rgba(0.0, 0.0, 0.0, 0.0));
        let Some(data_texture) = ImageTexture::create_from_image(&data_image) else {
            godot_error!("BfSmokeEdge: no data texture.");
            return;
        };

        let mesh = sheet_mesh();
        let huge = Aabb::new(
            Vector3::new(-2000.0, -500.0, -2000.0),
            Vector3::new(4000.0, 2000.0, 4000.0),
        );
        for outer in [0.0_f32, 1.0] {
            let mut material = make_material(SHEET_SHADER, &data_texture, &noise);
            material.set_shader_parameter("outer", &outer.to_variant());
            material.set_shader_parameter("bottom_m", &BOTTOM_M.to_variant());
            let mut node = MeshInstance3D::new_alloc();
            node.set_mesh(&mesh);
            node.set_material_override(&material.clone().upcast::<Material>());
            node.set_cast_shadows_setting(Shadow::OFF);
            node.set_custom_aabb(huge);
            node.set_visible(false);
            self.base_mut().add_child(&node);
            self.sheets.push(node);
            self.materials.push(material);
        }

        let material = make_material(CARD_SHADER, &data_texture, &noise);
        let mut quad = QuadMesh::new_gd();
        quad.set_size(Vector2::new(1.0, 1.0));
        quad.set_material(&material.clone().upcast::<Material>());
        let count = self.card_count.max(0) as usize;
        let mut multi = MultiMesh::new_gd();
        multi.set_transform_format(TransformFormat::TRANSFORM_3D);
        multi.set_use_custom_data(true);
        multi.set_mesh(&quad);
        multi.set_instance_count(count as i32);
        for i in 0..count {
            let h = |k: i32| hash01(i as i32, k, 0xCA2D);
            // Spread the cards around the circle with a little jitter.
            let angle_part = (i as f32 + h(0)) / count.max(1) as f32;
            let data = Color::from_rgba(
                angle_part.clamp(0.0, 0.999),
                h(1),
                0.05 + 0.55 * h(2),
                0.7 + 0.6 * h(3),
            );
            multi.set_instance_transform(i as i32, Transform3D::IDENTITY);
            multi.set_instance_custom_data(i as i32, data);
        }
        let mut cards = MultiMeshInstance3D::new_alloc();
        cards.set_multimesh(&multi);
        cards.set_cast_shadows_setting(Shadow::OFF);
        cards.set_custom_aabb(huge);
        cards.set_visible(false);
        self.base_mut().add_child(&cards);
        self.cards = Some(cards);
        self.materials.push(material);
        self.data_texture = Some(data_texture);
    }

    /// Gives the look values to the materials.
    fn apply_look(&mut self) {
        let (smoke, scroll, fade) = (self.smoke_color, self.scroll_speed, self.fade_distance_m);
        let (haze, haze_density) = (self.haze_color, self.haze_density);
        let (wall, card, size) = (self.wall_opacity, self.card_opacity, self.card_size_m);
        for (i, material) in self.materials.iter_mut().enumerate() {
            material.set_shader_parameter("smoke_color", &smoke.to_variant());
            material.set_shader_parameter("haze_color", &haze.to_variant());
            material.set_shader_parameter("haze_density", &haze_density.to_variant());
            material.set_shader_parameter("scroll", &scroll.to_variant());
            material.set_shader_parameter("fade_distance", &fade.to_variant());
            if i < 2 {
                material.set_shader_parameter("opacity", &wall.to_variant());
            } else {
                material.set_shader_parameter("opacity", &card.to_variant());
                material.set_shader_parameter("card_size", &size.to_variant());
            }
        }
    }

    /// Writes the shape of the wall at the time now into the data texture.
    fn write_texture(&mut self) {
        let (Some(edge), Some(texture)) = (&self.edge, &mut self.data_texture) else {
            return;
        };
        let t = self.clock as f32;
        let mut bytes = Vec::with_capacity(SAMPLES * 2 * 16);
        let mut density = Vec::with_capacity(SAMPLES);
        for i in 0..SAMPLES {
            let s = edge.sample(i as f32 * TAU / SAMPLES as f32, t);
            let ground = self.ground_y.get(i).copied().unwrap_or(0.0);
            for v in [s.front_m, s.height_m, s.band_m, ground] {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
            density.push(s.density);
        }
        for d in density {
            for v in [d, 0.0, 0.0, 0.0] {
                bytes.extend_from_slice(&v.to_le_bytes());
            }
        }
        let packed = PackedByteArray::from(bytes.as_slice());
        if let Some(image) =
            Image::create_from_data(SAMPLES as i32, 2, false, Format::RGBAF, &packed)
        {
            texture.update(&image);
        }
    }
}

/// Makes a material from a shader and gives it the two textures.
fn make_material(
    code: &str,
    data: &Gd<ImageTexture>,
    noise: &Gd<ImageTexture>,
) -> Gd<ShaderMaterial> {
    let mut shader = Shader::new_gd();
    shader.set_code(code);
    let mut material = ShaderMaterial::new_gd();
    material.set_shader(&shader);
    material.set_shader_parameter(
        "edge_data",
        &data.clone().upcast::<Texture2D>().to_variant(),
    );
    material.set_shader_parameter("puffs", &noise.clone().upcast::<Texture2D>().to_variant());
    material.set_shader_parameter("samples", &(SAMPLES as f32).to_variant());
    material
}

/// Makes the noise texture of the smoke (it repeats, with mipmaps).
fn noise_texture(seed: i64) -> Option<Gd<ImageTexture>> {
    let bytes = PackedByteArray::from(tile_noise(NOISE_SIZE, seed));
    let size = NOISE_SIZE as i32;
    let mut image = Image::create_from_data(size, size, false, Format::L8, &bytes)?;
    image.generate_mipmaps();
    ImageTexture::create_from_image(&image)
}

/// Makes the ring for a sheet. The vertex shader gives the real
/// positions. The positions here only make a plain mesh that holds the
/// angle in `UV.x` and the height part in `UV.y`.
fn sheet_mesh() -> Gd<ArrayMesh> {
    let mut positions = PackedVector3Array::new();
    let mut uvs = PackedVector2Array::new();
    let mut indices = PackedInt32Array::new();
    for i in 0..=SAMPLES {
        let part = i as f32 / SAMPLES as f32;
        let (s, c) = (part * TAU).sin_cos();
        for row in 0..2 {
            positions.push(Vector3::new(c * 800.0, row as f32 * 300.0, s * 800.0));
            uvs.push(Vector2::new(part, row as f32));
        }
    }
    for i in 0..SAMPLES as i32 {
        let (a, b) = (i * 2, i * 2 + 2);
        indices.extend([a, a + 1, b, b, a + 1, b + 1]);
    }
    let mut arrays = VarArray::new();
    arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
    arrays.set(ArrayType::VERTEX.ord() as usize, &positions.to_variant());
    arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
    arrays.set(ArrayType::INDEX.ord() as usize, &indices.to_variant());
    let mut mesh = ArrayMesh::new_gd();
    mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    mesh
}
