extends Node2D
## The map (decision 0034): the camp, the places around it, and the
## routes there, with their dangers drawn on them. Tap a place, then a
## route; the picture at the bottom shows the chance of coming back well
## and how long it takes. The footprints button sends the friend who was
## being looked after in the tent. Friends already out walk their route.
##
## Map-only open world (0035). Preset routes for the first playable
## (Gate A). Drawn once and baked; only the walking friends move.

signal go_to(place: String)

const VIEW := Vector2(360, 640)

var place := ""
var route := 0

var camera: Camera2D
var layer: CanvasLayer
var hud: MapHud
var _land: Baked
var _routes: Baked
var _walkers: Node2D
var _t := 0.0


func _ready() -> void:
	var under := ColorRect.new()
	under.color = Color(0.36, 0.3, 0.22)
	under.position = Vector2(-1500, -1500)
	under.size = Vector2(3720, 4280)
	under.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(under)
	_land = Baked.new()
	add_child(_land)
	_land.setup(Rect2(-120, -160, 960, 1600), _paint_land, 1.0)
	_land.refresh(1)
	_routes = Baked.new()
	add_child(_routes)
	_routes.setup(Rect2(0, 120, 720, 980), _paint_routes, 1.0)
	_walkers = Walkers.new()
	add_child(_walkers)
	camera = Camera2D.new()
	camera.position = VIEW
	add_child(camera)
	layer = CanvasLayer.new()
	add_child(layer)
	hud = MapHud.new()
	hud.map = self
	layer.add_child(hud)
	_refresh()


func set_active(on: bool) -> void:
	visible = on
	layer.visible = on
	process_mode = Node.PROCESS_MODE_INHERIT if on else Node.PROCESS_MODE_DISABLED
	if on:
		camera.make_current()
		_refresh()


func _refresh() -> void:
	_routes.refresh([place, route])
	hud.queue_redraw()


func to_world(screen: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * screen


## The chance the friend in front comes back well on the chosen route.
func chance() -> float:
	if place == "" or Save.friend == null:
		return 0.0
	return Quests.chance(Save.friend, place, route)


## Can the friend in front be sent? Someone must stay at camp.
func can_send() -> bool:
	var f := Save.friend
	if place == "" or f == null or f.dead or f.headless:
		return false
	var others := 0
	for g in Save.camp.friends:
		if g != f and not g.dead and not g.headless:
			others += 1
	return others > 0


func send() -> void:
	if not can_send():
		hud.shake = 0.5
		Voice.thud()
		return
	Save.camp.send(Save.camp.current, place, route, Save.now())
	Save.mark_dirty()
	Events.quest_started.emit(place)
	Voice.cheer()
	place = ""
	_refresh()


func _unhandled_input(event: InputEvent) -> void:
	if not (event is InputEventScreenTouch and event.index == 0 and not event.pressed):
		return
	var sp: Vector2 = event.position
	var h := hud.hit(sp)
	if h == "back":
		go_to.emit("entrance")
		return
	if h == "go":
		send()
		return
	var w := to_world(sp)
	# A place's icon.
	for id in Quests.PLACES:
		if w.distance_to(Quests.PLACES[id]["at"]) < 70.0:
			place = id
			route = 0
			Voice.pip()
			_refresh()
			return
	# One of the chosen place's routes.
	if place != "":
		var best := -1
		var best_d := 48.0
		for i in Quests.PLACES[place]["routes"].size():
			var pts := Quests.route_points(place, i)
			for k in pts.size() - 1:
				var q := Geometry2D.get_closest_point_to_segment(w, pts[k], pts[k + 1])
				if q.distance_to(w) < best_d:
					best_d = q.distance_to(w)
					best = i
		if best >= 0:
			route = best
			Voice.pip()
			_refresh()


func _process(delta: float) -> void:
	_t += delta
	_walkers.queue_redraw()


# --- Drawing --------------------------------------------------------------

func _paint_land(ci: CanvasItem) -> void:
	# Old paper.
	ci.draw_rect(Rect2(-120, -160, 960, 1600), Color(0.86, 0.79, 0.63))
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	for i in 40:
		var at := Vector2(rng.randf_range(-100, 820), rng.randf_range(-140, 1420))
		PlushArt.ellipse(ci, at, rng.randf_range(30, 90), rng.randf_range(20, 50), Color(0.78, 0.7, 0.52, 0.25), false)
	# The meadow.
	PlushArt.ellipse(ci, Vector2(190, 800), 120, 70, Color(0.66, 0.74, 0.45), false)
	for i in 14:
		var g := Vector2(rng.randf_range(100, 280), rng.randf_range(760, 840))
		ci.draw_line(g, g + Vector2(-3, -12), Color(0.42, 0.55, 0.3), 2.0)
		ci.draw_line(g, g + Vector2(4, -10), Color(0.42, 0.55, 0.3), 2.0)
	# The woods.
	for i in 16:
		var tr := Vector2(rng.randf_range(470, 670), rng.randf_range(530, 690))
		ci.draw_colored_polygon(PackedVector2Array([tr + Vector2(0, -34), tr + Vector2(-20, 6), tr + Vector2(20, 6)]), Color(0.3, 0.46, 0.3))
		ci.draw_line(tr + Vector2(0, 6), tr + Vector2(0, 14), Color(0.4, 0.3, 0.2), 4.0)
	# The river, across the map.
	var river := PackedVector2Array()
	for i in 13:
		var x := -100.0 + i * 80.0
		river.append(Vector2(x, 470 + sin(i * 0.9) * 40.0 + i * 10.0))
	ci.draw_polyline(river, Color(0.42, 0.62, 0.75), 34.0, true)
	ci.draw_polyline(river, Color(0.6, 0.78, 0.86), 10.0, true)
	# Hills and the ruins (an old toy shop).
	for hill in [Vector2(600, 330), Vector2(680, 410), Vector2(120, 260)]:
		ci.draw_colored_polygon(PackedVector2Array([hill + Vector2(-90, 50), hill + Vector2(0, -60), hill + Vector2(90, 50)]), Color(0.66, 0.58, 0.46))
	ci.draw_rect(Rect2(440, 190, 80, 60), Color(0.55, 0.42, 0.36))
	ci.draw_colored_polygon(PackedVector2Array([Vector2(430, 192), Vector2(480, 150), Vector2(530, 192)]), Color(0.45, 0.3, 0.28))
	ci.draw_rect(Rect2(470, 220, 20, 30), Color(0.3, 0.22, 0.18))
	# The camp: a tent.
	var c := Quests.CAMP
	ci.draw_colored_polygon(PackedVector2Array([c + Vector2(-50, 30), c + Vector2(0, -44), c + Vector2(50, 30)]), Color(0.8, 0.69, 0.52))
	ci.draw_colored_polygon(PackedVector2Array([c + Vector2(-10, 30), c + Vector2(0, -10), c + Vector2(10, 30)]), Color(0.36, 0.28, 0.2))
	ci.draw_polyline(PackedVector2Array([c + Vector2(-50, 30), c + Vector2(0, -44), c + Vector2(50, 30)]), Color(0.42, 0.29, 0.18), 3.0, true)
	# Place markers: a ring to tap.
	for id in Quests.PLACES:
		var at: Vector2 = Quests.PLACES[id]["at"]
		ci.draw_arc(at, 40.0, 0, TAU, 32, Color(0.45, 0.3, 0.2, 0.7), 4.0, true)


func _paint_routes(ci: CanvasItem) -> void:
	for id in Quests.PLACES:
		var routes: Array = Quests.PLACES[id]["routes"]
		for i in routes.size():
			var pts := Quests.route_points(id, i)
			var chosen: bool = id == place and i == route
			var shown: bool = id == place
			var col := Color(0.45, 0.3, 0.2, 0.9 if shown else 0.3)
			if chosen:
				ci.draw_polyline(pts, Color(1.0, 0.9, 0.55, 0.8), 16.0, true)
			_dashes(ci, pts, col, 6.0 if shown else 3.0)
			if shown:
				for d in routes[i]["dangers"]:
					_danger(ci, Quests.along(pts, d[1]), d[0])
	if place != "":
		ci.draw_arc(Quests.PLACES[place]["at"], 46.0, 0, TAU, 32, Color(1.0, 0.85, 0.4), 6.0, true)


func _dashes(ci: CanvasItem, pts: PackedVector2Array, col: Color, width: float) -> void:
	for i in pts.size() - 1:
		var a := pts[i]
		var b := pts[i + 1]
		var n := int(a.distance_to(b) / 18.0)
		for k in range(0, n, 2):
			ci.draw_line(a.lerp(b, float(k) / n), a.lerp(b, float(k + 1) / n), col, width, true)


## A danger, drawn as itself: waves, a mud puddle, thorns, a wolf's
## eyes, a cliff edge.
static func _danger(ci: CanvasItem, at: Vector2, kind: String) -> void:
	ci.draw_circle(at, 26.0, Color(0.95, 0.9, 0.8))
	ci.draw_arc(at, 26.0, 0, TAU, 24, Color(0.6, 0.25, 0.2), 3.0, true)
	match kind:
		"water":
			for k in 2:
				var y := at.y - 4.0 + k * 10.0
				var wave := PackedVector2Array()
				for i in 7:
					wave.append(Vector2(at.x - 15.0 + i * 5.0, y + (3.0 if i % 2 == 0 else -3.0)))
				ci.draw_polyline(wave, Color(0.3, 0.5, 0.7), 3.0, true)
		"mud":
			PlushArt.ellipse(ci, at + Vector2(0, 4), 17.0, 9.0, Color(0.42, 0.3, 0.2), false)
			ci.draw_circle(at + Vector2(-6, -6), 4.0, Color(0.42, 0.3, 0.2))
		"thorns":
			for i in 4:
				var x := at.x - 12.0 + i * 8.0
				ci.draw_colored_polygon(PackedVector2Array([Vector2(x - 4, at.y + 10), Vector2(x, at.y - 12), Vector2(x + 4, at.y + 10)]), Color(0.35, 0.4, 0.2))
		"wolf":
			ci.draw_circle(at, 18.0, Color(0.2, 0.18, 0.2))
			ci.draw_circle(at + Vector2(-7, -3), 3.5, Color(1.0, 0.85, 0.3))
			ci.draw_circle(at + Vector2(7, -3), 3.5, Color(1.0, 0.85, 0.3))
		"cliff":
			ci.draw_colored_polygon(PackedVector2Array([at + Vector2(-16, 14), at + Vector2(-16, -10), at + Vector2(4, -10), at + Vector2(16, 14)]), Color(0.5, 0.42, 0.34))
			ci.draw_line(at + Vector2(4, -10), at + Vector2(16, 14), Color(0.3, 0.22, 0.16), 3.0)


## Friends out on quests, walking their routes there and back.
class Walkers:
	extends Node2D

	func _draw() -> void:
		var now := Save.now()
		for q in Save.camp.quests:
			var pts := Quests.route_points(q.place, q.route)
			var p := q.progress(now)
			var t := p * 2.0 if p < 0.5 else (1.0 - p) * 2.0
			var at := Quests.along(pts, t)
			LooseHead.draw_head(self, q.friend, Transform2D(0.0, Vector2(0.28, 0.28), 0.0, at + Vector2(0, -20)))
