class_name FxLayer
extends Node2D
## Small, cheap particles drawn in code: fluff that drifts and settles,
## hydraulic fluid, soap bubbles, water drips, mud flakes, sparkles and
## sleepy z's.

## Fluff and flakes come to rest here (world y).
var floor_y := 900.0

var _parts: Array = []
var _rng := RandomNumberGenerator.new()


func _ready() -> void:
	_rng.randomize()


func spawn(kind: String, at: Vector2, count := 1, spread := 10.0) -> void:
	for i in count:
		var p := {
			"kind": kind,
			"pos": at + Vector2(_rng.randf_range(-spread, spread), _rng.randf_range(-spread, spread)),
			"vel": Vector2.ZERO, "life": 1.0, "max": 1.0, "size": 6.0,
			"spin": _rng.randf_range(-2.0, 2.0), "rest": false,
		}
		match kind:
			"fluff":
				p["vel"] = Vector2(_rng.randf_range(-160, 160), _rng.randf_range(-260, -40))
				p["life"] = _rng.randf_range(2.5, 4.0)
				p["size"] = _rng.randf_range(5.0, 11.0)
			"fluid":
				p["vel"] = Vector2(_rng.randf_range(-140, 140), _rng.randf_range(-220, 20))
				p["life"] = _rng.randf_range(0.8, 1.4)
				p["size"] = _rng.randf_range(3.0, 6.0)
			"bubble":
				p["vel"] = Vector2(_rng.randf_range(-40, 40), _rng.randf_range(-90, -30))
				p["life"] = _rng.randf_range(0.7, 1.3)
				p["size"] = _rng.randf_range(4.0, 10.0)
			"drip":
				p["vel"] = Vector2(_rng.randf_range(-10, 10), _rng.randf_range(0, 40))
				p["life"] = 1.2
				p["size"] = _rng.randf_range(2.5, 4.0)
			"mud":
				p["vel"] = Vector2(_rng.randf_range(-60, 60), _rng.randf_range(-60, 10))
				p["life"] = 1.0
				p["size"] = _rng.randf_range(2.5, 5.0)
			"sparkle":
				p["vel"] = Vector2(_rng.randf_range(-60, 60), _rng.randf_range(-90, -20))
				p["life"] = 0.7
				p["size"] = _rng.randf_range(5.0, 9.0)
			"z":
				p["vel"] = Vector2(25, -45)
				p["life"] = 1.8
				p["size"] = 26.0
			"heart":
				p["vel"] = Vector2(_rng.randf_range(-20, 20), -60)
				p["life"] = 1.2
				p["size"] = 12.0
		p["max"] = p["life"]
		_parts.append(p)


func _process(delta: float) -> void:
	if _parts.is_empty():
		return
	var keep: Array = []
	for p in _parts:
		p["life"] -= delta
		if p["life"] <= 0.0:
			continue
		var v: Vector2 = p["vel"]
		var pos: Vector2 = p["pos"]
		match p["kind"]:
			"fluff":
				if not p["rest"]:
					v.y += 120.0 * delta
					v *= 1.0 - 1.6 * delta
					v.x += sin(p["life"] * 3.0 + p["spin"]) * 20.0 * delta
					if pos.y > floor_y:
						pos.y = floor_y
						p["rest"] = true
						v = Vector2.ZERO
			"fluid", "drip", "mud":
				v.y += 900.0 * delta
				if pos.y > floor_y:
					p["life"] = minf(p["life"], 0.15)
					v = Vector2.ZERO
			"bubble", "sparkle", "z", "heart":
				v *= 1.0 - 0.8 * delta
		pos += v * delta
		p["vel"] = v
		p["pos"] = pos
		keep.append(p)
	_parts = keep
	queue_redraw()


func _draw() -> void:
	var font := ThemeDB.fallback_font
	for p in _parts:
		var pos: Vector2 = p["pos"]
		var s: float = p["size"]
		var a: float = clampf(p["life"] / p["max"] * 2.0, 0.0, 1.0)
		match p["kind"]:
			"fluff":
				PlushArt.fluff(self, pos, s, a)
			"fluid":
				var c := PlushArt.FLUID
				c.a = a
				draw_circle(pos, s, c)
			"bubble":
				draw_arc(pos, s, 0, TAU, 14, Color(1, 1, 1, 0.8 * a), 1.5, true)
				draw_circle(pos + Vector2(-s * 0.3, -s * 0.3), s * 0.2, Color(1, 1, 1, 0.7 * a))
			"drip":
				draw_circle(pos, s, Color(0.55, 0.75, 0.95, 0.8 * a))
			"mud":
				draw_circle(pos, s, Color(0.33, 0.24, 0.15, a))
			"sparkle":
				var c := Color(1.0, 0.95, 0.7, a)
				draw_line(pos - Vector2(s, 0), pos + Vector2(s, 0), c, 2.0, true)
				draw_line(pos - Vector2(0, s), pos + Vector2(0, s), c, 2.0, true)
			"z":
				draw_string(font, pos, "z", HORIZONTAL_ALIGNMENT_LEFT, -1, int(s), Color(0.95, 0.9, 0.8, a))
			"heart":
				var c := Color(0.9, 0.35, 0.4, a)
				draw_circle(pos + Vector2(-s * 0.35, 0), s * 0.45, c)
				draw_circle(pos + Vector2(s * 0.35, 0), s * 0.45, c)
				draw_colored_polygon(PackedVector2Array([pos + Vector2(-s * 0.78, s * 0.15), pos + Vector2(s * 0.78, s * 0.15), pos + Vector2(0, s * 0.95)]), c)
