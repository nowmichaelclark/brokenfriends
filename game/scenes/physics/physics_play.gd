class_name PhysicsPlay
extends Node2D
## The physics test (prototype 4), reached by `#physics` (or
## `-- --physics`; `&three` starts with three friends), never from the
## tent: friends as soft ragdolls to drag and fling, with a ramp, a
## step and a skateboard to land on, ride down and fall off. Debug
## buttons (words are fine here) change the friend. The friends are
## copies or made up: nothing here is saved.

const FLOOR_Y := 1180.0
## The ledge at the top of the ramp, where the board waits.
const LEDGE := Vector2(210, 900)
const RAMP_FOOT := Vector2(495, FLOOR_Y)
const STEP := Rect2(620, 1100, 100, 80)
## Where friends are dropped in: over the floor between ramp and step.
const DROP_AT := Vector2(560, 300)

var fx: FxLayer
var camera: Camera2D
var board: Skateboard
var layer: CanvasLayer
var friends: Array[Ragdoll] = []
## Which test friend comes next (see `variant`).
var next := 0

var _held: Ragdoll
var _buttons: Array[Button] = []
var _label: Label


func _ready() -> void:
	_build_world()
	board = Skateboard.new()
	board.position = _board_spot()
	add_child(board)
	fx = FxLayer.new()
	fx.floor_y = FLOOR_Y
	add_child(fx)
	camera = Camera2D.new()
	camera.position = Vector2(360, 640)
	add_child(camera)
	camera.make_current()
	layer = CanvasLayer.new()
	add_child(layer)
	var row := HBoxContainer.new()
	row.position = Vector2(12, 14)
	row.add_theme_constant_override("separation", 8)
	layer.add_child(row)
	for b in [["next friend", show_next], ["three at once", show_three], ["board", reset_board], ["drop", drop_again]]:
		var button := Button.new()
		button.text = b[0]
		button.add_theme_font_size_override("font_size", 24)
		button.focus_mode = Control.FOCUS_NONE
		button.pressed.connect(b[1])
		row.add_child(button)
		_buttons.append(button)
	_label = Label.new()
	_label.position = Vector2(14, 72)
	_label.add_theme_font_size_override("font_size", 22)
	_label.add_theme_color_override("font_color", Color(0.25, 0.17, 0.12))
	layer.add_child(_label)
	if Flags.has("three"):
		show_three()
	else:
		show_next()


func _build_world() -> void:
	var world := StaticBody2D.new()
	var mat := PhysicsMaterial.new()
	mat.friction = 0.9
	world.physics_material_override = mat
	world.collision_layer = 1
	world.collision_mask = 3
	add_child(world)
	var pieces := [
		# Floor, walls, ceiling: thick, so nothing slips through.
		PackedVector2Array([Vector2(-200, FLOOR_Y), Vector2(920, FLOOR_Y), Vector2(920, FLOOR_Y + 300), Vector2(-200, FLOOR_Y + 300)]),
		PackedVector2Array([Vector2(-200, -200), Vector2(0, -200), Vector2(0, FLOOR_Y), Vector2(-200, FLOOR_Y)]),
		PackedVector2Array([Vector2(720, -200), Vector2(920, -200), Vector2(920, FLOOR_Y), Vector2(720, FLOOR_Y)]),
		PackedVector2Array([Vector2(-200, -200), Vector2(920, -200), Vector2(920, 0), Vector2(-200, 0)]),
		# The ledge and the ramp down from it.
		PackedVector2Array([Vector2(0, LEDGE.y), LEDGE, RAMP_FOOT, Vector2(0, FLOOR_Y)]),
		# The step.
		PackedVector2Array([STEP.position, Vector2(STEP.end.x, STEP.position.y), STEP.end, Vector2(STEP.position.x, STEP.end.y)]),
	]
	for pts in pieces:
		var cs := CollisionShape2D.new()
		var shape := ConvexPolygonShape2D.new()
		shape.points = pts
		cs.shape = shape
		world.add_child(cs)


func _draw() -> void:
	draw_rect(Rect2(0, 0, 720, FLOOR_Y), Color(0.96, 0.9, 0.8))
	draw_rect(Rect2(0, FLOOR_Y, 720, 200), Color(0.62, 0.48, 0.36))
	var ramp := PackedVector2Array([Vector2(0, LEDGE.y), LEDGE, RAMP_FOOT, Vector2(0, FLOOR_Y)])
	draw_colored_polygon(ramp, Color(0.72, 0.58, 0.42))
	draw_polyline(PackedVector2Array([Vector2(0, LEDGE.y), LEDGE, RAMP_FOOT]), Color(0.45, 0.33, 0.22), 5.0, true)
	draw_rect(STEP, Color(0.72, 0.58, 0.42))
	draw_rect(STEP, Color(0.45, 0.33, 0.22), false, 5.0)


## The test friends in turn: a copy of the friend in front at camp,
## then tiny, huge, no legs, one arm, soaked, and one with a loose seam.
static func variant(n: int) -> Dictionary:
	var f: FriendData
	var size := 1.0
	var label := ""
	match n % 7:
		0:
			if Save.friend and not Save.friend.dead and not Save.friend.headless:
				f = FriendData.from_dict(Save.friend.to_dict())
				label = "the friend in front (a copy)"
			else:
				f = FriendData.test_friend(7)
				label = "a friend"
		1:
			f = FriendData.test_friend(11)
			size = 0.6
			label = "tiny"
		2:
			f = FriendData.test_friend(23)
			size = 1.5
			label = "huge"
		3:
			f = FriendData.test_friend(31)
			f.parts["leg_l"] = null
			f.parts["leg_r"] = null
			label = "no legs"
		4:
			f = FriendData.test_friend(42)
			f.parts["arm_r"] = null
			label = "one arm"
		5:
			f = FriendData.test_friend(57)
			f.wetness = 1.0
			label = "soaked"
		6:
			f = FriendData.test_friend(64)
			var arm: PartData = f.parts["arm_l"]
			arm.pulls = arm.pulls_needed() - 1
			label = "a loose seam (left arm)"
	return {"friend": f, "size": size, "label": label}


## Clear the friends away and drop new ones in.
func spawn(variants: Array, xs: Array) -> void:
	for r in friends:
		r.queue_free()
	friends.clear()
	_held = null
	var labels: Array = []
	for i in variants.size():
		var v := variant(int(variants[i]))
		var r := Ragdoll.new()
		r.fx = fx
		add_child(r)
		r.setup(v["friend"], Vector2(float(xs[i]), DROP_AT.y), float(v["size"]))
		friends.append(r)
		labels.append(v["label"])
	_label.text = ", ".join(labels)


func show_next() -> void:
	spawn([next], [DROP_AT.x])
	next += 1


func show_three() -> void:
	spawn([next, next + 1, next + 2], [200.0, 380.0, 560.0])
	next += 3


func reset_board() -> void:
	board.place(_board_spot())


## On the ledge, at the top of the ramp.
func _board_spot() -> Vector2:
	return Vector2(100, LEDGE.y - Skateboard.DECK * 0.5 - Skateboard.WHEEL_R * 2.0 + 2.0)


## The same friends again, from the top.
func drop_again() -> void:
	var xs: Array = []
	var vs: Array = []
	for r in friends:
		xs.append(r.bodies["body"].global_position.x)
	for i in friends.size():
		vs.append((next - friends.size() + i))
	spawn(vs, xs)


# --- The finger -------------------------------------------------------------

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.index == 0:
		var w := _world(event.position)
		if event.pressed:
			_press(event.position, w)
		elif _held:
			_held.release()
			_held = null
	elif event is InputEventScreenDrag and event.index == 0 and _held:
		_held.drag_to(_world(event.position))


func _press(screen: Vector2, w: Vector2) -> void:
	for b in _buttons:
		if b.get_global_rect().has_point(screen):
			return
	var q := PhysicsPointQueryParameters2D.new()
	q.position = w
	q.collision_mask = 2
	for hit in get_world_2d().direct_space_state.intersect_point(q, 8):
		var body: Object = hit["collider"]
		if body.has_meta("ragdoll"):
			_held = body.get_meta("ragdoll")
			_held.grab(body, w)
			return


func _world(screen: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * screen
