extends Node2D
## The play corner inside the tent, like a child's play corner in a
## living room (Mikey's recording). The friend walks, sits and naps.
## Throw the ball: at first it misses and laughs when the ball bonks
## its head; the more you play, the better it catches (decision 0017).

signal go_to(place: String)

const FLOOR_Y := 1000.0
const FRIEND_SCALE := 0.74
const VIEW_CENTER := Vector2(360, 640)

var friend: FriendView
var fx: FxLayer
var camera: Camera2D
var ball: ToyBall
var layer: CanvasLayer
var hud: PlayHud
var hint: Label

var _mode := ""
var _press_time := 0.0
var _moved := 0.0
var _last_world := Vector2.ZERO
var _ball_vel := Vector2.ZERO
var _now := 0.0
var _caught := false
var _catch_timer := 0.0
var _cooldown := 0.0
var _arms_timer := 0.0
var _walk_target := 360.0
var _walk_timer := 0.0
var _idle := 0.0
var _base_y := 0.0
var _rng := RandomNumberGenerator.new()
## Wandering off to the back of the corner: seen from behind (0031).
var _away := false


func _ready() -> void:
	_rng.randomize()
	add_child(PlayBackground.new())
	_build_walls()
	friend = FriendView.new()
	friend.scale = Vector2(FRIEND_SCALE, FRIEND_SCALE)
	add_child(friend)
	ball = ToyBall.new()
	ball.position = Vector2(520, 700)
	add_child(ball)
	fx = FxLayer.new()
	fx.floor_y = FLOOR_Y
	add_child(fx)
	friend.fx = fx
	camera = Camera2D.new()
	camera.position = VIEW_CENTER
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
		friend.setup(Save.friend)
		_base_y = FLOOR_Y - friend.feet_y() * FRIEND_SCALE
		friend.position = Vector2(300, _base_y)
		_idle = 0.0
		camera.make_current()


func _build_walls() -> void:
	var body := StaticBody2D.new()
	add_child(body)
	for r in [Rect2(-600, FLOOR_Y, 1920, 400), Rect2(-400, -400, 430, 1800), Rect2(690, -400, 430, 1800), Rect2(-600, -400, 1920, 520)]:
		var cs := CollisionShape2D.new()
		var shape := RectangleShape2D.new()
		shape.size = r.size
		cs.shape = shape
		cs.position = r.position + r.size * 0.5
		body.add_child(cs)


func to_world(screen: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * screen


# --- Input --------------------------------------------------------------

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.index == 0:
		if event.pressed:
			_press(event.position)
		else:
			_release(event.position)
	elif event is InputEventScreenDrag and event.index == 0:
		_drag(event.position, event.relative, event.velocity)


func _press(sp: Vector2) -> void:
	_idle = 0.0
	if hud.back_rect().grow(10).has_point(sp):
		go_to.emit("tent")
		return
	var w := to_world(sp)
	_last_world = w
	_press_time = _now
	_moved = 0.0
	_ball_vel = Vector2.ZERO
	if not _caught and w.distance_to(ball.global_position) < ToyBall.RADIUS + 34.0:
		_mode = "ball"
		ball.freeze = true
		ball.global_position = w
		return
	if friend.is_on_friend(friend.to_local(w)):
		_mode = "touch"
	else:
		_mode = ""


func _drag(sp: Vector2, rel: Vector2, vel: Vector2) -> void:
	var w := to_world(sp)
	_moved += rel.length()
	match _mode:
		"ball":
			ball.global_position = Vector2(clampf(w.x, 60, 660), clampf(w.y, 160, FLOOR_Y - ToyBall.RADIUS))
			_ball_vel = vel / camera.zoom.x
		"touch":
			if _moved > 30.0:
				friend.pet(friend.to_local(w), (w - _last_world).length() / FRIEND_SCALE)
	_last_world = w


func _release(sp: Vector2) -> void:
	var w := to_world(sp)
	match _mode:
		"ball":
			ball.freeze = false
			ball.linear_velocity = _ball_vel.limit_length(1700.0)
			ball.angular_velocity = _ball_vel.x * 0.01
		"touch":
			if _moved < 30.0 and _now - _press_time < 0.45:
				friend.poke(friend.to_local(w))
			friend.end_pet()
	_mode = ""


# --- Play ---------------------------------------------------------------

func _physics_process(delta: float) -> void:
	_cooldown -= delta
	var data := Save.friend
	if _caught:
		ball.global_position = friend.to_global(friend.head_center() + Vector2(0, -FriendView.HEAD_R - 40.0))
		_catch_timer -= delta
		if _catch_timer <= 0.0:
			_caught = false
			ball.freeze = false
			ball.linear_velocity = Vector2(randf_range(-280, 280), -880)
			friend.arms_up = false
			_cooldown = 0.6
		return
	if _mode == "ball" or _cooldown > 0.0 or data.asleep:
		return
	# The ball can meet any part of the friend (pillar 6: more than one
	# way to play with a toy).
	if ball.linear_velocity.length() < 120.0:
		return
	var local := friend.to_local(ball.global_position)
	var r := ToyBall.RADIUS / FRIEND_SCALE
	var head := friend.head_center()
	var hr := FriendArt.head_ry(data) + r
	if local.distance_to(head) < hr and friend.facing == "front":
		_at_head(data)
		return
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		var part: PartData = data.part(slot)
		if part == null or not part.attached:
			continue
		var q := Geometry2D.get_closest_point_to_segment(local, friend.socket(slot), friend.tip(slot))
		if local.distance_to(q) < friend.limb_width(slot) * 0.5 + r:
			if slot.begins_with("leg") and local.distance_to(friend.tip(slot)) < 60.0 + r:
				_at_foot(data, slot)
			else:
				_off_part(slot, local - q)
			return
	var rx := 112.0 * friend.body_scale() * friend.body_w() + r
	var ry := 132.0 * friend.body_h() + r
	if pow(local.x / rx, 2) + pow(local.y / ry, 2) <= 1.0:
		_off_part("", local)


## At the head: they try to catch it. Catching is learned by throwing
## again and again; even a good catcher fumbles now and then (0033).
func _at_head(data: FriendData) -> void:
	var head := friend.to_global(friend.head_center())
	var skill := data.catch_skill
	data.catch_skill = FriendData.learn(skill)
	Save.mark_dirty()
	_idle = 0.0
	match FriendData.try_skill(skill, _rng):
		"catch":
			_caught = true
			_catch_timer = 1.3
			ball.freeze = true
			friend.arms_up = true
			friend.set_expression("squint", "open", 1.3)
			Events.friend_reacted.emit("catch")
		"fumble":
			# Got it... and it pops out of their arms.
			friend.arms_up = true
			_arms_timer = 0.5
			ball.linear_velocity = Vector2(randf_range(-160, 160), -380)
			friend.set_expression("open", "o", 0.8)
			Events.friend_reacted.emit("giggle")
			_cooldown = 0.6
		_:
			ball.linear_velocity = (ball.global_position - head).normalized() * 480.0 + Vector2(0, -160)
			friend.bonk()
			# A late grab at the ball: they're still learning.
			friend.arms_up = true
			_arms_timer = 0.35
			_cooldown = 0.7


## At a foot: sometimes they kick it back, better with practice.
func _at_foot(data: FriendData, slot: String) -> void:
	var skill := data.kick_skill
	data.kick_skill = FriendData.learn(skill)
	Save.mark_dirty()
	_idle = 0.0
	var side := -1.0 if slot == "leg_l" else 1.0
	if FriendData.try_skill(skill, _rng) != "miss":
		# A proper kick, up and away across the corner.
		friend.nudge_limb(slot, Vector2(-side * 420.0, -380.0))
		ball.linear_velocity = Vector2(-side * randf_range(380, 620), randf_range(-760, -520))
		friend.set_expression("squint", "smile", 0.8)
		Events.friend_reacted.emit("cheer")
	else:
		friend.nudge_limb(slot, Vector2(side * 200.0, -120.0))
		ball.linear_velocity = Vector2(side * 260.0, -240.0)
		friend.set_expression("open", "o", 0.5)
		Events.friend_reacted.emit("boop")
	_cooldown = 0.6


## Off an arm or the body: a bounce and a little reaction.
func _off_part(slot: String, away: Vector2) -> void:
	var dir := away.normalized() if away.length() > 0.1 else Vector2.UP
	ball.linear_velocity = (dir * 420.0 + Vector2(0, -260)).rotated(friend.rotation)
	if slot == "":
		friend.bounce_off()
	else:
		friend.nudge_limb(slot, -dir * 300.0)
		Events.friend_reacted.emit("boop")
	_idle = 0.0
	_cooldown = 0.5


func _process(delta: float) -> void:
	_now += delta
	var data := Save.friend
	if _arms_timer > 0.0:
		_arms_timer -= delta
		if _arms_timer <= 0.0 and not _caught:
			friend.arms_up = false
	_idle += delta
	if _idle > 40.0 and not data.asleep:
		data.asleep = true
		Save.mark_dirty()
	var legs := 0
	for slot in ["leg_l", "leg_r"]:
		if data.part(slot) != null and data.part(slot).attached:
			legs += 1
	var legs_ok := legs == 2
	# Two legs walk; one leg hops; none, and they crawl (decision 0025).
	var speed: float = [30.0, 55.0, 80.0][legs]
	var walking := false
	if not data.asleep and not _caught:
		_walk_timer -= delta
		if _walk_timer <= 0.0:
			_walk_timer = randf_range(2.5, 5.0)
			_away = false
			if ball.global_position.y > FLOOR_Y - 60.0 and ball.linear_velocity.length() < 60.0 and _mode != "ball":
				_walk_target = ball.global_position.x
			else:
				_walk_target = randf_range(150, 570)
				# Now and then they wander to the back, facing away.
				_away = randf() < 0.25
		_walk_target = clampf(_walk_target, 140, 580)
		var dx := _walk_target - friend.position.x
		if absf(dx) > 40.0:
			friend.position.x += signf(dx) * minf(absf(dx), speed * delta)
			friend.walk_phase += delta * (9.0 if legs == 2 else 5.0)
			walking = true
	var far := 1.0 if (_away and walking) else 0.0
	friend.facing = "back" if far > 0.0 else "front"
	var k := FRIEND_SCALE * (1.0 - 0.1 * far)
	friend.scale = Vector2(k, k)
	if walking:
		match legs:
			2: friend.position.y = _base_y - absf(sin(friend.walk_phase)) * 7.0
			1: friend.position.y = _base_y - absf(sin(friend.walk_phase)) * 30.0
			_: friend.position.y = _base_y - absf(sin(friend.walk_phase * 2.0)) * 3.0
	else:
		friend.walk_phase = 0.0
		friend.position.y = move_toward(friend.position.y, _base_y, delta * 60.0)
	var vs := get_viewport_rect().size
	hint.position = Vector2(40, 118)
	hint.size = Vector2(vs.x - 80, 120)
	hint.text = _hint(data, legs_ok)


func _hint(data: FriendData, legs_ok: bool) -> String:
	if data.asleep:
		return "Napping. Poke them to wake them up."
	if not legs_ok:
		return "Missing a leg, they hop; with none, they crawl. A leg can be sewn on in the tent."
	if data.catch_skill < 0.3:
		return "Throw the ball to them. They'll catch better the more you play."
	if data.catch_skill < 0.6:
		return "They're getting the hang of it."
	return ""


class PlayHud:
	extends Node2D

	func back_rect() -> Rect2:
		return Rect2(24.0, 24.0, 80.0, 80.0)

	func _draw() -> void:
		Icons.button(self, back_rect().get_center(), Color(0.42, 0.55, 0.38), "tent")


class PlayBackground:
	extends Node2D

	func _draw() -> void:
		var floor_y := 1000.0
		draw_rect(Rect2(-700, -700, 2120, 2800), Color(0.24, 0.17, 0.13))
		draw_colored_polygon(PackedVector2Array([Vector2(-100, 120), Vector2(820, 120), Vector2(820, floor_y), Vector2(-100, floor_y)]), Color(0.78, 0.66, 0.5))
		for x in [80.0, 240.0, 400.0, 560.0]:
			PlushArt.seam(self, PackedVector2Array([Vector2(x, 130), Vector2(x, floor_y - 6)]), Color(0.62, 0.5, 0.36), 12.0, 2.0)
		# Bunting
		var flags := [Color(0.82, 0.36, 0.3), Color(0.92, 0.75, 0.32), Color(0.38, 0.58, 0.72), Color(0.5, 0.68, 0.42)]
		draw_polyline(PackedVector2Array([Vector2(40, 200), Vector2(360, 250), Vector2(680, 200)]), Color(0.35, 0.25, 0.18), 3.0, true)
		for i in 9:
			var t := (i + 0.5) / 9.0
			var top := Vector2(lerpf(40, 680, t), 200 + sin(t * PI) * 50.0)
			draw_colored_polygon(PackedVector2Array([top + Vector2(-20, 0), top + Vector2(20, 0), top + Vector2(0, 40)]), flags[i % 4])
		# Floor, rug and toy box
		draw_rect(Rect2(-700, floor_y, 2120, 1400), Color(0.36, 0.27, 0.2))
		PlushArt.ellipse(self, Vector2(360, floor_y + 30), 300.0, 60.0, Color(0.38, 0.52, 0.6), false)
		PlushArt.ellipse(self, Vector2(360, floor_y + 30), 230.0, 42.0, Color(0.85, 0.78, 0.6), false)
		draw_rect(Rect2(560, floor_y - 110, 130, 110), Color(0.62, 0.38, 0.26))
		draw_rect(Rect2(552, floor_y - 126, 146, 22), Color(0.5, 0.3, 0.2))
		draw_rect(Rect2(560, floor_y - 110, 130, 110), Color(0.3, 0.18, 0.12), false, 3.0)
		draw_circle(Vector2(625, floor_y - 60), 14.0, Color(0.92, 0.75, 0.32))
