class_name Ragdoll
extends Node2D
## A friend as a soft ragdoll (prototype 4): body, head, arms and legs
## are each a body in Godot's built-in 2D physics (decision 0005),
## pinned together at the seams. The friend's data sets the physics:
## stuffing its weight and bounce, wetness more weight. A limb that
## hits something hard counts as a hard pull on its seam (decision
## 0025), so a loose seam tears, and the torn part stays torn in the
## friend's data.
## Each part is drawn once into a picture and then only moved (Baked).

## Friend units to pixels, at size 1.
const SCALE := 0.42
const LIMBS := ["arm_l", "arm_r", "leg_l", "leg_r"]

# Guesses, all waiting on Mikey's play (game/README.md, "Assumed").
## Weights, the body being 1.
const MASS := {"body": 1.0, "head": 0.6, "arm": 0.12, "leg": 0.18}
## How hard gravity pulls, times the engine's default.
const GRAVITY := 2.0
## How fast a part must be going when something stops it (pixels a
## second) for the landing to count as a hard pull on its seam. A drop
## from the top of the screen lands at about 1700.
const HARD := 2500.0
## A change of speed within one physics step this big means it hit
## something.
const JOLT := 800.0
## How much the pinned seams give (Godot's PinJoint2D softness).
const SOFTNESS := 0.0
## How firmly each part springs back to its pose (swings a second),
## how much that swing is damped, and how far it swings freely before
## it is held back (radians).
const POSE_HZ := {"arm": 0.9, "leg": 1.6, "head": 2.2}
const POSE_DAMP := 0.25
const SWING := {"arm": 2.4, "leg": 1.1, "head": 0.55}
## Fastest a finger can throw a part, pixels a second.
const FLING_MAX := 4200.0

var data: FriendData
var size := 1.0
var fx: FxLayer
## "body", "head" and limb slots -> RigidBody2D.
var bodies: Dictionary = {}
## Parts torn off here, by slot.
var torn: Dictionary = {}
## The fastest any limb has hit something, pixels a second (for tests
## and tuning).
var max_impact := 0.0

## Seam name ("neck" or a limb slot) -> {joint, a, b, at_a, rest, kind, last_vel, cooldown}.
var _seams: Dictionary = {}
var _pics: Dictionary = {}
var _grab: RigidBody2D
var _grab_at := Vector2.ZERO
var _target := Vector2.ZERO
var _last_vel := Vector2.ZERO
var _thud_cd := 0.0


func k() -> float:
	return SCALE * size


## Build the ragdoll with its body's centre at `at` (world).
func setup(friend: FriendData, at: Vector2, friend_size := 1.0) -> void:
	data = friend
	size = friend_size
	var kk := k()
	var s := FriendArt.body_scale(data)
	var rx := 112.0 * s * float(data.bl("w"))
	var ry := 132.0 * float(data.bl("h"))
	var body := _make("body", _ellipse_shape(rx * kk, ry * kk), Vector2.ZERO, MASS["body"], (rx * rx + ry * ry) * kk * kk / 4.0)
	body.position = at
	_picture("body", body, Rect2(-180, -180, 360, 380), _paint_body)
	if not data.headless:
		var hrx := FriendArt.head_rx(data)
		var hry := FriendArt.head_ry(data)
		var head := _make("head", _ellipse_shape(hrx * kk, hry * kk), Vector2.ZERO, MASS["head"], (hrx * hrx + hry * hry) * kk * kk / 4.0)
		head.position = at + FriendRig.HEAD_C * kk
		_picture("head", head, Rect2(-150, -280, 300, 430), _paint_head)
		_seam("neck", body, head, FriendArt.socket(data, "neck") * kk, "head")
	for slot in LIMBS:
		var part: PartData = data.part(slot)
		if part == null or not part.attached:
			continue
		_add_limb(slot, part, body, at)
	var all := bodies.values()
	for i in all.size():
		for j in range(i + 1, all.size()):
			all[i].add_collision_exception_with(all[j])


func _add_limb(slot: String, part: PartData, body: RigidBody2D, at: Vector2) -> void:
	var kk := k()
	var length := FriendRig.LEG_LEN if part.kind == "leg" else FriendRig.ARM_LEN
	var w := FriendArt.limb_width(data, slot)
	var shape := CapsuleShape2D.new()
	shape.radius = w * 0.5 * kk
	shape.height = (length + w) * kk
	var m: float = MASS[part.kind]
	var limb := _make(slot, shape, Vector2(0, length * 0.5 * kk), m, (length * length + w * w) * kk * kk / 12.0)
	var socket := FriendArt.socket(data, slot) * kk
	limb.position = at + socket
	limb.rotation = _rest_dir(slot).angle() - PI * 0.5
	_picture(slot, limb, Rect2(-40, -40, 80, 185), _paint_limb.bind(slot, part))
	_seam(slot, body, limb, socket, part.kind)


func _rest_dir(slot: String) -> Vector2:
	match slot:
		"arm_l": return Vector2(-0.4, 0.92).normalized()
		"arm_r": return Vector2(0.4, 0.92).normalized()
		"leg_l": return Vector2(-0.06, 1.0).normalized()
	return Vector2(0.06, 1.0).normalized()


## The physics material for this friend: plump friends bounce more,
## soaked ones hardly at all and grip more.
func _material() -> PhysicsMaterial:
	var mat := PhysicsMaterial.new()
	mat.bounce = clampf(0.12 + 0.3 * (data.stuffing - 0.6), 0.02, 0.4) * (1.0 - 0.8 * data.wetness)
	mat.friction = 0.7 + 0.3 * data.wetness
	return mat


## Weight for a part: bigger friends weigh more (by area), stuffing
## fills them out, and water soaks in.
func _weight(base: float) -> float:
	var stuffed := lerpf(0.6, 1.3, clampf((data.stuffing - 0.35) / 1.1, 0.0, 1.0))
	return base * size * size * stuffed * (1.0 + data.wetness)


func _make(id: String, shape: Shape2D, shape_at: Vector2, base_mass: float, inertia_per_mass: float) -> RigidBody2D:
	var b := RigidBody2D.new()
	b.mass = _weight(base_mass)
	b.inertia = b.mass * inertia_per_mass
	b.center_of_mass_mode = RigidBody2D.CENTER_OF_MASS_MODE_CUSTOM
	b.center_of_mass = shape_at
	b.gravity_scale = GRAVITY
	b.physics_material_override = _material()
	# No continuous collision detection: with it, joined parts land
	# bouncing higher than they fell (Godot 4.7). The walls are thick.
	b.collision_layer = 2
	b.collision_mask = 3
	var cs := CollisionShape2D.new()
	cs.shape = shape
	cs.position = shape_at
	b.add_child(cs)
	b.set_meta("ragdoll", self)
	add_child(b)
	bodies[id] = b
	return b


func _ellipse_shape(rx: float, ry: float) -> ConvexPolygonShape2D:
	var shape := ConvexPolygonShape2D.new()
	shape.points = PlushArt.ellipse_points(Vector2.ZERO, rx, ry, 16)
	return shape


func _picture(id: String, on: Node2D, rect: Rect2, painter: Callable) -> void:
	var pic := Baked.new()
	pic.scale = Vector2(k(), k())
	on.add_child(pic)
	pic.setup(rect, painter, 1.5 * k())
	_pics[id] = pic


## Pin `b` to `a` at `at_a` (in a's units), where the seam is.
func _seam(id: String, a: RigidBody2D, b: RigidBody2D, at_a: Vector2, kind: String) -> void:
	var j := PinJoint2D.new()
	j.position = a.position + at_a
	j.softness = SOFTNESS
	add_child(j)
	j.node_a = j.get_path_to(a)
	j.node_b = j.get_path_to(b)
	_seams[id] = {
		"joint": j, "a": a, "b": b, "at_a": at_a, "rest": b.rotation - a.rotation,
		"kind": kind, "last_vel": Vector2.ZERO, "cooldown": 0.0,
	}


func _physics_process(delta: float) -> void:
	for id in _seams.keys():
		var sm: Dictionary = _seams[id]
		var a: RigidBody2D = sm["a"]
		var b: RigidBody2D = sm["b"]
		# Each part springs back to its pose, softly (a plush toy, not a
		# puppet on stiff wires).
		var off := wrapf(b.rotation - a.rotation - float(sm["rest"]), -PI, PI)
		var w: float = TAU * POSE_HZ[sm["kind"]]
		var inertia := b.inertia * a.inertia / (b.inertia + a.inertia)
		var spin := b.angular_velocity - a.angular_velocity
		# Past its swing, a part is held back firmly. (The joint's own
		# angular limits wrench the seams open in Godot 4.7, so not those.)
		var swing: float = SWING[sm["kind"]]
		var past := signf(off) * maxf(0.0, absf(off) - swing)
		var torque := -inertia * (w * w * (off + 30.0 * past) + 2.0 * POSE_DAMP * w * spin)
		b.apply_torque(torque)
		a.apply_torque(-torque)
		_check_jolt(id, sm, delta)
	if _grab:
		var p := _grab.to_global(_grab_at)
		var want := ((_target - p) * 22.0).limit_length(FLING_MAX)
		_grab.linear_velocity = _grab.linear_velocity.lerp(want, 0.6)
		_grab.angular_velocity *= 0.8
	# A soft thump on a hard landing.
	_thud_cd -= delta
	var body: RigidBody2D = bodies["body"]
	if (body.linear_velocity - _last_vel).length() > 900.0 and _thud_cd <= 0.0:
		_thud_cd = 0.25
		Voice.thud()
		if data.fluid < 0.4:
			Voice.squeak()
	_last_vel = body.linear_velocity


## A limb that hits something hard (a hard landing) counts once as a
## hard pull on its seam; enough pulls (or one, on a rotting part) and
## the part tears off.
func _check_jolt(id: String, sm: Dictionary, delta: float) -> void:
	if id == "neck":
		return
	var b: RigidBody2D = sm["b"]
	var before: Vector2 = sm["last_vel"]
	sm["last_vel"] = b.linear_velocity
	sm["cooldown"] = float(sm["cooldown"]) - delta
	if b == _grab or (b.linear_velocity - before).length() < JOLT:
		return
	max_impact = maxf(max_impact, before.length())
	if before.length() < HARD or float(sm["cooldown"]) > 0.0:
		return
	sm["cooldown"] = 0.4
	var part: PartData = data.part(id)
	part.pulls += 1
	_fx("fluff", sm["a"].to_global(sm["at_a"]), 3, 6.0)
	Voice.tear(0.15)
	if part.pulls >= _pulls_needed(part):
		tear(id)


func _pulls_needed(part: PartData) -> int:
	return 1 if part.rot >= 0.5 or data.dead else part.pulls_needed()


## Rip a part off: the seam goes, the part falls free, and the friend's
## data loses it (an open wound, a little stuffing gone).
func tear(slot: String) -> void:
	var sm: Dictionary = _seams[slot]
	var at: Vector2 = sm["a"].to_global(sm["at_a"])
	(sm["joint"] as Node).queue_free()
	_seams.erase(slot)
	var part: PartData = data.part(slot)
	torn[slot] = part
	data.parts[slot] = null
	data.stuffing = maxf(0.35, data.stuffing - 0.1)
	_pics["body"].refresh([slot, "torn"])
	_pics[slot].refresh(["torn"])
	_fx("fluff", at, 16, 12.0)
	_fx("fluid", at, 8, 6.0)
	Voice.tear(1.0)
	Events.part_changed.emit(slot, "torn")


## Every seam still holding.
func intact() -> bool:
	return torn.is_empty()


## True when nothing is moving much any more.
func settled() -> bool:
	for b in bodies.values():
		if (b as RigidBody2D).linear_velocity.length() > 25.0:
			return false
	return true


## The fastest any part is moving, pixels a second.
func speed() -> float:
	var top := 0.0
	for b in bodies.values():
		top = maxf(top, (b as RigidBody2D).linear_velocity.length())
	return top


## Set the whole friend moving (a drop, a throw).
func push(velocity: Vector2) -> void:
	for b in bodies.values():
		(b as RigidBody2D).linear_velocity = velocity
	# A push is not a jolt.
	for sm in _seams.values():
		sm["last_vel"] = velocity


# --- The finger -----------------------------------------------------------

func grab(b: RigidBody2D, world: Vector2) -> void:
	_grab = b
	_grab_at = b.to_local(world)
	_target = world
	for part in bodies.values():
		(part as RigidBody2D).sleeping = false


func drag_to(world: Vector2) -> void:
	_target = world


## Let go: whatever speed the finger gave it, it keeps (a fling).
func release() -> void:
	_grab = null


func is_grabbed() -> bool:
	return _grab != null


# --- Drawing ----------------------------------------------------------------

func _fabric() -> Color:
	return PlushArt.fabric_color(data.body_fabric, data.wetness)


func _paint_body(ci: CanvasItem) -> void:
	var s := FriendArt.body_scale(data)
	FriendArt.draw_body(ci, data, s, _fabric(), PlushArt.fabric_color(data.trim, data.wetness))
	for slot in LIMBS:
		if data.is_open(slot):
			var at := FriendArt.socket(data, slot)
			PlushArt.ellipse(ci, at, 24.0, 13.0, Color(0.14, 0.09, 0.07), false)
			for t in 3:
				PlushArt.fluff(ci, at + Vector2(-12 + t * 12, -2 + (t % 2) * 5), 8.0)


func _paint_head(ci: CanvasItem) -> void:
	FriendArt.draw_loose_head(ci, data, Transform2D.IDENTITY, false)


func _paint_limb(ci: CanvasItem, slot: String, part: PartData) -> void:
	var length := FriendRig.LEG_LEN if part.kind == "leg" else FriendRig.ARM_LEN
	PlushArt.limb(ci, Vector2.ZERO, Vector2(0, length), FriendArt.limb_width(data, slot), part, data.trim, data.wetness)
	if torn.has(slot):
		PlushArt.fluff(ci, Vector2(-8, -4), 10.0)
		PlushArt.fluff(ci, Vector2(9, -2), 8.0)


func _fx(kind: String, world: Vector2, count: int, spread: float) -> void:
	if fx:
		fx.spawn(kind, fx.to_local(world), count, spread)
