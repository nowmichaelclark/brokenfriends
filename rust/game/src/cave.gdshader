// The cave walls (S-164). Rust gives the colour in the vertex colour
// (linear light): the ground colour near a mouth, the rock colour deeper.
// The light is the same as outside (Lambert, no specular), like the
// light terrain shader. S-166 adds the sky visibility (AO) and the cave
// fog.
shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_lambert, specular_disabled;

// The grain texture of the ground (the same as on the terrain).
uniform sampler2D grain : source_color, filter_linear_mipmap, repeat_enable;
// The grain repeats each 1 / grain_scale metres (the terrain uses 0.25).
uniform float grain_scale = 0.25;
uniform float grain_strength : hint_range(0.0, 1.0) = 1.0;
uniform float brightness : hint_range(0.0, 2.0) = 1.0;

varying vec2 grain_uv;

void vertex() {
	// One planar map: the plane that faces the normal most.
	vec3 n = abs(NORMAL);
	vec3 p = VERTEX * grain_scale;
	grain_uv = n.y >= max(n.x, n.z) ? p.xz : (n.x >= n.z ? p.zy : p.xy);
}

void fragment() {
	float g = texture(grain, grain_uv).r;
	ALBEDO = COLOR.rgb * mix(1.0, g, grain_strength) * brightness;
}
