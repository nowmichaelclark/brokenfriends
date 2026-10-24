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


func _ready() -> void:
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
	var head := friend.to_global(friend.head_center())
	var reach := FriendView.HEAD_R * FRIEND_SCALE + ToyBall.RADIUS + 8.0
	if ball.global_position.distance_to(head) < reach and ball.linear_velocity.length() > 120.0:
		var skill := data.catch_skill
		data.catch_skill = minf(0.92, skill + 0.07)
		Save.mark_dirty()
		_idle = 0.0
		if randf() < skill:
			_caught = true
			_catch_timer = 1.3
			ball.freeze = true
			friend.arms_up = true
			friend.set_expression("squint", "open", 1.3)
			Events.friend_reacted.emit("catch")
		else:
			ball.linear_velocity = (ball.global_position - head).normalized() * 480.0 + Vector2(0, -160)
			friend.bonk()
			# A late grab at the ball: they're still learning.
			friend.arms_up = true
			_arms_timer = 0.35
			_cooldown = 0.7


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
	var legs_ok := data.part("leg_l") != null and data.part("leg_r") != null \
		and data.part("leg_l").attached and data.part("leg_r").attached
	var walking := false
	if legs_ok and not data.asleep and not _caught:
		_walk_timer -= delta
		if _walk_timer <= 0.0:
			_walk_timer = randf_range(2.5, 5.0)
			if ball.global_position.y > FLOOR_Y - 60.0 and ball.linear_velocity.length() < 60.0 and _mode != "ball":
				_walk_target = ball.global_position.x
			else:
				_walk_target = randf_range(150, 570)
		_walk_target = clampf(_walk_target, 140, 580)
		var dx := _walk_target - friend.position.x
		if absf(dx) > 40.0:
			friend.position.x += signf(dx) * minf(absf(dx), 80.0 * delta)
			friend.walk_phase += delta * 9.0
			walking = true
	if walking:
		friend.position.y = _base_y - absf(sin(friend.walk_phase)) * 7.0
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
		return "With a leg missing, they can't walk. Sew one on in the tent."
	if data.catch_skill < 0.3:
		return "Throw the ball to them. They'll catch better the more you play."
	if data.catch_skill < 0.6:
		return "They're getting the hang of it."
	return ""


class PlayHud:
	extends Node2D

	func back_rect() -> Rect2:
		return Rect2(28.0, 28.0, 190.0, 64.0)

	func _draw() -> void:
		var r := back_rect()
		var col := Color(0.42, 0.55, 0.38)
		draw_rect(r, col)
		draw_rect(r, col.darkened(0.4), false, 3.0)
		draw_string(ThemeDB.fallback_font, Vector2(r.position.x, r.position.y + r.size.y * 0.5 + 9.0), "Back to tent", HORIZONTAL_ALIGNMENT_CENTER, r.size.x, 26, Color(1, 0.96, 0.88))


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
