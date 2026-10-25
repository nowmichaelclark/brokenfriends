class_name PlayRoom
extends Node2D
## The play room (Mikey's playtest notes, Q-036): a room three screens
## wide where the camp's friends wander about together, "the more the
## merrier". Pinch, or the mouse wheel, zooms the camera in and out; a
## finger dragged across the floor looks around; a double-tap comes in
## on a spot or goes back out. Throw the ball: whoever it meets reacts,
## and catching and kicking are learned slowly by doing (0017, 0033).
## The dev menu's stress test (`crowd`, or `#crowd=N`) adds made-up
## friends, never saved, to see how many a phone holds (0045).

signal go_to(place: String)

const ROOM_W := 2160.0
const FLOOR_Y := Wanderer.FRONT_Y
const TOP_Y := 120.0
const ZOOM_MIN := 0.33
const ZOOM_MAX := 1.6

## Made-up friends added for the stress test.
static var crowd := 0

var wanderers: Array[Wanderer] = []
var fx: FxLayer
var camera: Camera2D
var ball: ToyBall
var layer: CanvasLayer
var hud: PlayHud
var hint: Label

var _friends_layer: Node2D
var _mode := ""
var _touched: Wanderer
var _press_time := 0.0
var _moved := 0.0
var _last_world := Vector2.ZERO
var _ball_vel := Vector2.ZERO
var _now := 0.0
var _rng := RandomNumberGenerator.new()
## Screen points of the fingers down (index -> position), for pinching.
var _fingers: Dictionary = {}
var _pinch_from := 0.0
var _zoom_from := 1.0
var _last_tap := -10.0
var _last_tap_pos := Vector2.ZERO
var _cam_tween: Tween


func _ready() -> void:
	_rng.randomize()
	crowd = maxi(crowd, int(Flags.value("crowd", "0")))
	add_child(PlayBackground.new())
	_build_walls()
	_friends_layer = Node2D.new()
	add_child(_friends_layer)
	ball = ToyBall.new()
	ball.position = Vector2(ROOM_W * 0.5 + 160.0, 700)
	ball.z_index = 200
	add_child(ball)
	fx = FxLayer.new()
	fx.floor_y = FLOOR_Y
	fx.z_index = 201
	add_child(fx)
	camera = Camera2D.new()
	add_child(camera)
	layer = CanvasLayer.new()
	add_child(layer)
	hud = PlayHud.new()
	layer.add_child(hud)
	hint = Label.new()
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	hint.add_theme_font_size_override("font_size", 30)
	hint.add_theme_color_override("font_color", Color(1.0, 0.95, 0.85))
	hint.add_theme_color_override("font_outline_color", Color(0.14, 0.09, 0.07))
	hint.add_theme_constant_override("outline_size", 12)
	hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(hint)
	hint.visible = Flags.has("hints")


func set_active(on: bool) -> void:
	visible = on
	layer.visible = on
	process_mode = Node.PROCESS_MODE_INHERIT if on else Node.PROCESS_MODE_DISABLED
	if on:
		_populate()
		# Start on the friend being looked after, close enough to see them.
		var w := wanderer_of(Save.friend)
		camera.position = Vector2(w.view.position.x if w else ROOM_W * 0.5, 640)
		camera.zoom = Vector2.ONE
		_clamp_camera()
		camera.make_current()


## Everyone at camp who can play (alive, with a head), and the crowd.
func _populate() -> void:
	for w in wanderers:
		w.view.queue_free()
	wanderers.clear()
	var friends: Array = []
	for f in Save.camp.friends:
		if not f.dead and not f.headless:
			friends.append(f)
	for i in crowd:
		friends.append(FriendData.test_friend(9000 + i))
	var n := friends.size()
	for i in n:
		var x := ROOM_W * (i + 0.5) / n + _rng.randf_range(-60.0, 60.0)
		var f: FriendData = friends[i]
		# The friend being looked after starts in the middle of the view.
		if f == Save.friend:
			x = ROOM_W * 0.5
		var w := Wanderer.new(f, x, _rng.randf(), ROOM_W, _rng)
		_friends_layer.add_child(w.view)
		w.view.fx = fx
		w.view.setup(f)
		w.place()
		wanderers.append(w)


func wanderer_of(f: FriendData) -> Wanderer:
	for w in wanderers:
		if w.data == f:
			return w
	return null


func _build_walls() -> void:
	var body := StaticBody2D.new()
	add_child(body)
	for r in [Rect2(-600, FLOOR_Y, ROOM_W + 1200, 400), Rect2(-400, -400, 430, 1800),
			Rect2(ROOM_W - 30, -400, 430, 1800), Rect2(-600, -400, ROOM_W + 1200, 400 + TOP_Y)]:
		var cs := CollisionShape2D.new()
		var shape := RectangleShape2D.new()
		shape.size = r.size
		cs.shape = shape
		cs.position = r.position + r.size * 0.5
		body.add_child(cs)


func to_world(screen: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * screen


# --- The camera -----------------------------------------------------------

## Keep the view inside the room.
func _clamp_camera() -> void:
	var view := get_viewport_rect().size / camera.zoom
	var half := view * 0.5
	var lo := Vector2(half.x, TOP_Y - 80.0 + half.y)
	var hi := Vector2(ROOM_W - half.x, FLOOR_Y + 260.0 - half.y)
	camera.position.x = ROOM_W * 0.5 if lo.x > hi.x else clampf(camera.position.x, lo.x, hi.x)
	camera.position.y = (TOP_Y + FLOOR_Y) * 0.5 if lo.y > hi.y else clampf(camera.position.y, lo.y, hi.y)


## Zoom to `z`, keeping the world point under `screen` where it is.
func zoom_at(z: float, screen: Vector2) -> void:
	var before := to_world(screen)
	camera.zoom = Vector2.ONE * clampf(z, ZOOM_MIN, ZOOM_MAX)
	camera.force_update_scroll()
	var after := to_world(screen)
	camera.position += before - after
	_clamp_camera()


func _toggle_zoom(world: Vector2) -> void:
	var out := camera.zoom.x > 0.6
	if _cam_tween:
		_cam_tween.kill()
	_cam_tween = create_tween().set_parallel().set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	var z := ZOOM_MIN if out else 1.2
	_cam_tween.tween_property(camera, "zoom", Vector2(z, z), 0.45)
	_cam_tween.tween_property(camera, "position", Vector2(ROOM_W * 0.5, 640) if out else world, 0.45)
	_cam_tween.chain().tween_callback(_clamp_camera)


# --- Input --------------------------------------------------------------

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch:
		if event.pressed:
			_fingers[event.index] = event.position
		else:
			_fingers.erase(event.index)
		if _fingers.size() == 2:
			# A second finger: pinch to zoom, whatever the first was doing.
			_cancel()
			_mode = "pinch"
			var p: Array = _fingers.values()
			_pinch_from = (p[0] as Vector2).distance_to(p[1])
			_zoom_from = camera.zoom.x
			return
		if event.index != 0:
			return
		if event.pressed:
			_press(event.position)
		else:
			_release(event.position)
	elif event is InputEventScreenDrag:
		_fingers[event.index] = event.position
		if _mode == "pinch" and _fingers.size() == 2:
			var p: Array = _fingers.values()
			var mid: Vector2 = ((p[0] as Vector2) + p[1]) * 0.5
			zoom_at(_zoom_from * (p[0] as Vector2).distance_to(p[1]) / maxf(_pinch_from, 1.0), mid)
		elif event.index == 0:
			_drag(event.position, event.relative, event.velocity)
	elif event is InputEventMouseButton and event.pressed:
		# The mouse wheel zooms, on a laptop.
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			zoom_at(camera.zoom.x * 1.1, event.position)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			zoom_at(camera.zoom.x / 1.1, event.position)


func _cancel() -> void:
	if _mode == "ball":
		ball.freeze = false
	if _mode == "touch" and _touched:
		_touched.view.end_pet()
	_touched = null


func _press(sp: Vector2) -> void:
	if hud.back_rect().grow(10).has_point(sp):
		go_to.emit("entrance")
		return
	var w := to_world(sp)
	_last_world = w
	_press_time = _now
	_moved = 0.0
	_ball_vel = Vector2.ZERO
	_touched = null
	if not _held_ball() and w.distance_to(ball.global_position) < ToyBall.RADIUS + 34.0 / camera.zoom.x:
		_mode = "ball"
		ball.freeze = true
		ball.global_position = w
		return
	# The nearest friend in front gets the touch.
	var best := -1
	for i in wanderers.size():
		var wd := wanderers[i]
		if wd.view.is_on_friend(wd.view.to_local(w)) and (best < 0 or wd.depth > wanderers[best].depth):
			best = i
	if best >= 0:
		_mode = "touch"
		_touched = wanderers[best]
		_touched.poked()
		return
	if _now - _last_tap < 0.32 and sp.distance_to(_last_tap_pos) < 70.0:
		_last_tap = -10.0
		_mode = ""
		_toggle_zoom(w)
		return
	_mode = "pan"


func _drag(sp: Vector2, rel: Vector2, vel: Vector2) -> void:
	var w := to_world(sp)
	_moved += rel.length()
	match _mode:
		"ball":
			ball.global_position = Vector2(clampf(w.x, 60, ROOM_W - 60), clampf(w.y, TOP_Y + 40, FLOOR_Y - ToyBall.RADIUS))
			_ball_vel = vel / camera.zoom.x
		"touch":
			if _moved > 30.0:
				_touched.view.pet(_touched.view.to_local(w), (w - _last_world).length() / _touched.k())
		"pan":
			camera.position -= rel / camera.zoom.x
			_clamp_camera()
	_last_world = to_world(sp)


func _release(sp: Vector2) -> void:
	var w := to_world(sp)
	match _mode:
		"ball":
			ball.freeze = false
			ball.linear_velocity = _ball_vel.limit_length(1700.0)
			ball.angular_velocity = _ball_vel.x * 0.01
		"touch":
			if _moved < 30.0 and _now - _press_time < 0.45:
				_touched.view.poke(_touched.view.to_local(w))
			_touched.view.end_pet()
		"pan":
			if _moved < 20.0:
				_last_tap = _now
				_last_tap_pos = sp
	_mode = ""
	_touched = null


func _held_ball() -> bool:
	for w in wanderers:
		if w.caught:
			return true
	return false


# --- Play ---------------------------------------------------------------

func _physics_process(delta: float) -> void:
	for w in wanderers:
		if w.caught:
			ball.global_position = w.view.to_global(w.view.head_center() + Vector2(0, -FriendView.HEAD_R - 40.0))
			w.catch_timer -= delta
			if w.catch_timer <= 0.0:
				w.caught = false
				ball.freeze = false
				ball.linear_velocity = Vector2(_rng.randf_range(-280, 280), -880)
				w.view.arms_up = false
				w.cooldown = 0.6
			return
	if _mode == "ball" or ball.linear_velocity.length() < 120.0:
		return
	# The ball can meet any part of any friend (pillar 6), nearest first.
	var order := wanderers.duplicate()
	order.sort_custom(func(a, b): return a.depth > b.depth)
	for w in order:
		if w.cooldown <= 0.0 and not w.data.asleep and _ball_meets(w):
			return


## Does the ball meet this friend? If so, they react, and it's true.
func _ball_meets(w: Wanderer) -> bool:
	var f := w.view
	var data := w.data
	var local := f.to_local(ball.global_position)
	var r := ToyBall.RADIUS / w.k()
	if local.distance_to(f.head_center()) < FriendArt.head_ry(data) + r and f.facing == "front":
		_at_head(w)
		return true
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		var part: PartData = data.part(slot)
		if part == null or not part.attached:
			continue
		var q := Geometry2D.get_closest_point_to_segment(local, f.socket(slot), f.tip(slot))
		if local.distance_to(q) < f.limb_width(slot) * 0.5 + r:
			if slot.begins_with("leg") and local.distance_to(f.tip(slot)) < 60.0 + r:
				_at_foot(w, slot)
			else:
				_off_part(w, slot, local - q)
			return true
	var rx := 112.0 * f.body_scale() * f.body_w() + r
	var ry := 132.0 * f.body_h() + r
	if pow(local.x / rx, 2) + pow(local.y / ry, 2) <= 1.0:
		_off_part(w, "", local)
		return true
	return false


## At the head: they try to catch it. Catching is learned by throwing
## again and again; even a good catcher fumbles now and then (0033).
func _at_head(w: Wanderer) -> void:
	var data := w.data
	var f := w.view
	var head := f.to_global(f.head_center())
	var skill := data.catch_skill
	data.catch_skill = FriendData.learn(skill)
	Save.mark_dirty()
	w.poked()
	match FriendData.try_skill(skill, _rng):
		"catch":
			w.caught = true
			w.catch_timer = 1.3
			ball.freeze = true
			f.arms_up = true
			f.set_expression("squint", "open", 1.3)
			Events.friend_reacted.emit("catch")
		"fumble":
			# Got it... and it pops out of their arms.
			f.arms_up = true
			w.arms_timer = 0.5
			ball.linear_velocity = Vector2(_rng.randf_range(-160, 160), -380)
			f.set_expression("open", "o", 0.8)
			Events.friend_reacted.emit("giggle")
			w.cooldown = 0.6
		_:
			ball.linear_velocity = (ball.global_position - head).normalized() * 480.0 + Vector2(0, -160)
			f.bonk()
			# A late grab at the ball: they're still learning.
			f.arms_up = true
			w.arms_timer = 0.35
			w.cooldown = 0.7


## At a foot: sometimes they kick it back, better with practice.
func _at_foot(w: Wanderer, slot: String) -> void:
	var data := w.data
	var skill := data.kick_skill
	data.kick_skill = FriendData.learn(skill)
	Save.mark_dirty()
	w.poked()
	var side := -1.0 if slot == "leg_l" else 1.0
	if FriendData.try_skill(skill, _rng) != "miss":
		# A proper kick, up and away across the room.
		w.view.nudge_limb(slot, Vector2(-side * 420.0, -380.0))
		ball.linear_velocity = Vector2(-side * _rng.randf_range(380, 620), _rng.randf_range(-760, -520))
		w.view.set_expression("squint", "smile", 0.8)
		Events.friend_reacted.emit("cheer")
	else:
		w.view.nudge_limb(slot, Vector2(side * 200.0, -120.0))
		ball.linear_velocity = Vector2(side * 260.0, -240.0)
		w.view.set_expression("open", "o", 0.5)
		Events.friend_reacted.emit("boop")
	w.cooldown = 0.6


## Off an arm or the body: a bounce and a little reaction.
func _off_part(w: Wanderer, slot: String, away: Vector2) -> void:
	var dir := away.normalized() if away.length() > 0.1 else Vector2.UP
	ball.linear_velocity = (dir * 420.0 + Vector2(0, -260)).rotated(w.view.rotation)
	if slot == "":
		w.view.bounce_off()
	else:
		w.view.nudge_limb(slot, -dir * 300.0)
		Events.friend_reacted.emit("boop")
	w.poked()
	w.cooldown = 0.5


func _process(delta: float) -> void:
	_now += delta
	var rest := ball.global_position.x if ball.global_position.y > FLOOR_Y - 60.0 and ball.linear_velocity.length() < 60.0 and _mode != "ball" else -INF
	for w in wanderers:
		w.process(delta, rest)
	var vs := get_viewport_rect().size
	hint.position = Vector2(40, 118)
	hint.size = Vector2(vs.x - 80, 120)
	if hint.visible:
		hint.text = "Throw the ball. Pinch or scroll to zoom; drag the floor to look around. %d friends." % wanderers.size()


class PlayHud:
	extends Node2D

	func back_rect() -> Rect2:
		return Rect2(24.0, 24.0, 80.0, 80.0)

	func _draw() -> void:
		Icons.button(self, back_rect().get_center(), Color(0.42, 0.55, 0.38), "tent")


## The room, drawn once into pictures (a wide room is several).
class PlayBackground:
	extends Node2D

	func _ready() -> void:
		for i in 3:
			var b := Baked.new()
			add_child(b)
			b.setup(Rect2(-200 + i * 920, -300, 920, 1900), _paint, 0.75)
			b.refresh(0)

	func _paint(ci: CanvasItem) -> void:
		var floor_y := PlayRoom.FLOOR_Y
		var back_y := Wanderer.BACK_Y - 40.0
		var w := PlayRoom.ROOM_W
		ci.draw_rect(Rect2(-700, -700, w + 1400, 2800), Color(0.24, 0.17, 0.13))
		# The back wall, then the floor running toward the front.
		ci.draw_rect(Rect2(0, PlayRoom.TOP_Y, w, back_y - PlayRoom.TOP_Y), Color(0.78, 0.66, 0.5))
		for x in range(80, int(w), 160):
			PlushArt.seam(ci, PackedVector2Array([Vector2(x, PlayRoom.TOP_Y + 10), Vector2(x, back_y - 6)]), Color(0.62, 0.5, 0.36), 12.0, 2.0)
		ci.draw_rect(Rect2(0, back_y, w, floor_y - back_y + 300), Color(0.45, 0.34, 0.25))
		for y in [back_y + 40.0, back_y + 100.0, back_y + 170.0, floor_y + 60.0]:
			ci.draw_line(Vector2(0, y), Vector2(w, y), Color(0.38, 0.28, 0.2), 2.0)
		# Bunting all the way across.
		var flags := [Color(0.82, 0.36, 0.3), Color(0.92, 0.75, 0.32), Color(0.38, 0.58, 0.72), Color(0.5, 0.68, 0.42)]
		for span in 3:
			var x0 := 40.0 + span * 700.0
			draw_bunting(ci, x0, x0 + 640.0, flags)
		# Rugs, a toy box, a window, a little slide.
		for c in [Vector2(400, floor_y - 40), Vector2(1100, floor_y - 60), Vector2(1800, floor_y - 30)]:
			PlushArt.ellipse(ci, c, 260.0, 60.0, Color(0.38, 0.52, 0.6), false)
			PlushArt.ellipse(ci, c, 200.0, 44.0, Color(0.85, 0.78, 0.6), false)
		ci.draw_rect(Rect2(60, back_y - 110, 130, 110), Color(0.62, 0.38, 0.26))
		ci.draw_rect(Rect2(52, back_y - 126, 146, 22), Color(0.5, 0.3, 0.2))
		ci.draw_circle(Vector2(125, back_y - 60), 14.0, Color(0.92, 0.75, 0.32))
		ci.draw_rect(Rect2(980, 300, 240, 200), Color(0.62, 0.78, 0.86))
		ci.draw_rect(Rect2(980, 300, 240, 200), Color(0.42, 0.3, 0.2), false, 10.0)
		ci.draw_line(Vector2(1100, 300), Vector2(1100, 500), Color(0.42, 0.3, 0.2), 6.0)
		ci.draw_colored_polygon(PackedVector2Array([Vector2(1900, back_y), Vector2(1960, back_y - 200), Vector2(2000, back_y - 200), Vector2(2080, back_y)]), Color(0.72, 0.4, 0.36))

	static func draw_bunting(ci: CanvasItem, x0: float, x1: float, flags: Array) -> void:
		ci.draw_polyline(PackedVector2Array([Vector2(x0, 200), Vector2((x0 + x1) * 0.5, 250), Vector2(x1, 200)]), Color(0.35, 0.25, 0.18), 3.0, true)
		for i in 9:
			var t := (i + 0.5) / 9.0
			var top := Vector2(lerpf(x0, x1, t), 200 + sin(t * PI) * 50.0)
			ci.draw_colored_polygon(PackedVector2Array([top + Vector2(-20, 0), top + Vector2(20, 0), top + Vector2(0, 40)]), flags[i % 4])
