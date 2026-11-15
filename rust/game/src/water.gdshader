// The still water of Broken Friends (S-160, plan/demo-2.md, "Water look").
//
// Rust (water_mesh.rs) bakes the look into the vertices:
//   COLOR.rgb  the deep colour of the body (linear)
//   COLOR.a    the clarity / clarity_scale (the depth in metres that hides
//              about 63% of the ground)
//   UV.x       the water depth at the vertex: level minus ground (negative
//              on dry ground at the shore)
//   UV.y       foam (0 for still water, later rows use it)
//   UV2        the flow vector in m/s (0 for still water, rivers use it)
//
// Cost (S-180): the render mode of the blended water is `blend_mix`. The
// opaque deep quads use the same code with OPAQUE_WATER (no blending).
// FAKE_GLINT (unshaded water) replaces the lights of Godot with a
// Lambert term of the sun, the flat ambient light and a sun glint.
//
// The shader reads no screen texture and no depth texture: the depth in
// the vertices gives the colour and the opacity. There are no
// reflections and no refraction. The fog of the environment applies.
shader_type spatial;
/*DEFINES*/
render_mode /*BLEND_MODES*/cull_back, diffuse_lambert, specular_schlick_ggx/*EXTRA_MODES*/;

uniform sampler2D ripples : filter_linear_mipmap, repeat_enable;
// 0, 1 or 2 ripple layers (the first cut of the GPU budget is 1).
uniform int ripple_layers = 2;
// The size of one tile of the ripple texture, in metres.
uniform float ripple_scale_m = 5.0;
// How fast the ripples move, in tiles each second.
uniform float ripple_speed = 0.03;
// How much the ripples bend the normal.
uniform float ripple_strength = 0.18;
// The water fades out above this depth at the shore, in metres (0: no fade).
uniform float shore_fade_m = 0.15;
// The opacity that the edge of the view adds (Fresnel).
uniform float fresnel_alpha = 0.45;
// The least opacity of thin, clear water.
uniform float min_alpha = 0.1;
uniform float roughness = 0.12;
uniform float specular = 0.5;
// The clarity in COLOR.a is clarity / clarity_scale.
uniform float clarity_scale = 8.0;
// The water has no reflections. At a flat view it gets this pale colour
// of the smog sky instead, so it reads as water.
uniform vec3 sky_colour : source_color = vec3(0.82, 0.74, 0.60);
uniform float sky_reflect = 0.35;

// The sun for the glint of unshaded water (FAKE_GLINT): the direction
// toward the sun and its colour with the strength.
uniform vec3 sun_dir = vec3(0.0, 1.0, 0.0);
uniform vec3 sun_light = vec3(1.0);
// The flat ambient light, for the diffuse light of unshaded water.
uniform vec3 ambient_light = vec3(0.5);

varying vec3 world_pos;
varying float depth;
varying vec4 look;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	depth = UV.x;
	look = COLOR;
}

vec3 ripple_normal(vec2 p) {
	vec3 n = vec3(0.0, 1.0, 0.0);
	vec2 uv = p / ripple_scale_m;
	float t = TIME * ripple_speed;
	if (ripple_layers > 0) {
		vec2 a = texture(ripples, uv + vec2(t, t * 0.6)).xy * 2.0 - 1.0;
		n.xz += a * ripple_strength;
	}
	if (ripple_layers > 1) {
		vec2 b = texture(ripples, uv * 2.3 + vec2(-t * 0.8, t * 1.1)).xy * 2.0 - 1.0;
		n.xz += b * ripple_strength * 0.6;
	}
	return normalize(n);
}

void fragment() {
	float clarity = max(look.a * clarity_scale, 0.05);
	float d = max(depth, 0.0);
	float opacity = 1.0 - exp(-d / clarity);
	vec3 world_normal = ripple_normal(world_pos.xz);
	NORMAL = normalize((VIEW_MATRIX * vec4(world_normal, 0.0)).xyz);
	float facing = clamp(dot(NORMAL, VIEW), 0.0, 1.0);
	float fresnel = pow(1.0 - facing, 5.0);
	float shore = shore_fade_m > 0.0 ? smoothstep(0.0, shore_fade_m, depth) : step(0.0, depth);
#ifdef FAKE_GLINT
	// Unshaded water has no lights: a Lambert term of the sun and the
	// flat ambient light replace them. There are no shadows.
	// The shaded water also reflects the flat ambient light, more at a
	// flat view.
	float reflectance = 0.04 + 0.96 * pow(1.0 - facing, 3.0);
	ALBEDO = look.rgb * (ambient_light + sun_light * max(dot(world_normal, sun_dir), 0.0)) * (0.75 * (1.0 - reflectance));
#else
	ALBEDO = look.rgb;
#endif
#ifndef OPAQUE_WATER
	ALPHA = clamp(max(opacity, min_alpha) + fresnel * fresnel_alpha, 0.0, 1.0) * shore;
#endif
	EMISSION = sky_colour * fresnel * sky_reflect;
#ifdef FAKE_GLINT
	// Unshaded water has no light, so the sun glint is made here.
	vec3 to_eye = normalize(CAMERA_POSITION_WORLD - world_pos);
	vec3 mirror = reflect(-to_eye, world_normal);
	float lobe = max(2.0 / (roughness * roughness + 0.0001) - 2.0, 1.0);
	EMISSION += ambient_light * reflectance;
	EMISSION += sun_light * specular * pow(max(dot(mirror, sun_dir), 0.0), min(lobe, 200.0));
#endif
	ROUGHNESS = roughness;
	SPECULAR = specular;
	METALLIC = 0.0;
}
