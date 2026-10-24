class_name CreatureView
extends Node2D
## Draws a creature front-on (decision pending: front and back views
## only) and handles how its body reacts: poking, tickling, petting,
## tugging limbs until rotten ones tear, a button eye dangling on its
## thread, wounds leaking fluff, washing, drying and feeding.
## The body is the interface (pillar 1): health shows as shaking,
## droopy lids, a slow heart patch and a pale, wet or muddy coat.

const HEAD_C := Vector2(0, -200)
const HEAD_R := 100.0
const EYE_R := 21.0
const ARM_LEN := 100.0
const LEG_LEN := 100.0

var data: CreatureData
var fx: FxLayer

## Arms raised, as when catching a ball.
var arms_up := false
## Set while walking in the play corner; swings the legs.
var walk_phase := 0.0
## Surgery overlay, filled in by the tent. See Tent.start_surgery().
var surgery: Dictionary = {}
## Open wounds: slot -> Array of fluff tuft offsets around the socket.
var wounds: Dictionary = {}

var _t := 0.0
var _limbs: Dictionary = {}
var _grabbed := ""
var _strain := 0.0
var _strain_fx := 0.0
var _strain_sound := 0.0
var _winced := false
var _eye_pos := Vector2.ZERO
var _eye_vel := Vector2.ZERO
var _eye_grabbed := false
var _squash := 0.0
var _squash_vel := 0.0
var _wiggle := 0.0
var _shake := 0.0
var _lid := 1.0
var _expr_eyes := ""
var _expr_mouth := ""
var _expr_timer := 0.0
var _blink := 0.0
var _next_blink := 3.0
var _pet_amount := 0.0
var _pet_sound := 0.0
var _leak := 0.0
var _drip := 0.0
var _zzz := 0.0
var _chew := 0.0
var _beat := 0.0
var _rng := RandomNumberGenerator.new()


func setup(creature: CreatureData) -> void:
	data = creature
	_rng.seed = data.look_seed
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		_limbs[slot] = {"off": Vector2.ZERO, "vel": Vector2.ZERO}
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if data.is_open(slot) and not wounds.has(slot):
			wounds[slot] = make_tufts()
	_eye_pos = eye_anchor("eye_r") + Vector2(0, 58)
	_lid = 1.0 if data.asleep else 0.0
	queue_redraw()


# --- Geometry -----------------------------------------------------------

func body_scale() -> float:
	return clampf(0.75 + 0.25 * data.stuffing, 0.62, 1.12)


func head_offset() -> Vector2:
	return Vector2(sin(_t * 30.0) * 10.0 * _shake, 0)


func head_center() -> Vector2:
	return HEAD_C + head_offset()


func eye_anchor(slot: String) -> Vector2:
	var side := -1.0 if slot == "eye_l" else 1.0
	return head_center() + Vector2(side * 40.0, -12.0)


func mouth_pos() -> Vector2:
	return head_center() + Vector2(0, 50)


func socket(slot: String) -> Vector2:
	var s := body_scale()
	match slot:
		"arm_l": return Vector2(-98.0 * s, -62.0)
		"arm_r": return Vector2(98.0 * s, -62.0)
		"leg_l": return Vector2(-46.0 * s, 108.0)
		"leg_r": return Vector2(46.0 * s, 108.0)
	return Vector2.ZERO


func limb_dir(slot: String) -> Vector2:
	match slot:
		"arm_l": return Vector2(-0.55, -0.83).normalized() if arms_up else Vector2(-0.4, 0.92).normalized()
		"arm_r": return Vector2(0.55, -0.83).normalized() if arms_up else Vector2(0.4, 0.92).normalized()
		"leg_l": return Vector2(-0.06, 1.0).normalized()
		"leg_r": return Vector2(0.06, 1.0).normalized()
	return Vector2.DOWN


func limb_len(slot: String) -> float:
	return LEG_LEN if slot.begins_with("leg") else ARM_LEN


func limb_width(slot: String) -> float:
	var w := 50.0 if slot.begins_with("leg") else 42.0
	return w * (0.82 + 0.18 * body_scale())


func rest_tip(slot: String) -> Vector2:
	var tip := socket(slot) + limb_dir(slot) * limb_len(slot)
	if slot.begins_with("leg") and walk_phase != 0.0:
		var side := -1.0 if slot == "leg_l" else 1.0
		tip.y -= maxf(0.0, sin(walk_phase) * side) * 18.0
	return tip


func tip(slot: String) -> Vector2:
	return rest_tip(slot) + _limbs[slot]["off"]


## Where the feet touch the ground, in local units.
func feet_y() -> float:
	return 108.0 + LEG_LEN + 24.0


# --- Hit tests ----------------------------------------------------------

## The limb or dangling eye under a local point, or "".
func part_at(p: Vector2) -> String:
	var eye: PartData = data.part("eye_r")
	if eye and not eye.attached and p.distance_to(_eye_pos) < EYE_R + 16.0:
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
	if p.distance_to(hc) < HEAD_R + 6.0 or _in_ears(p):
		return "head_top" if p.y < hc.y - 52.0 else "head"
	var s := body_scale()
	var spot := Vector2(data.tickle_spot.x * s, data.tickle_spot.y)
	if p.distance_to(spot) < 42.0:
		return "tickle"
	var rx := 112.0 * s
	var ry := 132.0
	if pow(p.x / rx, 2) + pow(p.y / ry, 2) <= 1.05:
		return "body"
	return ""


func _in_ears(p: Vector2) -> bool:
	var hc := head_center()
	return p.distance_to(hc + Vector2(-72, -78)) < 34.0 or p.distance_to(hc + Vector2(72, -78)) < 34.0


func is_on_creature(p: Vector2) -> bool:
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
	Events.creature_reacted.emit("wake")
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
	Events.creature_reacted.emit(kind)
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
			Events.creature_reacted.emit("purr")
			_fx("heart", head_center() + Vector2(0, -HEAD_R), 1, 20.0)
			_pet_sound = 1.1


func end_pet() -> void:
	_pet_amount = 0.0


func feed_stuffing() -> void:
	data.stuffing = minf(1.45, data.stuffing + 0.18)
	_chew = 0.9
	_squash_vel -= 2.5
	Events.creature_reacted.emit("chomp")
	Events.cared.emit("fed_stuffing")
	Save.mark_dirty()


func feed_oil() -> void:
	data.fluid = 1.0
	_chew = 0.6
	Events.creature_reacted.emit("glug")
	Events.cared.emit("fed_oil")
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if data.part(slot):
			_fx("sparkle", socket(slot), 2, 10.0)
	set_expression("squint", "smile", 1.0)
	_wiggle = 0.6
	Save.mark_dirty()


## Rub with a sponge at a local point. True if it touched the creature.
func sponge(p: Vector2) -> bool:
	if not is_on_creature(p):
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
	if not is_on_creature(p):
		return false
	data.wetness = maxf(0.0, data.wetness - 0.02)
	Save.mark_dirty()
	return true


# --- Tugging limbs ------------------------------------------------------

func grab_limb(slot: String) -> void:
	_grabbed = slot
	_strain = 0.0
	_winced = false


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
			if not _winced:
				_winced = true
				set_expression("squint", "frown", 0.8)
				Events.creature_reacted.emit("wince")
			elif _strain > 1.2:
				_strain = 0.0
				_shake = 0.8
				set_expression("open", "frown", 0.8)
				Events.creature_reacted.emit("dislike")
		st["off"] = want
		return null
	st["off"] = want
	var stretch := want.length()
	if stretch > 45.0:
		if not _winced:
			_winced = true
			Events.creature_reacted.emit("wince")
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


## Hold the eye in its place while it is stitched on.
func snap_eye() -> void:
	_eye_pos = eye_anchor("eye_r")
	_eye_vel = Vector2.ZERO


## Hit on the head by a ball it didn't catch. It laughs.
func bonk() -> void:
	_squash_vel += 2.6
	_shake = 0.3
	set_expression("squint", "open", 0.9)
	Events.creature_reacted.emit("bonk")


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
	if not wounds.is_empty() and surgery.is_empty():
		_leak -= delta
		if _leak <= 0.0:
			_leak = 1.6
			var slot: String = wounds.keys()[0]
			_fx("fluff", socket(slot) + Vector2(0, 8), 1, 6.0)
			data.stuffing = maxf(0.35, data.stuffing - 0.006)
			Save.mark_dirty()
	if data.wetness > 0.12:
		_drip -= delta
		if _drip <= 0.0:
			_drip = lerpf(0.6, 0.12, data.wetness)
			var x := _rng.randf_range(-90, 90) * body_scale()
			_fx("drip", Vector2(x, 110), 1, 4.0)
		data.wetness = maxf(0.0, data.wetness - delta * 0.004)
	if data.asleep:
		_zzz -= delta
		if _zzz <= 0.0:
			_zzz = 1.5
			_fx("z", head_center() + Vector2(70, -90), 1, 2.0)
	queue_redraw()


func _lid_target(wb: float) -> float:
	if data.asleep:
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


func _mouth_kind(wb: float) -> String:
	if _chew > 0.0:
		return "o" if int(_chew * 10.0) % 2 == 0 else "flat"
	if data.asleep:
		return "flat"
	if _expr_timer > 0.0 and _expr_mouth != "":
		return _expr_mouth
	if wb < 0.45:
		return "wavy"
	if wb < 0.75:
		return "flat"
	return "smile"


# --- Drawing ------------------------------------------------------------

func _draw() -> void:
	if data == null:
		return
	var wb := data.wellbeing()
	var s := body_scale()
	var jitter := Vector2.ZERO
	var sick := maxf(0.0, 0.62 - wb)
	if sick > 0.0 and not data.asleep:
		jitter = Vector2(sin(_t * 47.0) * sick * 5.0, cos(_t * 39.0) * sick * 2.0)
	elif data.asleep and sick > 0.0:
		jitter = Vector2(sin(_t * 31.0) * sick * 2.5, 0)
	var breathe := 1.0 + sin(_t * (1.2 if data.asleep else 2.2)) * 0.012
	var tilt := sin(_t * 18.0) * 0.08 * _wiggle
	draw_set_transform(jitter + Vector2(0, _squash * feet_y()), tilt, Vector2(1.0 + _squash, (1.0 - _squash) * breathe))

	var pale := Color(0.6, 0.58, 0.54)
	var fabric := PlushArt.fabric_color(data.body_fabric.lerp(pale, (1.0 - wb) * 0.3), data.wetness)
	var trim := PlushArt.fabric_color(data.trim, data.wetness)

	# Floor shadow
	draw_set_transform(jitter, 0, Vector2.ONE)
	PlushArt.ellipse(self, Vector2(0, feet_y() + 4.0), 120.0 * s, 16.0, Color(0, 0, 0, 0.22), false)
	draw_set_transform(jitter + Vector2(0, _squash * feet_y()), tilt, Vector2(1.0 + _squash, (1.0 - _squash) * breathe))

	# Body
	var rx := 112.0 * s
	var ry := 132.0
	PlushArt.ellipse(self, Vector2.ZERO, rx, ry, fabric)
	PlushArt.ellipse(self, Vector2(0, 28), 68.0 * s, 80.0, trim, false)
	PlushArt.seam(self, PackedVector2Array([Vector2(0, -ry + 10), Vector2(0, -60)]), fabric.darkened(0.35))
	_draw_heart(Vector2(-42.0 * s, -52.0), wb)
	if data.wetness > 0.1:
		draw_arc(Vector2(30.0 * s, -40), 50.0, -1.2, -0.4, 10, Color(1, 1, 1, 0.35 * data.wetness), 4.0, true)
	_draw_mud(false, s)
	for slot in wounds:
		_draw_wound(slot)

	# Legs, then arms
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		var part: PartData = data.part(slot)
		if part and part.attached:
			PlushArt.limb(self, socket(slot), tip(slot), limb_width(slot), part, trim, data.wetness)

	# Head
	var hc := head_center()
	for side in [-1.0, 1.0]:
		var ear := hc + Vector2(72.0 * side, -78.0)
		PlushArt.ellipse(self, ear, 34.0, 34.0, fabric)
		PlushArt.ellipse(self, ear, 19.0, 19.0, trim, false)
	PlushArt.ellipse(self, hc, HEAD_R, HEAD_R * 0.94, fabric)
	PlushArt.seam(self, PackedVector2Array([hc + Vector2(0, -HEAD_R + 8), hc + Vector2(0, -30)]), fabric.darkened(0.35))
	PlushArt.ellipse(self, hc + Vector2(0, 38), 48.0, 33.0, trim, false)
	PlushArt.ellipse(self, hc + Vector2(0, 24), 12.0, 8.0, Color(0.22, 0.15, 0.13), false)
	_draw_mouth(_mouth_kind(wb))
	_draw_mud(true, s)

	# Eyes
	var happy := _lid > 0.5 and _lid < 0.8 and (_expr_eyes == "squint" or _pet_amount > 110.0)
	var eye_l: PartData = data.part("eye_l")
	if eye_l:
		PlushArt.button_eye(self, eye_anchor("eye_l"), EYE_R, eye_l.fabric, _lid, fabric, eye_l.stitched, happy)
	var eye_r: PartData = data.part("eye_r")
	if eye_r:
		if eye_r.attached:
			PlushArt.button_eye(self, eye_anchor("eye_r"), EYE_R, eye_r.fabric, _lid, fabric, eye_r.stitched, happy)
		else:
			var anchor := eye_anchor("eye_r")
			draw_circle(anchor, 9.0, Color(0.12, 0.08, 0.07))
			var mid := anchor.lerp(_eye_pos, 0.5) + Vector2(6, 10)
			draw_polyline(PackedVector2Array([anchor, mid, _eye_pos + Vector2(0, -EYE_R)]), PlushArt.THREAD, 2.5, true)
			PlushArt.button_eye(self, _eye_pos, EYE_R, eye_r.fabric, 0.0, fabric)

	if not surgery.is_empty():
		_draw_surgery(trim)
	draw_set_transform(Vector2.ZERO, 0, Vector2.ONE)


func _draw_heart(p: Vector2, wb: float) -> void:
	var pulse := 1.0 + maxf(0.0, sin(_beat)) * 0.12
	var r := 11.0 * pulse
	var c := Color(0.78, 0.3, 0.33).lerp(Color(0.5, 0.42, 0.42), (1.0 - wb) * 0.6)
	draw_circle(p + Vector2(-r * 0.5, 0), r * 0.62, c)
	draw_circle(p + Vector2(r * 0.5, 0), r * 0.62, c)
	draw_colored_polygon(PackedVector2Array([p + Vector2(-r * 1.08, r * 0.2), p + Vector2(r * 1.08, r * 0.2), p + Vector2(0, r * 1.3)]), c)
	PlushArt.seam(self, PackedVector2Array([p + Vector2(-r * 1.2, r * 0.1), p + Vector2(0, r * 1.45), p + Vector2(r * 1.2, r * 0.1)]), c.lightened(0.4), 4.0, 1.5)


func _draw_mouth(kind: String) -> void:
	var m := mouth_pos()
	var col := Color(0.3, 0.17, 0.14)
	match kind:
		"smile":
			draw_arc(m + Vector2(0, -10), 16.0, PI * 0.18, PI * 0.82, 14, col, 3.5, true)
		"frown":
			draw_arc(m + Vector2(0, 12), 15.0, PI * 1.18, PI * 1.82, 14, col, 3.5, true)
		"o":
			PlushArt.ellipse(self, m, 8.0, 9.0, col, false)
		"open":
			PlushArt.ellipse(self, m, 14.0, 11.0, col, false)
			PlushArt.ellipse(self, m + Vector2(0, 5), 7.0, 4.0, Color(0.85, 0.45, 0.45), false)
		"wavy":
			var pts := PackedVector2Array()
			for i in 9:
				pts.append(m + Vector2(-16 + i * 4, sin(i * 1.4 + _t * 3.0) * 2.5))
			draw_polyline(pts, col, 3.0, true)
		_:
			draw_line(m + Vector2(-12, 0), m + Vector2(12, 0), col, 3.5, true)


func _draw_mud(on_head: bool, s: float) -> void:
	for m in data.mud:
		var y := float(m["y"])
		if (y < -110.0) != on_head:
			continue
		var p := Vector2(float(m["x"]) * (1.0 if on_head else s), y)
		var r := float(m["r"])
		var a := float(m["a"])
		var c := Color(0.36, 0.26, 0.16, 0.85 * a)
		draw_circle(p, r * 0.8, c)
		draw_circle(p + Vector2(r * 0.5, r * 0.3), r * 0.5, c)
		draw_circle(p + Vector2(-r * 0.4, r * 0.35), r * 0.45, c)
		draw_circle(p + Vector2(r * 0.1, -r * 0.5), r * 0.35, c)


func _draw_wound(slot: String) -> void:
	var at := socket(slot)
	var hole := Color(0.14, 0.09, 0.07)
	PlushArt.ellipse(self, at, 27.0, 15.0, hole, false)
	var rim := PackedVector2Array()
	for i in 13:
		var a := TAU * i / 12.0
		var r := 27.0 if i % 2 == 0 else 22.0
		rim.append(at + Vector2(cos(a) * r, sin(a) * r * 0.58))
	draw_polyline(rim, data.body_fabric.darkened(0.3), 3.0, true)
	draw_polyline(PackedVector2Array([at + Vector2(-6, 2), at + Vector2(-14, 18), at + Vector2(-8, 26)]), Color(0.85, 0.2, 0.2), 2.5, true)
	draw_polyline(PackedVector2Array([at + Vector2(5, 3), at + Vector2(16, 14), at + Vector2(22, 12)]), Color(0.95, 0.8, 0.2), 2.5, true)
	var fluid := PlushArt.FLUID
	fluid.a = 0.6
	draw_circle(at + Vector2(3, 6), 6.0, fluid)
	var tufts: Array = wounds[slot]
	for tuft in tufts:
		PlushArt.fluff(self, at + tuft, 9.0)


## The surgery overlay: the new part waiting below the wound, the
## ligament tube, the spill, the stitch points and the needle.
func _draw_surgery(trim: Color) -> void:
	var slot: String = surgery.get("slot", "")
	if surgery.has("part"):
		var part: PartData = surgery["part"]
		var gap := Vector2(0, 16)
		PlushArt.limb(self, socket(slot) + gap, rest_tip(slot) + gap, limb_width(slot), part, trim, 0.0)
		var top := socket(slot) + gap + Vector2(10, 6)
		var port := socket(slot) + Vector2(10, 0)
		draw_arc(port, 6.0, 0, TAU, 12, Color(0.85, 0.85, 0.9), 2.5, true)
		var end: Vector2 = surgery.get("tube_end", top + Vector2(30, 40))
		var mid := top.lerp(end, 0.5) + Vector2(14, 10)
		var tube := PackedVector2Array([top, mid, end])
		draw_polyline(tube, Color(0.1, 0.35, 0.33), 9.0, true)
		var inner := PlushArt.FLUID
		inner.a = 1.0 if surgery.get("connected", false) else 0.45
		draw_polyline(tube, inner, 5.0, true)
		draw_circle(end, 5.5, Color(0.75, 0.78, 0.82))
		if surgery.get("connected", false):
			var k := fmod(_t * 1.5, 1.0)
			draw_circle(top.lerp(end, k), 3.0, Color(1, 1, 1, 0.8))
	if surgery.has("spill") and float(surgery["spill"]) > 0.0:
		var sp: Vector2 = surgery["spill_pos"]
		var c := PlushArt.FLUID
		c.a = 0.75 * float(surgery["spill"])
		PlushArt.ellipse(self, sp, 32.0, 14.0, c, false)
		PlushArt.ellipse(self, sp + Vector2(-20, 8), 14.0, 7.0, c, false)
	var points: Array = surgery.get("points", [])
	var done: int = surgery.get("done", 0)
	for i in points.size():
		var p: Vector2 = points[i]
		if i < done:
			if i > 0:
				draw_line(points[i - 1], p, PlushArt.THREAD, 3.5, true)
			draw_circle(p, 2.5, PlushArt.THREAD.darkened(0.3))
		elif i == done:
			var pulse := 9.0 + sin(_t * 6.0) * 2.5
			draw_arc(p, pulse, 0, TAU, 16, Color(1, 0.95, 0.7, 0.95), 2.5, true)
			draw_circle(p, 3.0, Color(1, 0.95, 0.7))
		else:
			draw_circle(p, 3.0, Color(1, 1, 1, 0.45))
	if surgery.get("needle") != null and points.size() > 0:
		var n: Vector2 = surgery["needle"]
		if done > 0:
			draw_line(points[done - 1], n, PlushArt.THREAD, 2.0, true)
		draw_line(n, n + Vector2(18, -18), Color(0.85, 0.87, 0.9), 3.0, true)
		draw_circle(n + Vector2(17, -17), 2.0, Color(0.3, 0.3, 0.35))
	if surgery.get("cloth") != null:
		var cpos: Vector2 = surgery["cloth"]
		var sq := PackedVector2Array([cpos + Vector2(-22, -16), cpos + Vector2(20, -20), cpos + Vector2(24, 16), cpos + Vector2(-18, 20)])
		draw_colored_polygon(sq, Color(0.93, 0.93, 0.98))
		draw_polyline(sq, Color(0.6, 0.62, 0.75), 2.0, true)


func _fx(kind: String, local: Vector2, count: int, spread: float) -> void:
	if fx:
		fx.spawn(kind, fx.to_local(to_global(local)), count, spread)
