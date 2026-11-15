// The light terrain shader of Broken Friends (S-139).
//
// It is made from the "minimum" and "lightweight" shaders of Terrain3D
// v1.1.0 (MIT licence, Copyright (c) 2023-2026 Cory Petkovsek, Roope
// Palmroos, and Contributors). Terrain3D gives the private uniforms.
//
// What it does, for the R15 budget on integrated graphics:
// - The vertex shader makes the height and the normal (3 height reads).
// - The fragment shader reads only the colour map and one grain texture.
// - Lambert light and no specular. The ground is dry dust and rock.
// No texture blend, no control map in the fragment, no displacement.
// Mesh tessellation must be 0.

shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_back, diffuse_lambert, specular_disabled, skip_vertex_transform;

#define MAX_REGIONS 1024 // Must be the same as Terrain3DMaterial.max_regions.

// Private uniforms. Terrain3D sets them.
group_uniforms private;
uniform vec3 _target_pos = vec3(0.f);
uniform float _mesh_size = 48.f;
uniform float _subdiv = 1.f;
uniform uint _background_mode = 1u; // NONE = 0, FLAT = 1, NOISE = 2
uniform uint _mouse_layer = 0x80000000u;
uniform float _vertex_spacing = 1.0;
uniform float _vertex_density = 1.0;
uniform float _region_size = 1024.0;
uniform float _region_texel_size = 0.0009765625;
uniform int _region_map_size = 32;
uniform int _region_map[1024];
uniform vec2 _region_locations[MAX_REGIONS];
uniform float _texture_uv_scale_array[32];
uniform highp sampler2DArray _height_maps : repeat_disable;
uniform highp sampler2DArray _control_maps : repeat_disable;
uniform highp sampler2DArray _color_maps : source_color, filter_linear_mipmap, repeat_disable;
uniform highp sampler2DArray _texture_array_albedo : source_color, filter_linear_mipmap, repeat_enable;
group_uniforms;

// Public uniforms. BfTerrain sets them.
group_uniforms broken_friends;
// How much of the grain texture shows (0 = none, 1 = all of it).
uniform float grain_strength : hint_range(0.0, 1.0) = 1.0;
group_uniforms;

varying vec3 v_vertex;

// World XZ to (x, y) in the region and the region layer (-1: no region).
ivec3 get_index_coord(const vec2 uv) {
	vec2 r_uv = round(uv);
	ivec2 pos = ivec2(floor(r_uv * _region_texel_size)) + (_region_map_size / 2);
	int bounds = int(uint(pos.x | pos.y) < uint(_region_map_size));
	int map_val = _region_map[pos.y * _region_map_size + pos.x];
	int raw_index = map_val - 1;
	int is_region = bounds * int(raw_index >= 0) * int(raw_index < MAX_REGIONS);
	int is_dummy = bounds * int(map_val < 0);
	int layer_index = is_region * raw_index + is_dummy * (map_val - 1) - (1 - is_region - is_dummy);
	return ivec3(ivec2(mod(r_uv, _region_size)), layer_index);
}

// Region space (world / region size) to (0..1) in the region and the layer.
vec3 get_index_uv(const vec2 uv2) {
	ivec2 pos = ivec2(floor(uv2)) + (_region_map_size / 2);
	int bounds = int(uint(pos.x | pos.y) < uint(_region_map_size));
	int map_val = _region_map[pos.y * _region_map_size + pos.x];
	int raw_index = map_val - 1;
	int is_region = bounds * int(raw_index >= 0) * int(raw_index < MAX_REGIONS);
	int is_dummy = bounds * int(map_val < 0);
	int layer_index = is_region * raw_index + is_dummy * (map_val - 1) - (1 - is_region - is_dummy);
	return vec3(uv2 - _region_locations[max(layer_index, 0)], float(layer_index));
}

bool is_none_bg(const vec2 uv) {
	ivec4 regions = ivec4(
		get_index_coord(uv - vec2(0.5, 0.0)).z,
		get_index_coord(uv - vec2(0.0, 0.5)).z,
		get_index_coord(uv + vec2(1.0, 0.0)).z,
		get_index_coord(uv + vec2(0.0, 1.0)).z);
	return any(equal(regions, ivec4(-1)));
}

float interpolated_height(vec2 pos) {
	const vec2 offsets = vec2(0, 1);
	vec2 index_id = floor(pos);
	float h0 = texelFetch(_height_maps, get_index_coord(index_id + offsets.xy), 0).r;
	float h1 = texelFetch(_height_maps, get_index_coord(index_id + offsets.yy), 0).r;
	float h2 = texelFetch(_height_maps, get_index_coord(index_id + offsets.yx), 0).r;
	float h3 = texelFetch(_height_maps, get_index_coord(index_id + offsets.xx), 0).r;
	vec2 f = fract(pos);
	vec2 i = 1.0 - f;
	vec4 w = vec4(i.x * f.y, f.x * f.y, f.x * i.y, i.x * i.y);
	return h0 * w[0] + h1 * w[1] + h2 * w[2] + h3 * w[3];
}

void vertex() {
	v_vertex = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;

	// Geomorph across the clipmap LODs (the same as Terrain3D).
	float scale = MODEL_MATRIX[0][0];
	float inv_scale = 1.0 / scale;
	float max_xz = max(abs(v_vertex.x - _target_pos.x), abs(v_vertex.z - _target_pos.z));
	float vertex_lerp = smoothstep(0.0, 1.0, (max_xz * inv_scale - _mesh_size - 4.0) / (_mesh_size - 4.0));
	vec2 vertex_fract = fract(VERTEX.xz * 0.5) * 2.0;
	vec2 shift = (scale < _vertex_spacing / _subdiv + 1e-6) ?
		mix(vertex_fract, vec2(vertex_fract.x, -vertex_fract.y),
			round(fract(round(mod(v_vertex.z * inv_scale, 4.0)) *
			round(mod(v_vertex.x * inv_scale, 4.0)) * 0.25))) :
		vertex_fract * round((fract(v_vertex.xz * 0.25 * inv_scale) - 0.5) * 4.0);
	vec2 start_pos = v_vertex.xz * _vertex_density;
	vec2 end_pos = (v_vertex.xz - shift * scale) * _vertex_density;
	v_vertex.xz -= shift * scale * vertex_lerp;

	UV = v_vertex.xz * _vertex_density;
	UV2 = fma(UV, vec2(_region_texel_size), vec2(0.5 * _region_texel_size));

	ivec3 v_region = get_index_coord(start_pos);
	uint control = floatBitsToUint(texelFetch(_control_maps, v_region, 0)).r;
	bool hole = bool(control >> 2u & 0x1u);

	vec3 w_normal = vec3(0.0, 1.0, 0.0);
	if (!(CAMERA_VISIBLE_LAYERS == _mouse_layer) &&
			(hole || (_background_mode == 0u && is_none_bg(UV)))) {
		v_vertex.x = 0. / 0.;
	} else {
		float h, u, v;
		if (scale < _vertex_spacing) {
			h = interpolated_height(UV);
			u = interpolated_height(UV + vec2(1, 0));
			v = interpolated_height(UV + vec2(0, 1));
		} else {
			h = mix(texelFetch(_height_maps, get_index_coord(start_pos), 0).r,
				texelFetch(_height_maps, get_index_coord(end_pos), 0).r, vertex_lerp);
			u = mix(texelFetch(_height_maps, get_index_coord(start_pos + vec2(1, 0)), 0).r,
				texelFetch(_height_maps, get_index_coord(end_pos + vec2(1, 0)), 0).r, vertex_lerp);
			v = mix(texelFetch(_height_maps, get_index_coord(start_pos + vec2(0, 1)), 0).r,
				texelFetch(_height_maps, get_index_coord(end_pos + vec2(0, 1)), 0).r, vertex_lerp);
		}
		v_vertex.y = h;
		w_normal = normalize(vec3(h - u, _vertex_spacing, h - v));
	}

	VERTEX = (VIEW_MATRIX * vec4(v_vertex, 1.0)).xyz;
	NORMAL = mat3(VIEW_MATRIX) * w_normal;
}

void fragment() {
	vec2 uv2 = UV2; // A built-in cannot go into a const parameter.
	vec3 region_uv = get_index_uv(uv2);
	vec3 colour = region_uv.z > -1.0 ? texture(_color_maps, region_uv).rgb : vec3(0.5);
	float grain = texture(_texture_array_albedo, vec3(v_vertex.xz * _texture_uv_scale_array[0], 0.0)).r;
	ALBEDO = colour * mix(1.0, grain, grain_strength);
}
