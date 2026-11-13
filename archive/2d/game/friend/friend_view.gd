class_name FriendView
extends FriendRig
## A friend and how its body reacts (FriendRig draws it): poking,
## tickling, petting, tugging limbs until rotten ones tear, a button eye
## dangling on its thread, wounds leaking fluff, washing, drying and
## feeding. The body is the interface (pillar 1): health shows as
## shaking, droopy lids, a slow heart patch and a pale, wet or muddy coat.

var fx: FxLayer

var _grabbed := ""
var _strain := 0.0
var _strain_fx := 0.0
var _strain_sound := 0.0
var _winced := false
var _eye_vel := Vector2.ZERO
var _eye_grabbed := false
var _squash_vel := 0.0
var _blink := 0.0
var _next_blink := 3.0
var _pet_sound := 0.0
var _leak := 0.0
var _drip := 0.0
var _zzz := 0.0
var _rng := RandomNumberGenerator.new()
## Pulling the head (decision 0026): hard pulls so far, and whether
## this grab counted one.
var _head_grabbed := false
var _neck_pulls := 0
var _pull_time := 0.0
var _pull_counted := false



func setup(friend: FriendData) -> void:
	if friend != data:
		wounds = {}
		_neck_pulls = 0
		_head_pull = Vector2.ZERO
	data = friend
	_rng.seed = data.look_seed
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		_limbs[slot] = {"off": Vector2.ZERO, "vel": Vector2.ZERO}
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if data.is_open(slot) and not wounds.has(slot):
			wounds[slot] = make_tufts()
	if data.headless and not wounds.has("neck"):
		wounds["neck"] = make_tufts()
	_eye_pos = eye_anchor("eye_r") + Vector2(0, 58)
	_lid = 1.0 if data.asleep else 0.0
	_build_parts()
	for child in [_shadow, _body, _heart, _head_art, _face, _dangle, _surgery_art]:
		child.key = null
	for slot in _limb_nodes:
		_limb_nodes[slot].key = null


# --- Hit tests ----------------------------------------------------------

## The limb or dangling eye under a local point, or "".
func part_at(p: Vector2) -> String:
	var eye: PartData = data.part("eye_r")
	if eye and not eye.attached and p.distance_to(_eye_pos) < eye_radius() + 16.0:
		return "eye_r"
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if data.part(slot) == null:
			continue
		var a := socket(slot) + limb_dir(slot) * 22.0
		var b := tip(slot)
		var q := Geometry2D.get_closest_point_to_segment(p, a, b)
		if p.distance_to(q) < limb_width(slot) * 0.65:
			return slot
	return ""


## "head_top", "head", "mouth", "tickle", "body" or "".
func region_at(p: Vector2) -> String:
	var hc := head_center()
	if p.distance_to(mouth_pos()) < 46.0:
		return "mouth"
	var hrx := FriendArt.head_rx(data) + 6.0
	var hry := FriendArt.head_ry(data) + 6.0
	if not data.headless and (pow((p.x - hc.x) / hrx, 2) + pow((p.y - hc.y) / hry, 2) <= 1.0 or _in_ears(p)):
		return "head_top" if p.y < hc.y - hry * 0.55 else "head"
	var s := body_scale()
	var spot := Vector2(data.tickle_spot.x * s, data.tickle_spot.y)
	if p.distance_to(spot) < 42.0:
		return "tickle"
	var rx := 112.0 * s * body_w()
	var ry := 132.0 * body_h()
	if pow(p.x / rx, 2) + pow(p.y / ry, 2) <= 1.05:
		return "body"
	return ""


func _in_ears(p: Vector2) -> bool:
	var length := FriendArt.ear_length(data)
	if length <= 0.0:
		return false
	for side in [-1.0, 1.0]:
		var base := head_center() + FriendArt.ear_base(data, side)
		var tip_at := base + Vector2(0, -length).rotated(FriendArt.ear_angle(data, side))
		var q := Geometry2D.get_closest_point_to_segment(p, base, tip_at)
		if p.distance_to(q) < 30.0:
			return true
	return false


func is_on_friend(p: Vector2) -> bool:
	return region_at(p) != "" or part_at(p) != ""


# --- Reactions ----------------------------------------------------------

func set_expression(eyes: String, mouth: String, seconds: float) -> void:
	_expr_eyes = eyes
	_expr_mouth = mouth
	_expr_timer = seconds


func wake() -> void:
	if not data.asleep:
		return
	data.asleep = false
	set_expression("half", "flat", 1.6)
	_squash_vel -= 2.0
	Events.friend_reacted.emit("wake")
	Save.mark_dirty()


## A short tap. Returns the reaction it caused.
func poke(p: Vector2) -> String:
	if data.asleep:
		wake()
		return "wake"
	var kind := "boop"
	match region_at(p):
		"head_top":
			kind = "dislike"
			_shake = 1.0
			set_expression("open", "frown", 0.9)
		"tickle":
			kind = "giggle"
			_wiggle = 1.0
			set_expression("squint", "open", 1.1)
			_fx("sparkle", p, 4, 14.0)
		_:
			if data.wellbeing() < 0.4:
				kind = "wince"
				set_expression("squint", "frown", 0.6)
			else:
				set_expression("blink", "o", 0.35)
	_squash_vel += 2.0
	if data.fluid < 0.4:
		Voice.squeak()
	Events.friend_reacted.emit(kind)
	return kind


## A stroke across the body or head. Enough of it soothes them.
func pet(p: Vector2, moved: float) -> void:
	if region_at(p) == "":
		return
	if data.asleep:
		_pet_amount += moved
		if _pet_amount > 60.0:
			wake()
			_pet_amount = 0.0
		return
	_pet_amount += moved
	if _pet_amount > 110.0:
		set_expression("squint", "smile", 0.6)
		if _pet_sound <= 0.0:
			Events.friend_reacted.emit("purr")
			_fx("heart", head_center() + Vector2(0, -FriendArt.head_ry(data)), 1, 20.0)
			_pet_sound = 1.1


func end_pet() -> void:
	_pet_amount = 0.0


func feed_stuffing() -> void:
	data.stuffing = minf(1.45, data.stuffing + 0.18)
	_chew = 0.9
	_squash_vel -= 2.5
	Events.friend_reacted.emit("chomp")
	Events.cared.emit("fed_stuffing")
	Save.mark_dirty()


func feed_oil() -> void:
	data.fluid = 1.0
	_chew = 0.6
	Events.friend_reacted.emit("glug")
	Events.cared.emit("fed_oil")
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if data.part(slot):
			_fx("sparkle", socket(slot), 2, 10.0)
	set_expression("squint", "smile", 1.0)
	_wiggle = 0.6
	Save.mark_dirty()


## Rub with a sponge at a local point. True if it touched the friend.
func sponge(p: Vector2) -> bool:
	if not is_on_friend(p):
		return false
	var s := body_scale()
	for m in data.mud:
		var mp := Vector2(float(m["x"]) * (s if float(m["y"]) > -110.0 else 1.0), float(m["y"]))
		if p.distance_to(mp) < float(m["r"]) + 28.0 and float(m["a"]) > 0.0:
			m["a"] = maxf(0.0, float(m["a"]) - 0.05)
			if _rng.randf() < 0.3:
				_fx("mud", mp, 1, 6.0)
	data.mud = data.mud.filter(func(m): return float(m["a"]) > 0.02)
	data.wetness = minf(1.0, data.wetness + 0.012)
	if _rng.randf() < 0.5:
		_fx("bubble", p, 1, 16.0)
	Save.mark_dirty()
	return true


func towel(p: Vector2) -> bool:
	if not is_on_friend(p):
		return false
	data.wetness = maxf(0.0, data.wetness - 0.02)
	Save.mark_dirty()
	return true


# --- Tugging limbs ------------------------------------------------------

func grab_limb(slot: String) -> void:
	_grabbed = slot
	_strain = 0.0
	_winced = false
	_pull_time = 0.0
	_pull_counted = false


## Hard pulls a part needs before it comes off. A friend who has died,
## or a body without a head, gives its parts up at the first pull.
func pulls_needed(part: PartData) -> int:
	return 1 if data.dead or data.headless else part.pulls_needed()


## Pull a grabbed limb toward a local point. Healthy limbs stretch a
## little and protest; a rotting limb keeps stretching until it tears
## off, and then this returns the torn part.
func drag_limb(p: Vector2, delta: float) -> PartData:
	if _grabbed == "":
		return null
	var slot := _grabbed
	var part: PartData = data.part(slot)
	if part == null:
		return null
	var want := p - rest_tip(slot)
	var st: Dictionary = _limbs[slot]
	if part.rot < 0.5:
		var l := want.length()
		var cap := 62.0
		if l > cap:
			want = want.normalized() * (cap + (l - cap) * 0.12)
			_strain += delta
			# A hard pull, held: it counts once per grab. Enough of them
			# and even a healthy part comes off (decision 0025).
			_pull_time += delta
			if l > 120.0 and _pull_time > 0.3 and not _pull_counted:
				_pull_counted = true
				part.pulls += 1
				_fx("fluff", socket(slot), 3, 10.0)
				Voice.tear(0.15)
				Save.mark_dirty()
				if part.pulls >= pulls_needed(part):
					return tear(slot)
			if data.dead or data.headless:
				st["off"] = want
				return null
			if not _winced:
				_winced = true
				set_expression("squint", "frown", 0.8)
				Events.friend_reacted.emit("wince")
			elif _strain > 1.2:
				_strain = 0.0
				_shake = 0.8
				set_expression("open", "frown", 0.8)
				Events.friend_reacted.emit("dislike")
		st["off"] = want
		return null
	st["off"] = want
	var stretch := want.length()
	if stretch > 45.0:
		if not _winced:
			_winced = true
			Events.friend_reacted.emit("wince")
		set_expression("squint", "frown", 0.3)
		_strain_fx -= delta
		if _strain_fx <= 0.0:
			_strain_fx = 0.08
			_fx("fluff", socket(slot), 1, 12.0)
		_strain_sound -= delta
		if _strain_sound <= 0.0:
			_strain_sound = 0.28
			Voice.tear(0.15)
	if stretch > 150.0:
		return tear(slot)
	return null


func release_limb() -> void:
	_grabbed = ""


# --- Pulling the head (0026) ----------------------------------------------

func grab_head() -> void:
	_head_grabbed = true
	_pull_time = 0.0
	_pull_counted = false
	set_expression("open", "frown", 0.6)
	Events.friend_reacted.emit("wince")


## Pull the head toward a local point. The neck stretches; a hard pull
## held counts once per grab, and after about three the head comes off.
## True when it does.
func drag_head(p: Vector2, delta: float) -> bool:
	if not _head_grabbed:
		return false
	var want := p - HEAD_C
	var l := want.length()
	var cap := 46.0
	if l > cap:
		want = want.normalized() * (cap + (l - cap) * 0.1)
		_pull_time += delta
		if l > 130.0 and _pull_time > 0.3 and not _pull_counted:
			_pull_counted = true
			_neck_pulls += 1
			_fx("fluff", HEAD_C + Vector2(0, FriendArt.head_ry(data) * 0.8), 4, 16.0)
			Voice.tear(0.3)
			_shake = 0.6
			if _neck_pulls >= 3:
				_head_grabbed = false
				_head_pull = Vector2.ZERO
				return true
	_head_pull = want
	return false


func release_head() -> void:
	_head_grabbed = false
	_head_pull = Vector2.ZERO
	_pull_time = 0.0
	_pull_counted = false


func is_pulling_head() -> bool:
	return _head_grabbed


func is_grabbing() -> bool:
	return _grabbed != ""


## Rip a part off. Fluff bursts out and fluid spills; the slot becomes
## an open wound that slowly leaks stuffing until something is sewn on.
func tear(slot: String) -> PartData:
	var part: PartData = data.part(slot)
	data.parts[slot] = null
	wounds[slot] = make_tufts()
	data.stuffing = maxf(0.35, data.stuffing - 0.1)
	_limbs[slot] = {"off": Vector2.ZERO, "vel": Vector2.ZERO}
	_grabbed = ""
	var at := socket(slot)
	_fx("fluff", at, 18, 18.0)
	_fx("fluid", at, 10, 8.0)
	Voice.tear(1.0)
	_squash_vel += 4.0
	set_expression("open", "o", 1.0)
	Events.part_changed.emit(slot, "torn")
	Save.mark_dirty()
	return part


func make_tufts() -> Array:
	var t: Array = []
	for i in 6:
		var a := TAU * i / 6.0 + _rng.randf_range(-0.3, 0.3)
		t.append(Vector2(cos(a) * _rng.randf_range(10, 24), sin(a) * _rng.randf_range(4, 12)))
	return t


# --- The dangling eye ---------------------------------------------------

func grab_eye() -> void:
	_eye_grabbed = true


func drag_eye(p: Vector2) -> void:
	var anchor := eye_anchor("eye_r")
	var v := p - anchor
	if v.length() > 150.0:
		v = v.normalized() * 150.0
	_eye_pos = anchor + v
	_eye_vel = Vector2.ZERO


## True if the eye was let go close enough to its place to sew it on.
func release_eye() -> bool:
	_eye_grabbed = false
	return _eye_pos.distance_to(eye_anchor("eye_r")) < 34.0


## Mold spray: clears the mold off the part under `p`. Rot already in
## the fabric stays; it just has no mold feeding it. True if any came off.
func spray(p: Vector2) -> bool:
	var best := ""
	var best_d := 70.0
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		var part: PartData = data.part(slot)
		if part == null or part.mold.is_empty() or not part.attached:
			continue
		var q := Geometry2D.get_closest_point_to_segment(p, socket(slot), tip(slot))
		if q.distance_to(p) < best_d:
			best_d = q.distance_to(p)
			best = slot
	if best == "":
		return false
	var part: PartData = data.part(best)
	_fx("bubble", p, 4, 20.0)
	part.mold.clear()
	Save.mark_dirty()
	return true


## Hold the eye in its place while it is stitched on.
func snap_eye() -> void:
	_eye_pos = eye_anchor("eye_r")
	_eye_vel = Vector2.ZERO


## Give a limb a shove (a kick, a swat, a ball off it).
func nudge_limb(slot: String, impulse: Vector2) -> void:
	if _limbs.has(slot):
		_limbs[slot]["vel"] += impulse


## A ball bounced off the body: a soft "oof" and a wobble.
func bounce_off() -> void:
	_squash_vel += 3.2
	_wiggle = maxf(_wiggle, 0.5)
	set_expression("squint", "o", 0.5)
	Events.friend_reacted.emit("boop")


## Hit on the head by a ball it didn't catch. It laughs.
func bonk() -> void:
	_squash_vel += 2.6
	_shake = 0.3
	set_expression("squint", "open", 0.9)
	Events.friend_reacted.emit("bonk")


func eye_position() -> Vector2:
	return _eye_pos


# --- Simulation ---------------------------------------------------------

func _process(delta: float) -> void:
	if data == null:
		return
	_t += delta
	var wb := data.wellbeing()
	# Soft teddy-bear bounce: firm damping, no jelly jiggle.
	_squash_vel += (-220.0 * _squash - 24.0 * _squash_vel) * delta
	_squash += _squash_vel * delta
	_squash = clampf(_squash, -0.18, 0.18)
	_wiggle = maxf(0.0, _wiggle - delta * 1.2)
	_shake = maxf(0.0, _shake - delta * 1.5)
	_expr_timer -= delta
	_pet_sound -= delta
	_chew = maxf(0.0, _chew - delta)
	_beat += delta * lerpf(0.55, 1.25, wb) * TAU
	# Limb springs. Dry joints are stiffer (more damping).
	var damping := 7.0 + (1.0 - data.fluid) * 16.0
	for slot in _limbs:
		var st: Dictionary = _limbs[slot]
		if slot == _grabbed:
			continue
		var off: Vector2 = st["off"]
		var vel: Vector2 = st["vel"]
		var sway := Vector2(sin(_t * 1.7 + socket(slot).x) * 2.0, 0) if slot.begins_with("arm") else Vector2.ZERO
		vel += ((sway - off) * 90.0 - vel * damping) * delta
		off += vel * delta
		st["off"] = off
		st["vel"] = vel
	# Dangling eye swings on its thread.
	var eye: PartData = data.part("eye_r")
	if eye and not eye.attached and not _eye_grabbed and not surgery.has("eye"):
		var rest := eye_anchor("eye_r") + Vector2(0, 58)
		_eye_vel += ((rest - _eye_pos) * 30.0 - _eye_vel * 3.0) * delta
		_eye_pos += _eye_vel * delta
	elif eye and eye.attached:
		_eye_pos = eye_anchor("eye_r")
	# Lids follow the expression.
	_blink -= delta
	_next_blink -= delta
	if _next_blink <= 0.0:
		_blink = 0.14
		_next_blink = _rng.randf_range(2.5, 5.5)
	var target := _lid_target(wb)
	_lid = move_toward(_lid, target, delta * (2.0 if data.asleep else 8.0))
	# Open wounds leak stuffing; wet fabric drips; sleepers snore.
	if not wounds.is_empty() and surgery.is_empty() and not data.dead:
		_leak -= delta
		if _leak <= 0.0:
			_leak = 0.9
			var slot: String = wounds.keys()[_rng.randi() % wounds.size()]
			_fx("fluff", socket(slot) + Vector2(0, 8), 1, 6.0)
			# Hydraulic fluid drips from where the part tore (Mikey: the
			# leak should show).
			if not data.dead and not data.headless and slot != "neck":
				_fx("fluid", socket(slot) + Vector2(4, 12), 2, 4.0)
	if data.wetness > 0.12:
		_drip -= delta
		if _drip <= 0.0:
			_drip = lerpf(0.6, 0.12, data.wetness)
			var x := _rng.randf_range(-90, 90) * body_scale()
			_fx("drip", Vector2(x, 110), 1, 4.0)
	if data.asleep:
		_zzz -= delta
		if _zzz <= 0.0:
			_zzz = 1.5
			_fx("z", head_center() + Vector2(70, -90), 1, 2.0)
	_update_parts(wb)


func _lid_target(wb: float) -> float:
	if data.asleep or data.dead:
		return 1.0
	if _blink > 0.0:
		return 1.0
	if _expr_timer > 0.0:
		match _expr_eyes:
			"closed": return 1.0
			"squint": return 0.62
			"half": return 0.5
			"blink": return 0.9
			"open": return 0.0
	if _pet_amount > 110.0:
		return 0.62
	if wb < 0.45:
		return 0.5
	return 0.0


func _fx(kind: String, local: Vector2, count: int, spread: float) -> void:
	if fx:
		fx.spawn(kind, fx.to_local(to_global(local)), count, spread)
