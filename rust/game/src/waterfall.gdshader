// The waterfalls of Broken Friends (S-165, plan/demo-2.md, "Waterfalls").
//
// Rust (waterfall_mesh.rs) makes two surfaces and bakes the data into
// the vertices (see the comment of that file). One shader code makes two
// materials:
//
//   CURTAIN  The sheet of falling water. Opaque with an alpha scissor
//            (no blending): a scrolling noise gives streaks and a ragged
//            edge. A thick fall is a few slow ropes.
//   EFFECTS  The foam on the pool, the splash and the mist. Blended.
//            Splash and mist are cards that face the camera and move in
//            the vertex shader, so there are no particle nodes. Far
//            cards shrink to nothing, and a mist card near the camera
//            fades out, so the mist never fills the screen.
//
// Cost: each pixel reads one or two noise values. No screen texture, no
// depth texture, no lights (a flat light colour), no shadows.
shader_type spatial;
/*DEFINES*/
#ifdef CURTAIN
render_mode unshaded, cull_disabled, depth_draw_opaque;
#else
render_mode unshaded, cull_disabled, blend_mix, depth_draw_never, skip_vertex_transform;
#endif

uniform sampler2D noise : filter_linear_mipmap, repeat_enable;
// The light on the water: the ambient light and a part of the sun.
uniform vec3 light = vec3(0.8);
uniform vec3 foam_colour : source_color = vec3(0.92, 0.9, 0.85);
uniform float foam_strength = 0.85;
// Mist cards fade out beyond this camera distance, in metres.
uniform float mist_far_m = 250.0;
uniform float mist_alpha = 0.3;
// Splash cards fade out beyond this camera distance, in metres.
uniform float splash_far_m = 160.0;
// 1.0 shows the effect, 0.0 hides it (the cuts of the GPU budget).
uniform float mist_on = 1.0;
uniform float splash_on = 1.0;
// How wide a streak is, in metres, and how long.
uniform vec2 streak_m = vec2(1.6, 7.0);

#ifdef CURTAIN
varying vec3 tint;
varying float mask;
varying vec2 st;
varying float speed;
varying float thick;
varying float half_w;

void vertex() {
	tint = COLOR.rgb;
	mask = COLOR.a;
	st = UV;
	speed = UV2.x;
	thick = UV2.y >= 500.0 ? 1.0 : 0.0;
	half_w = UV2.y - thick * 1000.0;
}

void fragment() {
	float scroll = TIME * speed;
	float n1 = texture(noise, vec2(st.x / streak_m.x, (st.y - scroll) / streak_m.y)).r;
	float n2 = texture(noise, vec2(st.x / (streak_m.x * 0.55) + 0.37, (st.y - scroll * 1.35) / (streak_m.y * 0.6))).r;
	float n = n1 * 0.6 + n2 * 0.4;
	// Less cover at the edge: a ragged edge.
	float edge = 1.0 - abs(st.x) / max(half_w, 0.1);
	float density = mask * smoothstep(0.0, 0.3, edge);
	float cover = density * 0.8 + (n - 0.5) * 0.9;
	if (thick > 0.5) {
		// A few ropes: columns that stay in place and fall slowly.
		float rope = texture(noise, vec2(st.x * 0.22 + 0.13, 0.5)).r;
		cover = density * smoothstep(0.5, 0.58, rope) + (n - 0.5) * 0.4 - 0.2;
	}
	ALPHA = cover;
	ALPHA_SCISSOR_THRESHOLD = 0.5;
	float froth = smoothstep(0.5, 0.78, n);
	vec3 water = tint * 1.5 + vec3(0.08);
	vec3 col = mix(water, foam_colour, froth * 0.7 + 0.12);
	ALBEDO = col * light;
}
#else
varying vec2 corner;
varying float kind;
varying float seed;
varying vec3 tint;
varying float fade;

void vertex() {
	vec3 centre = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	corner = UV;
	kind = UV2.x;
	seed = UV2.y;
	tint = COLOR.rgb;
	float size = COLOR.a;
	float dist = distance(centre, CAMERA_POSITION_WORLD);
	vec3 world = centre;
	float s = size;
	fade = 1.0;
	if (kind > 2.5) {
		// Splash: a drop that jumps from the pool and falls back.
		float rate = 0.8 + 0.6 * fract(seed * 13.7);
		float ph = fract(TIME * rate + seed * 7.0);
		float a = seed * 62.83;
		float up = 3.0 + 6.0 * fract(seed * 5.3);
		float out_s = 0.8 + 2.2 * fract(seed * 9.1);
		float t = ph * 1.2;
		world += vec3(cos(a) * out_s * t, up * t - 4.9 * t * t, sin(a) * out_s * t);
		fade = (1.0 - ph) * splash_on * smoothstep(splash_far_m, splash_far_m * 0.6, dist);
		s = size * (1.0 - 0.5 * ph);
	} else if (kind > 1.5) {
		// Mist: a soft card that rises and spreads.
		float ph = fract(TIME * 0.08 + seed);
		world += vec3(sin(seed * 40.0 + TIME * 0.3) * size * 0.25, ph * size * 0.7, cos(seed * 40.0 + TIME * 0.3) * size * 0.25);
		fade = sin(ph * 3.14159) * mist_alpha * mist_on
			* smoothstep(mist_far_m, mist_far_m * 0.6, dist)
			* smoothstep(size * 0.4, size * 1.4, dist);
		s = size * (0.8 + 0.5 * ph);
	}
	vec3 view = (VIEW_MATRIX * vec4(world, 1.0)).xyz;
	if (kind > 1.5) {
		view.xy += corner * (fade > 0.002 ? s : 0.0);
	}
	VERTEX = view;
}

void fragment() {
	float r = length(corner);
	float alpha = 0.0;
	vec3 col = foam_colour * light;
	if (kind < 1.5) {
		// Foam on the pool: dense near the fall, thin and broken far.
		vec2 p = corner * 3.0;
		float n = texture(noise, p + vec2(TIME * 0.03, -TIME * 0.05) + seed).r;
		float n2 = texture(noise, p * 1.9 - vec2(TIME * 0.04, TIME * 0.02)).r;
		float body = clamp((n * 0.6 + n2 * 0.4) * 1.9 - 0.25 - r * 0.9, 0.0, 1.0);
		alpha = body * (1.0 - smoothstep(0.6, 1.0, r)) * foam_strength;
	} else if (kind < 2.5) {
		float soft = 1.0 - smoothstep(0.0, 1.0, r);
		float n = texture(noise, corner * 0.8 + seed * 5.0).r;
		alpha = soft * soft * (0.5 + 0.8 * n) * fade;
		col = mix(foam_colour, tint, 0.4) * light;
	} else {
		alpha = (1.0 - smoothstep(0.0, 1.0, r)) * 0.8 * fade;
	}
	if (alpha < 0.01) {
		discard;
	}
	ALBEDO = col;
	ALPHA = clamp(alpha, 0.0, 1.0);
}
#endif
