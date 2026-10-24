class_name LoosePart
extends Node2D
## A part that isn't on a creature: a torn-off leg in the player's hand,
## lying on the floor, or the spare leg being carried to a wound.

var part: PartData
var trim := Color(0.94, 0.86, 0.75)
var held := true
## The spare from the pocket goes back there if it isn't used.
var from_pocket := false
var floor_y := 890.0

var _vel := Vector2.ZERO
var _angle := 0.0
var _spin := 0.0


func _process(delta: float) -> void:
	if held:
		_angle = lerp_angle(_angle, 0.0, delta * 8.0)
	else:
		_vel.y += 1600.0 * delta
		position += _vel * delta
		_angle += _spin * delta
		if position.y > floor_y - 24.0:
			position.y = floor_y - 24.0
			_vel = Vector2.ZERO
			_angle = lerp_angle(_angle, PI * 0.5, delta * 10.0)
			_spin = 0.0
	rotation = _angle
	queue_redraw()


func drop(velocity: Vector2) -> void:
	held = false
	_vel = velocity
	_spin = velocity.x * 0.01


func pick_up() -> void:
	held = true


## True if a world point is on this part.
func hits(world: Vector2) -> bool:
	var p := to_local(world)
	var q := Geometry2D.get_closest_point_to_segment(p, Vector2(0, -50), Vector2(0, 50))
	return p.distance_to(q) < 40.0


func _draw() -> void:
	if part:
		PlushArt.limb(self, Vector2(0, -50), Vector2(0, 50), 50.0, part, trim, 0.0)
		# The torn top: fluff and a wire
		if not from_pocket:
			PlushArt.fluff(self, Vector2(-8, -56), 10.0)
			PlushArt.fluff(self, Vector2(10, -54), 8.0)
			draw_line(Vector2(2, -54), Vector2(6, -72), Color(0.85, 0.2, 0.2), 2.5, true)
