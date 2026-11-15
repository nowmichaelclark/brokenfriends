// The cave walls (S-164, S-166). Rust gives the colour in the vertex colour
// (linear light): the ground colour near a mouth, the rock colour deeper.
// The alpha of the vertex colour is the sky visibility (0 to 1). UV.x is
// the glow of a mineral and UV.y is its hue.
// The light is the same as outside (Lambert, no specular). The sky
// visibility goes to AO and also darkens the sun. The cave has its own
// fog: dark inside, smog colour near the sky.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_lambert, specular_disabled, fog_disabled;

// The grain texture of the ground (the same as on the terrain).
uniform sampler2D grain : source_color, filter_linear_mipmap, repeat_enable;
// The grain repeats each 1 / grain_scale metres (the terrain uses 0.25).
uniform float grain_scale = 0.25;
uniform float grain_strength : hint_range(0.0, 1.0) = 1.0;
uniform float brightness : hint_range(0.0, 2.0) = 1.0;
// The light of a cave with no sky (AO), so the bear is not in the dark.
uniform float dark_level : hint_range(0.0, 1.0) = 0.35;
// The fog of a cave with no sky: the colour and the density (1 / metre).
uniform vec3 cave_fog_colour : source_color = vec3(0.025, 0.028, 0.035);
uniform float cave_fog_density = 0.012;
// The fog near the sky (the same as the smog fog outside).
uniform vec3 smog_fog_colour : source_color = vec3(0.74, 0.60, 0.42);
uniform float smog_fog_density = 0.006;
// The glowing minerals: two colours and the strength of the light.
uniform vec3 mineral_colour_a : source_color = vec3(0.15, 0.85, 0.75);
uniform vec3 mineral_colour_b : source_color = vec3(0.95, 0.55, 0.15);
uniform float mineral_strength = 2.0;

varying vec2 grain_uv;

void vertex() {
	// One planar map: the plane that faces the normal most.
	vec3 n = abs(NORMAL);
	vec3 p = VERTEX * grain_scale;
	grain_uv = n.y >= max(n.x, n.z) ? p.xz : (n.x >= n.z ? p.zy : p.xy);
}

void fragment() {
	float g = texture(grain, grain_uv).r;
	float sky = clamp(COLOR.a, 0.0, 1.0);
	vec3 albedo = COLOR.rgb * mix(1.0, g, grain_strength) * brightness;
	AO = mix(dark_level, 1.0, sky);
	AO_LIGHT_AFFECT = 1.0;
	// The glow is a sharp core in a soft halo, in two colours.
	float glow = UV.x * UV.x;
	vec3 mineral = mix(mineral_colour_a, mineral_colour_b, step(0.5, UV.y));
	vec3 emission = mineral * (glow * mineral_strength + UV.x * 0.4);
	// The fog: the density and the colour follow the sky visibility.
	float fog = 1.0 - exp(-length(VERTEX) * mix(cave_fog_density, smog_fog_density, sky));
	vec3 fog_colour = mix(cave_fog_colour, smog_fog_colour, sky);
	ALBEDO = albedo * (1.0 - fog);
	EMISSION = emission * (1.0 - fog) + fog_colour * fog;
}
