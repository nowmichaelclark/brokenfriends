//! The smog, the sun, the sky and the dust (R7, R15). There is no heat
//! shimmer: Mikey removed it (R19).
//!
//! `BfAtmosphere` makes all of these parts in code. The Mobile renderer
//! has no volumetric fog, so the parts are cheap: depth fog, height
//! fog, a procedural sky and one particle system.

use crate::bear::BEAR_GROUP;
use godot::builtin::{Aabb, Color, Vector2, Vector3};
use godot::classes::environment::{BgMode, FogMode, ToneMapper};
use godot::classes::gpu_particles_3d::DrawOrder;
use godot::classes::particle_process_material::EmissionShape;
use godot::classes::{
    DirectionalLight3D, Environment, GpuParticles3D, INode3D, Material, Node3D,
    ParticleProcessMaterial, ProceduralSkyMaterial, QuadMesh, Shader, ShaderMaterial, Sky,
    WorldEnvironment,
};
use godot::prelude::*;

/// The shader for one dust particle: a soft, round, faint dot.
const DUST_SHADER: &str = "
shader_type spatial;
render_mode unshaded, blend_mix, depth_draw_never, cull_disabled, fog_disabled;
uniform vec4 tint : source_color = vec4(0.85, 0.72, 0.5, 1.0);
void fragment() {
    float d = length(UV - vec2(0.5)) * 2.0;
    float a = clamp(1.0 - d, 0.0, 1.0);
    ALBEDO = tint.rgb;
    ALPHA = a * a * tint.a * COLOR.a;
}
";

/// Makes the air of the zone: fog, sun, sky and dust. The
/// node makes its parts when the game starts. It reads its values again
/// twice each second, so a change in the Remote Inspector shows soon.
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct BfAtmosphere {
    /// The colour of the smog. Yellow-brown, not grey.
    #[export]
    #[init(val = Color::from_rgb(0.74, 0.60, 0.42))]
    fog_color: Color,

    /// How fast the smog gets thick with distance. At 0.006, an object
    /// 150 m away has 60% smog and one at 500 m has 95%.
    #[export]
    #[init(val = 0.006)]
    fog_density: f32,

    /// The height in metres under which the low smog starts. The smog
    /// gets thicker below this height.
    #[export]
    #[init(val = 90.0)]
    smog_height: f32,

    /// How much thicker the low smog is. Zero turns off the low smog.
    #[export]
    #[init(val = 0.004)]
    smog_height_density: f32,

    /// How much of the smog colour goes over the sky near the horizon
    /// (0 to 1). A high value hides the border between land and sky.
    #[export]
    #[init(val = 0.85)]
    fog_sky_affect: f32,

    /// How much the smog glows toward the sun (0 to 1).
    #[export]
    #[init(val = 0.25)]
    fog_sun_scatter: f32,

    /// The colour of the sky at the top.
    #[export]
    #[init(val = Color::from_rgb(0.55, 0.50, 0.42))]
    sky_top_color: Color,

    /// The colour of the sky at the horizon. It should be near the
    /// smog colour.
    #[export]
    #[init(val = Color::from_rgb(0.74, 0.60, 0.42))]
    sky_horizon_color: Color,

    /// The colour of the light of the sun and of the sun disc.
    #[export]
    #[init(val = Color::from_rgb(1.0, 0.90, 0.72))]
    sun_color: Color,

    /// The strength of the sun light.
    #[export]
    #[init(val = 0.9)]
    sun_energy: f32,

    /// The height of the sun over the horizon in degrees.
    #[export]
    #[init(val = 38.0)]
    sun_elevation_degrees: f32,

    /// The direction of the sun, in degrees around the vertical axis.
    #[export]
    #[init(val = 215.0)]
    sun_azimuth_degrees: f32,

    /// The size of the sun disc in degrees. A big, pale disc looks
    /// like a sun behind smog.
    #[export]
    #[init(val = 4.0)]
    sun_disc_degrees: f32,

    /// The strength of the ambient light from the sky (0 to 1).
    #[export]
    #[init(val = 0.55)]
    ambient_energy: f32,

    /// The shadow distance of the sun in metres. This is a cost: a
    /// longer distance draws more objects into the shadow map.
    #[export]
    #[init(val = 100.0)]
    shadow_distance: f32,

    /// The colour saturation after grading. Below 1 makes the colours
    /// look faded.
    #[export]
    #[init(val = 0.8)]
    grade_saturation: f32,

    /// The contrast after grading. Below 1 makes the picture flat.
    #[export]
    #[init(val = 0.95)]
    grade_contrast: f32,

    /// The brightness after grading.
    #[export]
    #[init(val = 1.0)]
    grade_brightness: f32,

    /// If this is on, dust and ash move past the camera.
    #[export]
    #[init(val = true)]
    dust_enabled: bool,

    /// The number of dust particles. This is a cost, but a small one.
    #[export]
    #[init(val = 300)]
    dust_count: i32,

    /// The half width of the box of dust around the camera, in metres.
    #[export]
    #[init(val = 8.0)]
    dust_radius: f32,

    /// The size of one dust particle in metres.
    #[export]
    #[init(val = 0.025)]
    dust_size: f32,

    /// The speed of the wind that moves the dust, in metres per second.
    #[export]
    #[init(val = 1.2)]
    dust_wind_speed: f32,

    environment: Option<Gd<Environment>>,
    sky_material: Option<Gd<ProceduralSkyMaterial>>,
    sun: Option<Gd<DirectionalLight3D>>,
    dust: Option<Gd<GpuParticles3D>>,
    dust_process: Option<Gd<ParticleProcessMaterial>>,
    dust_mesh: Option<Gd<QuadMesh>>,
    time_to_refresh: f64,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for BfAtmosphere {
    fn ready(&mut self) {
        self.make_environment();
        self.make_sun();
        self.make_dust();
        self.apply();
    }

    fn process(&mut self, delta: f64) {
        self.follow_camera();
        self.time_to_refresh -= delta;
        if self.time_to_refresh <= 0.0 {
            self.time_to_refresh = 0.5;
            self.apply();
        }
    }
}

#[godot_api]
impl BfAtmosphere {
    /// Reads all exported values again and gives them to the parts.
    #[func]
    pub fn apply(&mut self) {
        self.apply_environment();
        self.apply_sun();
        self.apply_dust();
    }
}

impl BfAtmosphere {
    fn make_environment(&mut self) {
        let mut sky_material = ProceduralSkyMaterial::new_gd();
        // The ground colours are the smog colour, so the horizon has no line.
        sky_material.set_sun_angle_max(self.sun_disc_degrees);
        sky_material.set_sun_curve(0.08);

        let mut sky = Sky::new_gd();
        sky.set_material(&sky_material);

        let mut env = Environment::new_gd();
        env.set_background(BgMode::SKY);
        env.set_sky(&sky);
        env.set_tonemapper(ToneMapper::FILMIC);
        env.set_fog_enabled(true);
        env.set_fog_mode(FogMode::EXPONENTIAL);
        env.set_adjustment_enabled(true);

        let mut world = WorldEnvironment::new_alloc();
        world.set_environment(&env);
        self.base_mut().add_child(&world);

        self.environment = Some(env);
        self.sky_material = Some(sky_material);
    }

    fn make_sun(&mut self) {
        let mut sun = DirectionalLight3D::new_alloc();
        sun.set_shadow(true);
        // Two cascades, as the budget says. Soft shadows are off.
        sun.set_shadow_mode(godot::classes::directional_light_3d::ShadowMode::PARALLEL_2_SPLITS);
        sun.set_param(godot::classes::light_3d::Param::SHADOW_BLUR, 0.0);
        self.base_mut().add_child(&sun);
        self.sun = Some(sun);
    }

    fn make_dust(&mut self) {
        let mut shader = Shader::new_gd();
        shader.set_code(DUST_SHADER);
        let mut material = ShaderMaterial::new_gd();
        material.set_shader(&shader);

        let mut mesh = QuadMesh::new_gd();
        mesh.set_material(&material.upcast::<Material>());

        let process = ParticleProcessMaterial::new_gd();

        let mut particles = GpuParticles3D::new_alloc();
        particles.set_process_material(&process);
        particles.set_draw_pass_mesh(0, &mesh);
        particles.set_draw_order(DrawOrder::INDEX);
        // The particles stay in the world while the camera moves.
        particles.set_use_local_coordinates(false);
        particles.set_lifetime(10.0);
        particles.set_pre_process_time(10.0);
        self.base_mut().add_child(&particles);

        self.dust = Some(particles);
        self.dust_process = Some(process);
        self.dust_mesh = Some(mesh);
    }

    fn apply_environment(&mut self) {
        let Some(env) = &mut self.environment else {
            return;
        };
        env.set_fog_light_color(self.fog_color);
        env.set_fog_density(self.fog_density);
        env.set_fog_height(self.smog_height);
        env.set_fog_height_density(self.smog_height_density);
        env.set_fog_sky_affect(self.fog_sky_affect);
        env.set_fog_sun_scatter(self.fog_sun_scatter);
        env.set_ambient_source(godot::classes::environment::AmbientSource::SKY);
        env.set_ambient_light_energy(self.ambient_energy);
        env.set_adjustment_saturation(self.grade_saturation);
        env.set_adjustment_contrast(self.grade_contrast);
        env.set_adjustment_brightness(self.grade_brightness);

        if let Some(sky) = &mut self.sky_material {
            sky.set_sky_top_color(self.sky_top_color);
            sky.set_sky_horizon_color(self.sky_horizon_color);
            sky.set_ground_horizon_color(self.sky_horizon_color);
            sky.set_ground_bottom_color(self.fog_color.darkened(0.25));
            sky.set_sun_angle_max(self.sun_disc_degrees);
        }
    }

    fn apply_sun(&mut self) {
        let Some(sun) = &mut self.sun else {
            return;
        };
        sun.set_color(self.sun_color);
        sun.set_param(godot::classes::light_3d::Param::ENERGY, self.sun_energy);
        sun.set_param(
            godot::classes::light_3d::Param::SIZE,
            self.sun_disc_degrees * 0.5,
        );
        sun.set_param(
            godot::classes::light_3d::Param::SHADOW_MAX_DISTANCE,
            self.shadow_distance,
        );
        // The light shines along -Z of the node. Turn the node so that
        // -Z points from the sun to the ground.
        let azimuth = self.sun_azimuth_degrees.to_radians();
        let elevation = self.sun_elevation_degrees.to_radians();
        sun.set_rotation(Vector3::new(-elevation, azimuth, 0.0));
    }

    fn apply_dust(&mut self) {
        let (Some(particles), Some(process)) = (&mut self.dust, &mut self.dust_process) else {
            return;
        };
        particles.set_emitting(self.dust_enabled);
        particles.set_amount(self.dust_count.max(1));
        let r = self.dust_radius.max(1.0);
        particles.set_visibility_aabb(Aabb::new(
            Vector3::new(-r, -r, -r),
            Vector3::new(2.0 * r, 2.0 * r, 2.0 * r),
        ));
        process.set_emission_shape(EmissionShape::BOX);
        process.set_emission_box_extents(Vector3::new(r, r * 0.5, r));
        // The wind blows along +X and a little up. The dust drifts.
        process.set_direction(Vector3::new(1.0, 0.1, 0.2));
        process.set_spread(25.0);
        process.set_param_min(
            godot::classes::particle_process_material::Parameter::INITIAL_LINEAR_VELOCITY,
            self.dust_wind_speed * 0.5,
        );
        process.set_param_max(
            godot::classes::particle_process_material::Parameter::INITIAL_LINEAR_VELOCITY,
            self.dust_wind_speed * 1.5,
        );
        process.set_gravity(Vector3::new(0.0, -0.02, 0.0));
        process.set_param_min(
            godot::classes::particle_process_material::Parameter::SCALE,
            0.5,
        );
        process.set_param_max(
            godot::classes::particle_process_material::Parameter::SCALE,
            1.5,
        );
        if let Some(mesh) = &mut self.dust_mesh {
            mesh.set_size(Vector2::new(self.dust_size, self.dust_size));
        }
    }

    /// Moves the dust box to the camera, or to the bear if no camera is
    /// current. The dust particles stay in the world.
    fn follow_camera(&mut self) {
        let Some(mut particles) = self.dust.clone() else {
            return;
        };
        if !self.base().is_inside_tree() {
            return;
        }
        let viewport = self.base().get_viewport();
        let position = if let Some(camera) = viewport.and_then(|v| v.get_camera_3d()) {
            Some(camera.get_global_position())
        } else {
            self.base()
                .get_tree()
                .get_first_node_in_group(BEAR_GROUP)
                .and_then(|n| n.try_cast::<Node3D>().ok())
                .map(|n| n.get_global_position())
        };
        if let Some(position) = position {
            particles.set_global_position(position);
        }
    }
}
