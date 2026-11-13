class_name ToyBall
extends RigidBody2D
## A soft striped ball for the play corner, using Godot's built-in
## physics (decision 0005).

const RADIUS := 30.0


func _ready() -> void:
	var shape := CollisionShape2D.new()
	var circle := CircleShape2D.new()
	circle.radius = RADIUS
	shape.shape = circle
	add_child(shape)
	mass = 0.4
	var mat := PhysicsMaterial.new()
	mat.bounce = 0.55
	mat.friction = 0.6
	physics_material_override = mat
	continuous_cd = RigidBody2D.CCD_MODE_CAST_SHAPE
	freeze_mode = RigidBody2D.FREEZE_MODE_KINEMATIC
	angular_damp = 0.8


func _draw() -> void:
	draw_circle(Vector2.ZERO, RADIUS, Color(0.92, 0.84, 0.62))
	draw_arc(Vector2.ZERO, RADIUS * 0.62, 0, TAU, 24, Color(0.82, 0.32, 0.28), 9.0, true)
	draw_line(Vector2(-RADIUS, 0), Vector2(RADIUS, 0), Color(0.3, 0.5, 0.7), 6.0, true)
	draw_arc(Vector2.ZERO, RADIUS, 0, TAU, 28, Color(0.35, 0.25, 0.18), 3.0, true)
	draw_circle(Vector2(-10, -12), 6.0, Color(1, 1, 1, 0.35))
