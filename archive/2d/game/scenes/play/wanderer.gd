class_name Wanderer
extends RefCounted
## One friend wandering about the play room: where it is going, how it
## walks (two legs walk, one hops, none crawl; decision 0025), when it
## turns its back to walk away (0031), and when it naps.

## The floor's depth: feet from the back of the room to the front.
const BACK_Y := 900.0
const FRONT_Y := 1060.0
## Scale at the back and at the front.
const FAR_K := 0.56
const NEAR_K := 0.74
## Seconds alone before a nap.
const NAP_AFTER := 40.0

var view: FriendView
var data: FriendData
## 0 at the back of the room, 1 at the front.
var depth := 0.5
var target := Vector2.ZERO
var walk_timer := 0.0
var away := false
var idle := 0.0
## After a ball meets it, a moment before it can again.
var cooldown := 0.0
var arms_timer := 0.0
var caught := false
var catch_timer := 0.0
## Hold still where it is (a test, or a ball being aimed at it).
var hold := false

var _rng: RandomNumberGenerator
var _room_w := 0.0


func _init(friend: FriendData, x: float, d: float, room_w: float, rng: RandomNumberGenerator) -> void:
	data = friend
	depth = d
	_room_w = room_w
	_rng = rng
	view = FriendView.new()
	view.position = Vector2(x, 0)
	target = Vector2(x, d)
	walk_timer = rng.randf_range(0.0, 3.0)


func k() -> float:
	return lerpf(FAR_K, NEAR_K, depth)


func feet_y() -> float:
	return lerpf(BACK_Y, FRONT_Y, depth)


func legs() -> int:
	var n := 0
	for slot in ["leg_l", "leg_r"]:
		var leg: PartData = data.part(slot)
		if leg != null and leg.attached:
			n += 1
	return n


## Stand on the floor where it is (once set up).
func place() -> void:
	var kk := k()
	view.scale = Vector2(kk, kk)
	view.position.y = feet_y() - view.feet_y() * kk
	view.z_index = int(depth * 100.0)


func poked() -> void:
	idle = 0.0


func process(delta: float, ball_rest_x: float) -> void:
	cooldown -= delta
	if arms_timer > 0.0:
		arms_timer -= delta
		if arms_timer <= 0.0 and not caught:
			view.arms_up = false
	idle += delta
	if idle > NAP_AFTER and not data.asleep and not data.dead:
		data.asleep = true
		Save.mark_dirty()
	var n := legs()
	# Two legs walk; one leg hops; none, and they crawl.
	var speed: float = [30.0, 55.0, 80.0][n]
	var walking := false
	if not data.asleep and not caught and not hold:
		walk_timer -= delta
		if walk_timer <= 0.0:
			walk_timer = _rng.randf_range(2.5, 6.0)
			away = false
			if ball_rest_x > -INF and absf(ball_rest_x - view.position.x) < 600.0 and _rng.randf() < 0.5:
				# Go and look at the ball lying still.
				target = Vector2(ball_rest_x, 1.0)
			else:
				target = Vector2(view.position.x + _rng.randf_range(-420.0, 420.0), _rng.randf())
				# Now and then they wander to the back, facing away.
				away = target.y < depth - 0.2 and _rng.randf() < 0.6
		target.x = clampf(target.x, 160.0, _room_w - 160.0)
		var to := Vector2(target.x - view.position.x, (target.y - depth) * 300.0)
		if to.length() > 30.0:
			var step := to.normalized() * minf(to.length(), speed * delta)
			view.position.x += step.x
			depth = clampf(depth + step.y / 300.0, 0.0, 1.0)
			view.walk_phase += delta * (9.0 if n == 2 else 5.0)
			walking = true
	view.facing = "back" if away and walking else "front"
	var kk := k()
	view.scale = Vector2(kk, kk)
	var base := feet_y() - view.feet_y() * kk
	if walking:
		match n:
			2: view.position.y = base - absf(sin(view.walk_phase)) * 7.0
			1: view.position.y = base - absf(sin(view.walk_phase)) * 30.0
			_: view.position.y = base - absf(sin(view.walk_phase * 2.0)) * 3.0
	else:
		view.walk_phase = 0.0
		view.position.y = move_toward(view.position.y, base, delta * 60.0)
	# Nearer the front draws over further back.
	view.z_index = int(depth * 100.0)
