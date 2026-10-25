class_name Entrance
extends Node2D
## The entrance to the tent: the starting screen (Mikey's playtest
## notes, Q-037). The camp's friends stand outside; a newcomer, and
## anything that came back from a quest, sparkles. Tap a friend to
## choose them, then a picture to go: the grooming screen, the surgery
## table, the play room, the map. What the chosen friend needs most
## glows (no words, 0032). Start over is here; so is the dev menu
## (0045).

signal go_to(place: String)

const FLOOR_Y := 900.0
const FRIEND_SCALE := 0.42
## Where friends stand, in the order they fill up (a camp holds five).
## (y: nearer the front, positive; further back, negative.)
const SPOTS := [Vector2(360, 40), Vector2(175, 0), Vector2(545, 0), Vector2(60, -50), Vector2(660, -50)]
## The buttons along the bottom: place, icon, colour.
const GOES := [
	["grooming", "sponge", Color(0.5, 0.62, 0.7)],
	["table", "needle", Color(0.62, 0.42, 0.42)],
	["play_room", "room", Color(0.42, 0.55, 0.38)],
	["map", "map", Color(0.5, 0.42, 0.3)],
]

var camera: Camera2D
var layer: CanvasLayer
var hud: EntranceHud
## Index in Save.camp.friends -> BenchFriend.
var standing: Dictionary = {}
## Friends not yet seen when they were shown here: they sparkle until
## chosen.
var fresh: Array[FriendData] = []
var fx: FxLayer
var dev: DevMenu

var _layer_friends: Node2D
var _spares_seen := -1
var _t := 0.0
var _sparkle := 0.0
var _reset_timer := 0.0


func _ready() -> void:
	var bg := Baked.new()
	add_child(bg)
	bg.setup(Rect2(-400, -300, 1520, 1800), _paint_bg, 1.0)
	bg.refresh(0)
	_layer_friends = Node2D.new()
	add_child(_layer_friends)
	fx = FxLayer.new()
	fx.floor_y = FLOOR_Y
	add_child(fx)
	camera = Camera2D.new()
	camera.position = Vector2(360, 640)
	add_child(camera)
	layer = CanvasLayer.new()
	add_child(layer)
	hud = EntranceHud.new()
	hud.entrance = self
	layer.add_child(hud)
	dev = DevMenu.new()
	dev.entrance = self
	dev.visible = false
	layer.add_child(dev)


func set_active(on: bool) -> void:
	visible = on
	layer.visible = on
	process_mode = Node.PROCESS_MODE_INHERIT if on else Node.PROCESS_MODE_DISABLED
	if on:
		camera.make_current()
		build()
	else:
		dev.visible = false


## Stand the camp's friends outside. Everyone shown has now been seen
## (FriendData.met); those seen for the first time sparkle.
func build() -> void:
	for v in standing.values():
		v.queue_free()
	standing.clear()
	var camp := Save.camp
	for i in mini(camp.friends.size(), SPOTS.size()):
		var f: FriendData = camp.friends[i]
		if not f.met:
			f.met = true
			if not f in fresh:
				fresh.append(f)
		var v := BenchFriend.new()
		var spot: Vector2 = SPOTS[i]
		var k := FRIEND_SCALE * (1.0 + spot.y / 600.0)
		v.position = Vector2(spot.x, FLOOR_Y - 20.0 + spot.y - 232.0 * k)
		v.z_index = int(spot.y) + 100
		_layer_friends.add_child(v)
		v.setup(f, k)
		standing[i] = v
	# Parts and heads that came in since the last look sparkle too.
	var spares := camp.spares.size() + camp.heads.size()
	hud.crate_new = _spares_seen >= 0 and spares > _spares_seen
	_spares_seen = spares
	hud.queue_redraw()


func to_world(screen: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * screen


func _unhandled_input(event: InputEvent) -> void:
	if dev.visible:
		return
	if event is InputEventScreenTouch and event.index == 0 and event.pressed:
		press(event.position)


func press(sp: Vector2) -> void:
	var h := hud.hit(sp)
	if h == "reset":
		if hud.reset_armed:
			Save.new_game()
			get_tree().reload_current_scene()
		else:
			hud.reset_armed = true
			_reset_timer = 3.0
		return
	if h == "dev":
		dev.open()
		return
	if h != "":
		var f := Save.friend
		var away := f == null or f.dead or f.headless
		if h == "play_room" and away:
			hud.shake = h
			Voice.thud()
			return
		Voice.pop()
		go_to.emit(h)
		return
	var w := to_world(sp)
	for i in standing:
		var v: BenchFriend = standing[i]
		if v.hits(w):
			choose(i)
			return


## Choose who to look after.
func choose(i: int) -> void:
	var camp := Save.camp
	if i < 0 or i >= camp.friends.size():
		return
	camp.current = i
	fresh.erase(camp.friends[i])
	var v: BenchFriend = standing.get(i)
	if v:
		v.scale = Vector2(0.85, 0.85)
		create_tween().tween_property(v, "scale", Vector2.ONE, 0.25).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	Voice.pop()
	Save.mark_dirty()
	Events.friend_chosen.emit(i)


func _process(delta: float) -> void:
	_t += delta
	if hud.reset_armed:
		_reset_timer -= delta
		if _reset_timer <= 0.0:
			hud.reset_armed = false
	# A friend laid to rest, a newcomer, a return: stand them again.
	if standing.size() != mini(Save.camp.friends.size(), SPOTS.size()):
		build()
	hud.glow = TentCues.needs(Save.friend)
	var v: BenchFriend = standing.get(Save.camp.current)
	hud.chosen_at = _screen(v.global_position) if v else Vector2.INF
	_sparkle -= delta
	if _sparkle <= 0.0:
		_sparkle = 0.6
		for f in fresh:
			var i := Save.camp.friends.find(f)
			if standing.has(i):
				fx.spawn("sparkle", fx.to_local(standing[i].global_position + Vector2(0, -120)), 3, 60.0)


func _screen(world: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform() * world


func _paint_bg(ci: CanvasItem) -> void:
	# Sky and the meadow around the camp.
	ci.draw_rect(Rect2(-700, -700, 2120, 2800), Color(0.62, 0.74, 0.8))
	for i in 6:
		ci.draw_rect(Rect2(-700, 200 + i * 60, 2120, 60), Color(0.62, 0.74, 0.8).lerp(Color(0.9, 0.82, 0.66), i / 6.0))
	ci.draw_rect(Rect2(-700, 560, 2120, 2000), Color(0.5, 0.6, 0.38))
	# Trees far back.
	for x in [-60.0, 90.0, 640.0, 780.0]:
		PlushArt.ellipse(ci, Vector2(x, 470), 90.0, 120.0, Color(0.32, 0.45, 0.3), false)
	# The tent, its flap tied open.
	var tent := PackedVector2Array([Vector2(60, 860), Vector2(360, 240), Vector2(660, 860)])
	ci.draw_colored_polygon(tent, Color(0.86, 0.75, 0.56))
	ci.draw_polyline(tent, Color(0.55, 0.43, 0.3), 5.0, true)
	ci.draw_colored_polygon(PackedVector2Array([Vector2(250, 860), Vector2(360, 470), Vector2(470, 860)]), Color(0.3, 0.22, 0.16))
	ci.draw_colored_polygon(PackedVector2Array([Vector2(250, 860), Vector2(360, 470), Vector2(300, 860)]), Color(0.74, 0.62, 0.45))
	ci.draw_colored_polygon(PackedVector2Array([Vector2(470, 860), Vector2(360, 470), Vector2(420, 860)]), Color(0.74, 0.62, 0.45))
	# A lantern glow inside, and the ground in front.
	ci.draw_circle(Vector2(360, 700), 60.0, Color(1.0, 0.82, 0.45, 0.25))
	ci.draw_rect(Rect2(-700, FLOOR_Y - 40, 2120, 1400), Color(0.46, 0.55, 0.34))
	PlushArt.ellipse(ci, Vector2(360, FLOOR_Y + 20), 340.0, 70.0, Color(0.56, 0.46, 0.32), false)
	# The crate where what came back from quests waits.
	ci.draw_rect(Rect2(560, 760, 120, 100), Color(0.5, 0.36, 0.22))
	ci.draw_rect(Rect2(560, 760, 120, 100), Color(0.3, 0.2, 0.12), false, 3.0)
	ci.draw_line(Vector2(560, 810), Vector2(680, 810), Color(0.3, 0.2, 0.12), 3.0)


## The entrance's buttons and marks, in screen space.
class EntranceHud:
	extends Node2D

	var entrance: Entrance
	## A place's button to glow ("table", "grooming"), or "".
	var glow := ""
	var shake := ""
	var reset_armed := false
	var crate_new := false
	var chosen_at := Vector2.INF
	var _t := 0.0

	func go_center(i: int) -> Vector2:
		var s := get_viewport_rect().size
		return Vector2(s.x * 0.5 + (i - 1.5) * 150.0, s.y - 110.0)

	func reset_rect() -> Rect2:
		return Rect2(28.0, 28.0, 64.0, 64.0)

	func dev_rect() -> Rect2:
		return Rect2(get_viewport_rect().size.x - 92.0, 28.0, 64.0, 64.0)

	## "reset", "dev", a place's name, or "".
	func hit(p: Vector2) -> String:
		if reset_rect().grow(10).has_point(p):
			return "reset"
		if dev_rect().grow(10).has_point(p):
			return "dev"
		for i in Entrance.GOES.size():
			if p.distance_to(go_center(i)) < 64.0:
				return Entrance.GOES[i][0]
		return ""

	func _process(delta: float) -> void:
		_t += delta
		queue_redraw()

	func _draw() -> void:
		var pulse := 0.5 + 0.5 * sin(_t * 4.0)
		if chosen_at != Vector2.INF:
			draw_arc(chosen_at + Vector2(0, 40), 70.0, 0, TAU, 32, Color(1.0, 0.92, 0.55, 0.5 + 0.3 * pulse), 4.0, true)
		for i in Entrance.GOES.size():
			var g: Array = Entrance.GOES[i]
			var c := go_center(i)
			if g[0] == shake:
				c.x += sin(_t * 40.0) * 6.0
			if g[0] == glow:
				draw_circle(c, 62.0 + 6.0 * pulse, Color(1.0, 0.9, 0.55, 0.12 + 0.14 * pulse))
				draw_arc(c, 62.0 + 6.0 * pulse, 0, TAU, 32, Color(1.0, 0.92, 0.55, 0.5 + 0.4 * pulse), 4.0, true)
			Icons.button(self, c, g[2], g[1], 52.0)
		Icons.button(self, reset_rect().get_center(), Color(0.55, 0.3, 0.27) if reset_armed else Color(0.3, 0.24, 0.2, 0.85),
			"again_armed" if reset_armed else "again", 32.0)
		Icons.button(self, dev_rect().get_center(), Color(0.3, 0.3, 0.34, 0.85), "gear", 32.0)
		# The crate: spare parts and heads waiting, sparkling when new.
		var camp := Save.camp
		var crate := entrance._screen(Vector2(620, 760))
		var n := mini(camp.spares.size(), 4)
		for k in n:
			var part: PartData = camp.spares[k]
			draw_set_transform(crate + Vector2(-30 + k * 20, -10), -0.3 + k * 0.2, Vector2(0.45, 0.45))
			PlushArt.limb(self, Vector2(0, -60), Vector2(0, 40), 50.0, part, Color(0.94, 0.86, 0.75), 0.0)
		draw_set_transform(Vector2.ZERO, 0, Vector2.ONE)
		if not camp.heads.is_empty():
			LooseHead.draw_head(self, camp.heads[0], Transform2D(0.0, Vector2(0.3, 0.3), 0.0, crate + Vector2(30, -30)))
		if crate_new and (n > 0 or not camp.heads.is_empty()):
			draw_arc(crate + Vector2(0, 20), 80.0, 0, TAU, 32, Color(1.0, 0.92, 0.55, 0.4 + 0.4 * pulse), 4.0, true)
