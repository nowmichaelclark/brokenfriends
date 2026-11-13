class_name CampTraces
extends Node2D
## What happened at camp, shown on the tent floor (decision 0029): muddy
## footprints a muddy friend tracked in, and bugs drawn by rot. The
## player finds out by looking, never from a report. Footprints scrub
## off with the sponge; a tapped bug scurries away.
##
## Cheap: the footprints are one baked picture, and the bugs are sprites
## sharing one baked bug, drawn together.

## The floor area the footprints are baked in, tent units.
const FLOOR := Rect2(0, 860, 720, 240)
const BUG_AREA := Rect2(40, 900, 640, 170)

var camp: CampData
var _prints: Baked
var _stamp: Baked
var _bugs: Array[Sprite2D] = []
var _bug_state: Array = []
var _leaving: Array = []


func setup(c: CampData) -> void:
	camp = c
	if _prints == null:
		_prints = Baked.new()
		add_child(_prints)
		_prints.setup(FLOOR, _paint_prints, 1.0)
		_stamp = Baked.new()
		add_child(_stamp)
		_stamp.setup(Rect2(-14, -10, 28, 20), _paint_bug, 2.0)
		_stamp.refresh(1)
		_stamp.hide_picture()
	refresh()


## Match what's shown to the camp (after time passes or a scrub).
func refresh() -> void:
	var key: Array = []
	for t in camp.tracks:
		key.append(int(float(t["x"])))
		key.append(int(float(t["r"]) * 10.0))
	_prints.refresh(key)
	while _bugs.size() < camp.bugs:
		_add_bug()
	while _bugs.size() > camp.bugs:
		var b: Sprite2D = _bugs.pop_back()
		_bug_state.pop_back()
		b.queue_free()


func _add_bug() -> void:
	var b := Sprite2D.new()
	b.texture = _stamp.picture()
	b.material = _stamp.picture_material()
	b.scale = Vector2(0.5, 0.5)
	b.position = Vector2(randf_range(BUG_AREA.position.x, BUG_AREA.end.x), randf_range(BUG_AREA.position.y, BUG_AREA.end.y))
	add_child(b)
	_bugs.append(b)
	_bug_state.append({"dir": randf() * TAU, "wait": randf() * 2.0})


func _process(delta: float) -> void:
	for i in _bugs.size():
		var b := _bugs[i]
		var st: Dictionary = _bug_state[i]
		st["wait"] -= delta
		if st["wait"] > 0.0:
			continue
		if randf() < delta * 0.6:
			st["dir"] = float(st["dir"]) + randf_range(-1.2, 1.2)
		if randf() < delta * 0.3:
			st["wait"] = randf_range(0.4, 1.8)
		var dir := Vector2.from_angle(st["dir"])
		var next := b.position + dir * 38.0 * delta
		if not BUG_AREA.has_point(next):
			st["dir"] = (BUG_AREA.get_center() - b.position).angle()
		else:
			b.position = next
		b.rotation = float(st["dir"])
	for i in range(_leaving.size() - 1, -1, -1):
		var b: Sprite2D = _leaving[i]
		b.position += Vector2.from_angle(b.rotation) * 420.0 * delta
		if not Rect2(-100, 700, 920, 600).has_point(b.position):
			b.queue_free()
			_leaving.remove_at(i)


## Tap a bug and it scurries off. Returns true if one was hit.
func shoo(world: Vector2) -> bool:
	for i in _bugs.size():
		var b := _bugs[i]
		if b.position.distance_to(world) < 34.0:
			_bugs.remove_at(i)
			_bug_state.remove_at(i)
			b.rotation = (b.position - world).angle() if b.position != world else 0.0
			_leaving.append(b)
			camp.bugs = maxi(0, camp.bugs - 1)
			return true
	return false


## Scrub footprints under the sponge. Returns true if any came off.
func scrub(world: Vector2, amount: float) -> bool:
	var hit := false
	for i in range(camp.tracks.size() - 1, -1, -1):
		var t: Dictionary = camp.tracks[i]
		if Vector2(float(t["x"]), float(t["y"])).distance_to(world) < 60.0:
			t["r"] = float(t["r"]) - amount
			hit = true
			if float(t["r"]) <= 0.0:
				camp.tracks.remove_at(i)
	if hit:
		refresh()
	return hit


func _paint_prints(ci: CanvasItem) -> void:
	for t in camp.tracks:
		var at := Vector2(float(t["x"]), float(t["y"]))
		var dir := Vector2.from_angle(float(t["a"]))
		var c := Color(0.27, 0.19, 0.12, 0.55 * clampf(float(t["r"]), 0.0, 1.0))
		for k in 3:
			var side := dir.orthogonal() * (10.0 if k % 2 == 0 else -10.0)
			var p := at + dir * (k - 1) * 34.0 + side
			PlushArt.ellipse(ci, p, 11.0, 8.0, c, false)
			for toe in 3:
				ci.draw_circle(p + dir * 12.0 + dir.orthogonal() * (toe - 1) * 6.0, 3.0, c)


func _paint_bug(ci: CanvasItem) -> void:
	var dark := Color(0.12, 0.1, 0.08)
	for k in 3:
		var x := -6.0 + k * 6.0
		ci.draw_line(Vector2(x, 0), Vector2(x - 3, -9), dark, 1.5, true)
		ci.draw_line(Vector2(x, 0), Vector2(x - 3, 9), dark, 1.5, true)
	PlushArt.ellipse(ci, Vector2(-2, 0), 9.0, 6.0, Color(0.22, 0.17, 0.1), false)
	ci.draw_circle(Vector2(8, 0), 3.6, dark)
	ci.draw_line(Vector2(-6, 0), Vector2(5, 0), Color(0.35, 0.28, 0.18), 1.0)
