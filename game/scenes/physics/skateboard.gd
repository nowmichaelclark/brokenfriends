class_name Skateboard
extends RigidBody2D
## A skateboard for the physics test (prototype 4): a grippy deck on
## two free-spinning wheels, so it rolls down the ramp with whoever
## lands on it.

const LENGTH := 170.0
const DECK := 12.0
const WHEEL_R := 15.0

var wheels: Array[RigidBody2D] = []


func _ready() -> void:
	mass = 0.8
	gravity_scale = Ragdoll.GRAVITY
	var mat := PhysicsMaterial.new()
	mat.friction = 1.0
	mat.bounce = 0.05
	physics_material_override = mat
	collision_layer = 1
	collision_mask = 3
	var cs := CollisionShape2D.new()
	var box := RectangleShape2D.new()
	box.size = Vector2(LENGTH, DECK)
	cs.shape = box
	add_child(cs)
	for i in 2:
		var wheel := _Wheel.new()
		wheel.mass = 0.15
		wheel.gravity_scale = Ragdoll.GRAVITY
		wheel.physics_material_override = mat
		wheel.collision_layer = 1
		wheel.collision_mask = 1
		var ws := CollisionShape2D.new()
		var circle := CircleShape2D.new()
		circle.radius = WHEEL_R
		ws.shape = circle
		wheel.add_child(ws)
		wheel.top_level = true
		wheel.position = global_position + _wheel_at(i)
		add_child(wheel)
		wheel.add_collision_exception_with(self)
		wheels.append(wheel)
		var pin := PinJoint2D.new()
		pin.position = _wheel_at(i)
		add_child(pin)
		pin.node_a = pin.get_path_to(self)
		pin.node_b = pin.get_path_to(wheel)


func _wheel_at(i: int) -> Vector2:
	return Vector2((-1.0 if i == 0 else 1.0) * (LENGTH * 0.5 - 26.0), DECK * 0.5 + WHEEL_R - 2.0)


## Put the board (and its wheels) at a spot, standing still.
func place(at: Vector2) -> void:
	global_position = at
	rotation = 0.0
	linear_velocity = Vector2.ZERO
	angular_velocity = 0.0
	for i in wheels.size():
		wheels[i].global_position = at + _wheel_at(i)
		wheels[i].rotation = 0.0
		wheels[i].linear_velocity = Vector2.ZERO
		wheels[i].angular_velocity = 0.0


func _draw() -> void:
	var half := LENGTH * 0.5
	var deck := PackedVector2Array([Vector2(-half, -DECK * 0.5), Vector2(half, -DECK * 0.5),
		Vector2(half + 10, -DECK * 0.5 - 8), Vector2(half + 6, DECK * 0.5), Vector2(-half - 6, DECK * 0.5),
		Vector2(-half - 10, -DECK * 0.5 - 8)])
	draw_colored_polygon(deck, Color(0.55, 0.32, 0.5))
	draw_polyline(deck + PackedVector2Array([deck[0]]), Color(0.3, 0.16, 0.28), 3.0, true)
	draw_line(Vector2(-half + 8, -DECK * 0.5 + 2), Vector2(half - 8, -DECK * 0.5 + 2), Color(0.2, 0.2, 0.22), 3.0)


## A wheel draws itself: a tyre and a spoke, so its spin shows.
class _Wheel:
	extends RigidBody2D

	func _draw() -> void:
		draw_circle(Vector2.ZERO, Skateboard.WHEEL_R, Color(0.95, 0.9, 0.7))
		draw_arc(Vector2.ZERO, Skateboard.WHEEL_R, 0, TAU, 20, Color(0.45, 0.4, 0.3), 3.0, true)
		draw_line(Vector2(-8, 0), Vector2(8, 0), Color(0.45, 0.4, 0.3), 3.0)
