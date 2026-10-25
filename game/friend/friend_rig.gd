class_name FriendRig
extends Node2D
## How a friend is drawn, front or back, and where its parts are: the
## part nodes, their transforms and painters, and the geometry the rest
## of the friend shares. FriendView (which extends this) makes it react.

const HEAD_C := Vector2(0, -200)
## The head's size for things outside the view that need a rough figure
## (the play corner's ball); the real size comes from the look.
const HEAD_R := 100.0
const ARM_LEN := 100.0
const LEG_LEN := 100.0

var data: FriendData

## "front" or "back" (decision 0031: front and back only).
var facing := "front"
## Arms raised, as when catching a ball.
var arms_up := false
## Set while walking in the play corner; swings the legs.
var walk_phase := 0.0
## Surgery overlay, filled in by the tent. See TentSurgery.
var surgery: Dictionary = {}
## Open wounds: slot -> Array of fluff tuft offsets around the socket.
var wounds: Dictionary = {}
## Texture pixels per unit for the baked parts: 2 for the friend up
## close, 1 for friends sitting further back.
var bake_detail := 2.0

# The pose and expression the drawing shows; FriendView moves them.
var _t := 0.0
var _limbs: Dictionary = {}
var _eye_pos := Vector2.ZERO
var _squash := 0.0
var _wiggle := 0.0
var _shake := 0.0
var _lid := 1.0
var _expr_eyes := ""
var _expr_mouth := ""
var _expr_timer := 0.0
var _pet_amount := 0.0
var _chew := 0.0
var _beat := 0.0
## How far the head is stretched from the neck (decision 0026).
var _head_pull := Vector2.ZERO


# --- Geometry -----------------------------------------------------------

func body_scale() -> float:
	return FriendArt.body_scale(data)


func head_offset() -> Vector2:
	return Vector2(sin(_t * 30.0) * 10.0 * _shake, 0) + _head_pull


func head_center() -> Vector2:
	return HEAD_C + head_offset()


func eye_anchor(slot: String) -> Vector2:
	var side := -1.0 if slot == "eye_l" else 1.0
	return head_center() + FriendArt.eye_pos(data, side)


func eye_radius() -> float:
	return FriendArt.eye_radius(data)


func mouth_pos() -> Vector2:
	return head_center() + FriendArt.mouth_pos(data)


## Body width and height from the look (0037).
func body_w() -> float:
	return float(data.bl("w")) if data else 1.0


func body_h() -> float:
	return float(data.bl("h")) if data else 1.0


func socket(slot: String) -> Vector2:
	return FriendArt.socket(data, slot)


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
	return FriendArt.limb_width(data, slot)


func rest_tip(slot: String) -> Vector2:
	var tip := socket(slot) + limb_dir(slot) * limb_len(slot)
	# A part loosened by hard pulls hangs a little lower (0025).
	var part: PartData = data.part(slot) if data else null
	if part:
		tip += limb_dir(slot) * part.pulls * 5.0
	if slot.begins_with("leg") and walk_phase != 0.0:
		var side := -1.0 if slot == "leg_l" else 1.0
		tip.y -= maxf(0.0, sin(walk_phase) * side) * 18.0
	return tip


func tip(slot: String) -> Vector2:
	return rest_tip(slot) + _limbs[slot]["off"]


## Where the feet touch the ground, in local units.
func feet_y() -> float:
	return 108.0 * body_h() + LEG_LEN + 24.0


## Which mouth the face shows now.
func _mouth_kind(wb: float) -> String:
	if _chew > 0.0:
		return "o" if int(_chew * 10.0) % 2 == 0 else "flat"
	if data.asleep or data.dead:
		return "flat"
	if _expr_timer > 0.0 and _expr_mouth != "":
		return _expr_mouth
	if wb < 0.45:
		return "wavy"
	if wb < 0.75:
		return "flat"
	return "smile"


# --- Drawing ------------------------------------------------------------
#
# Drawn once, moved cheaply (the Driftloom way: an idle screen paints
# nothing). The friend is a set of part nodes. Each draws itself only
# when something it shows has changed (its key); breathing, squash, sway,
# head shakes and stretched limbs are node transforms, which cost almost
# nothing per frame.

class Part:
	extends Node2D
	var painter: Callable
	var key = null

	func _draw() -> void:
		painter.call(self)

	## Redraw only if what this part shows has changed.
	func refresh(new_key) -> void:
		if new_key != key:
			key = new_key
			queue_redraw()


var _shadow: Baked
var _rig: Node2D
var _body: Baked
var _heart: Baked
var _limb_nodes: Dictionary = {}
var _head: Node2D
var _head_art: Baked
## The ears, -1 (left) and 1 (right): their own pictures, so they can
## swing (thin, long ears droop and swing; 0037).
var _ear_nodes: Dictionary = {}
var _face: Baked
var _dangle: Part
var _surgery_art: Part


## Bake every part again at another sharpness: more on the surgery
## table, where the camera comes in close (decision 0031).
func set_detail(d: float) -> void:
	bake_detail = d
	# Lying on the table (more detail), the floor shadow would show.
	_shadow.visible = d <= 2.0
	for b in [_shadow, _body, _heart, _head_art, _face, _ear_nodes[-1.0], _ear_nodes[1.0]]:
		b.set_resolution(d)
	for slot in _limb_nodes:
		_limb_nodes[slot].set_resolution(d)


## A part that changes every frame while shown: drawn directly.
func _part(parent: Node, painter: Callable) -> Part:
	var n := Part.new()
	n.painter = painter
	parent.add_child(n)
	return n


## A part that rarely changes: baked into a texture (one draw call).
## `rect` must hold everything its painter draws.
func _baked(parent: Node, rect: Rect2, painter: Callable) -> Baked:
	var n := Baked.new()
	parent.add_child(n)
	return n.setup(rect, painter, bake_detail)


func _build_parts() -> void:
	if _rig:
		return
	_shadow = _baked(self, Rect2(-170, 200, 340, 70), _paint_shadow)
	_rig = Node2D.new()
	add_child(_rig)
	_body = _baked(_rig, Rect2(-180, -180, 360, 380), _paint_body)
	_heart = _baked(_rig, Rect2(-16, -12, 32, 30), _paint_heart)
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		_limb_nodes[slot] = _baked(_rig, Rect2(-40, -40, 80, 185), _paint_limb.bind(slot))
	_head = Node2D.new()
	_rig.add_child(_head)
	for side in [-1.0, 1.0]:
		_ear_nodes[side] = _baked(_head, Rect2(-44, -224, 88, 236), _paint_ear)
	_head_art = _baked(_head, Rect2(-135, -130, 270, 270), _paint_head)
	_face = _baked(_head, Rect2(-95, -64, 190, 160), _paint_face)
	_dangle = _part(_rig, _paint_dangle)
	_surgery_art = _part(_rig, _paint_surgery)


func _q(v: float, step: float) -> int:
	return int(round(v / step))


## Called every frame: set transforms, and redraw only parts whose look changed.
func _update_parts(wb: float) -> void:
	var s := body_scale()
	var sick := maxf(0.0, 0.62 - wb)
	var jitter := Vector2.ZERO
	if sick > 0.0 and not data.asleep:
		jitter = Vector2(sin(_t * 47.0) * sick * 5.0, cos(_t * 39.0) * sick * 2.0)
	elif data.asleep and sick > 0.0:
		jitter = Vector2(sin(_t * 31.0) * sick * 2.5, 0)
	var breathe := 1.0 + sin(_t * (1.2 if data.asleep else 2.2)) * 0.012
	if data.dead:
		# Still, slumped to one side.
		jitter = Vector2.ZERO
		breathe = 1.0
	# Without legs a friend sits on the floor; with one, it leans (0025).
	var legs := 0
	var lean := 0.0
	for slot in ["leg_l", "leg_r"]:
		var leg: PartData = data.part(slot)
		if leg != null and leg.attached:
			legs += 1
		else:
			lean += -0.07 if slot == "leg_l" else 0.07
	var sit := Vector2(0, LEG_LEN * 0.85) if legs == 0 else Vector2.ZERO
	if legs == 0:
		lean = 0.0
	# Build shows in how they move (0037): a thin friend wobbles, a plump
	# one bounces.
	var wobble := 0.0
	var bounce := 1.0
	if not data.dead and not data.asleep:
		wobble = sin(_t * 2.6) * 0.05 * clampf((0.78 - data.stuffing) / 0.4, 0.0, 1.0)
		bounce = 1.0 + absf(sin(_t * 3.2)) * 0.02 * clampf((data.stuffing - 1.02) / 0.4, 0.0, 1.0)
	_rig.position = jitter + Vector2(0, _squash * feet_y()) + (Vector2(6, 22) if data.dead else Vector2.ZERO) + sit
	_rig.rotation = sin(_t * 18.0) * 0.08 * _wiggle + (0.12 if data.dead else 0.0) + lean + wobble
	_rig.scale = Vector2(1.0 + _squash, (1.0 - _squash) * breathe * bounce)
	_shadow.position = jitter
	_shadow.refresh([_q(s, 0.02)])

	var wq := _q(wb, 0.05)
	var wet := _q(data.wetness, 0.05)
	var mud_body := 0
	var mud_head := 0
	for m in data.mud:
		var a := _q(float(m["a"]), 0.1)
		if float(m["y"]) < -110.0:
			mud_head += a + 1000
		else:
			mud_body += a + 1000
	var wound_key: Array = []
	for slot in wounds:
		wound_key.append(slot)
		wound_key.append((wounds[slot] as Array).size())
	var back := facing == "back"
	_body.refresh([_q(s, 0.02), wq, wet, mud_body, wound_key, back, data.body_look.hash(), data.body_fabric])

	_heart.position = Vector2(-42.0 * s, -52.0)
	var pulse := 1.0 + maxf(0.0, sin(_beat)) * 0.12
	_heart.scale = Vector2(pulse, pulse)
	_heart.visible = not back
	_heart.refresh([wq])

	for slot in _limb_nodes:
		var node: Baked = _limb_nodes[slot]
		var part: PartData = data.part(slot)
		node.visible = part != null and part.attached
		if not node.visible:
			continue
		var a := socket(slot)
		var d := tip(slot) - a
		node.position = a
		node.rotation = d.angle() - PI * 0.5
		node.scale = Vector2(1.0, maxf(0.2, d.length() / limb_len(slot)))
		node.refresh([part.get_instance_id(), _q(part.rot, 0.05), part.mold.size(), part.torn, part.stitched, wet, _q(limb_width(slot), 0.5)])

	_head.visible = not data.headless
	_head.position = head_center()
	_head_art.refresh([wq, wet, mud_head, data.head_color(), back, data.head_look.hash()])
	# Ears swing: long and thin ones most, and with every shake of the head.
	var ears := str(data.hl("ears"))
	var swing := 0.07 if ears in ["long", "floppy", "antennae"] else 0.015
	if float(data.hl("ear_size")) < 1.0:
		swing *= 1.5
	if data.dead:
		swing = 0.0
	for side in _ear_nodes:
		var ear: Baked = _ear_nodes[side]
		ear.visible = ears != "none"
		if not ear.visible:
			continue
		ear.position = FriendArt.ear_base(data, side)
		ear.rotation = FriendArt.ear_angle(data, side) + sin(_t * 1.9 + side) * swing \
			+ sin(_t * 30.0) * _shake * swing * 4.0 + _wiggle * sin(_t * 14.0) * swing * 3.0
		ear.refresh([wq, wet, back, data.head_look.hash(), data.head_color()])
	var mouth := _mouth_kind(wb)
	var happy := _lid > 0.5 and _lid < 0.8 and (_expr_eyes == "squint" or _pet_amount > 110.0)
	var eye_l: PartData = data.part("eye_l")
	var eye_r: PartData = data.part("eye_r")
	_face.visible = not back
	_face.refresh([data.head_look.hash(), _q(_lid, 0.04), mouth, happy, wq, wet,
		eye_l != null and eye_l.stitched, eye_r != null and eye_r.attached, eye_r != null and eye_r.stitched,
		int(_t * 6.0) if mouth == "wavy" else 0])

	_dangle.visible = eye_r != null and not eye_r.attached
	if _dangle.visible:
		_dangle.refresh([_q(_eye_pos.x, 0.5), _q(_eye_pos.y, 0.5), _q(head_center().x, 0.5), wq, wet])

	_surgery_art.visible = not surgery.is_empty()
	if _surgery_art.visible:
		_surgery_art.queue_redraw()


func _fabric(wb: float) -> Color:
	var pale := Color(0.6, 0.58, 0.54)
	var fade := 0.6 if data.dead else (1.0 - wb) * 0.3
	return PlushArt.fabric_color(data.body_fabric.lerp(pale, fade), data.wetness)


func _head_fabric(wb: float) -> Color:
	var pale := Color(0.6, 0.58, 0.54)
	var fade := 0.6 if data.dead else (1.0 - wb) * 0.3
	return PlushArt.fabric_color(data.head_color().lerp(pale, fade), data.wetness)


func _trim() -> Color:
	return PlushArt.fabric_color(data.trim, data.wetness)


func _paint_shadow(ci: CanvasItem) -> void:
	PlushArt.ellipse(ci, Vector2(0, feet_y() + 4.0), 120.0 * body_scale() * body_w(), 16.0, Color(0, 0, 0, 0.22), false)


func _paint_body(ci: CanvasItem) -> void:
	var wb := data.wellbeing()
	var s := body_scale()
	var fabric := _fabric(wb)
	FriendArt.draw_body(ci, data, s, fabric, _trim(), facing == "back")
	if data.wetness > 0.1:
		ci.draw_arc(Vector2(30.0 * s, -40), 50.0, -1.2, -0.4, 10, Color(1, 1, 1, 0.35 * data.wetness), 4.0, true)
	_paint_mud(ci, false, s, Vector2.ZERO)
	for slot in wounds:
		_paint_wound(ci, slot)


func _paint_heart(ci: CanvasItem) -> void:
	var wb := data.wellbeing()
	var r := 11.0
	var c := Color(0.78, 0.3, 0.33).lerp(Color(0.5, 0.42, 0.42), (1.0 - wb) * 0.6)
	ci.draw_circle(Vector2(-r * 0.5, 0), r * 0.62, c)
	ci.draw_circle(Vector2(r * 0.5, 0), r * 0.62, c)
	ci.draw_colored_polygon(PackedVector2Array([Vector2(-r * 1.08, r * 0.2), Vector2(r * 1.08, r * 0.2), Vector2(0, r * 1.3)]), c)
	PlushArt.seam(ci, PackedVector2Array([Vector2(-r * 1.2, r * 0.1), Vector2(0, r * 1.45), Vector2(r * 1.2, r * 0.1)]), c.lightened(0.4), 4.0, 1.5)


## A limb drawn straight down from its socket; the node's transform
## points and stretches it.
func _paint_limb(ci: CanvasItem, slot: String) -> void:
	var part: PartData = data.part(slot)
	if part == null:
		return
	PlushArt.limb(ci, Vector2.ZERO, Vector2(0, limb_len(slot)), limb_width(slot), part, _trim(), data.wetness)


## The head, centered on its node.
func _paint_head(ci: CanvasItem) -> void:
	FriendArt.draw_head(ci, data, Vector2.ZERO, _head_fabric(data.wellbeing()), _trim(), facing == "back")
	_paint_mud(ci, true, 1.0, -HEAD_C)


## One ear, from its own base (the node sits where it joins the head).
func _paint_ear(ci: CanvasItem) -> void:
	FriendArt.draw_ear(ci, data, _head_fabric(data.wellbeing()), _trim(), facing == "back")


## Eyes and mouth, in the head's coordinates.
func _paint_face(ci: CanvasItem) -> void:
	var wb := data.wellbeing()
	var fabric := _head_fabric(wb)
	_paint_mouth(ci, _mouth_kind(wb), FriendArt.mouth_pos(data))
	var happy := _lid > 0.5 and _lid < 0.8 and (_expr_eyes == "squint" or _pet_amount > 110.0)
	var r := eye_radius()
	var eye_l: PartData = data.part("eye_l")
	if eye_l:
		PlushArt.button_eye(ci, FriendArt.eye_pos(data, -1.0), r, eye_l.fabric, _lid, fabric, eye_l.stitched, happy)
	var eye_r: PartData = data.part("eye_r")
	if eye_r:
		if eye_r.attached:
			PlushArt.button_eye(ci, FriendArt.eye_pos(data, 1.0), r, eye_r.fabric, _lid, fabric, eye_r.stitched, happy)
		else:
			ci.draw_circle(FriendArt.eye_pos(data, 1.0), 9.0, Color(0.12, 0.08, 0.07))


## The eye hanging on its thread, in the friend's coordinates.
func _paint_dangle(ci: CanvasItem) -> void:
	var eye_r: PartData = data.part("eye_r")
	if eye_r == null:
		return
	var anchor := eye_anchor("eye_r")
	var mid := anchor.lerp(_eye_pos, 0.5) + Vector2(6, 10)
	ci.draw_polyline(PackedVector2Array([anchor, mid, _eye_pos + Vector2(0, -eye_radius())]), PlushArt.THREAD, 2.5, true)
	PlushArt.button_eye(ci, _eye_pos, eye_radius(), eye_r.fabric, 0.0, _head_fabric(data.wellbeing()))


func _paint_mouth(ci: CanvasItem, kind: String, m: Vector2) -> void:
	var col := Color(0.3, 0.17, 0.14)
	match kind:
		"smile":
			ci.draw_arc(m + Vector2(0, -10), 16.0, PI * 0.18, PI * 0.82, 14, col, 3.5, true)
		"frown":
			ci.draw_arc(m + Vector2(0, 12), 15.0, PI * 1.18, PI * 1.82, 14, col, 3.5, true)
		"o":
			PlushArt.ellipse(ci, m, 8.0, 9.0, col, false)
		"open":
			PlushArt.ellipse(ci, m, 14.0, 11.0, col, false)
			PlushArt.ellipse(ci, m + Vector2(0, 5), 7.0, 4.0, Color(0.85, 0.45, 0.45), false)
		"wavy":
			var pts := PackedVector2Array()
			for i in 9:
				pts.append(m + Vector2(-16 + i * 4, sin(i * 1.4 + _t * 3.0) * 2.5))
			ci.draw_polyline(pts, col, 3.0, true)
		_:
			ci.draw_line(m + Vector2(-12, 0), m + Vector2(12, 0), col, 3.5, true)


## Mud splotches; `origin` moves them into the drawing node's coordinates.
func _paint_mud(ci: CanvasItem, on_head: bool, s: float, origin: Vector2) -> void:
	for m in data.mud:
		var y := float(m["y"])
		if (y < -110.0) != on_head:
			continue
		var p := Vector2(float(m["x"]) * (1.0 if on_head else s), y) + origin
		var r := float(m["r"])
		var c := Color(0.36, 0.26, 0.16, 0.85 * float(m["a"]))
		ci.draw_circle(p, r * 0.8, c)
		ci.draw_circle(p + Vector2(r * 0.5, r * 0.3), r * 0.5, c)
		ci.draw_circle(p + Vector2(-r * 0.4, r * 0.35), r * 0.45, c)
		ci.draw_circle(p + Vector2(r * 0.1, -r * 0.5), r * 0.35, c)


func _paint_wound(ci: CanvasItem, slot: String) -> void:
	var at := socket(slot)
	PlushArt.ellipse(ci, at, 27.0, 15.0, Color(0.14, 0.09, 0.07), false)
	var rim := PackedVector2Array()
	for i in 13:
		var a := TAU * i / 12.0
		var r := 27.0 if i % 2 == 0 else 22.0
		rim.append(at + Vector2(cos(a) * r, sin(a) * r * 0.58))
	ci.draw_polyline(rim, data.body_fabric.darkened(0.3), 3.0, true)
	ci.draw_polyline(PackedVector2Array([at + Vector2(-6, 2), at + Vector2(-14, 18), at + Vector2(-8, 26)]), Color(0.85, 0.2, 0.2), 2.5, true)
	ci.draw_polyline(PackedVector2Array([at + Vector2(5, 3), at + Vector2(16, 14), at + Vector2(22, 12)]), Color(0.95, 0.8, 0.2), 2.5, true)
	var fluid := PlushArt.FLUID
	fluid.a = 0.6
	ci.draw_circle(at + Vector2(3, 6), 6.0, fluid)
	for tuft in wounds[slot]:
		PlushArt.fluff(ci, at + tuft, 9.0)


## The surgery overlay: the new part waiting below the wound, the
## ligament tube, the spill, the stitch points and the needle. Redrawn
## every frame, but only while a surgery is on.
func _paint_surgery(ci: CanvasItem) -> void:
	if surgery.is_empty():
		return
	var slot: String = surgery.get("slot", "")
	if surgery.has("part"):
		var part: PartData = surgery["part"]
		var gap := Vector2(0, 16)
		PlushArt.limb(ci, socket(slot) + gap, rest_tip(slot) + gap, limb_width(slot), part, _trim(), 0.0)
		var top := socket(slot) + gap + Vector2(10, 6)
		var port := socket(slot) + Vector2(10, 0)
		ci.draw_arc(port, 6.0, 0, TAU, 12, Color(0.85, 0.85, 0.9), 2.5, true)
		var end: Vector2 = surgery.get("tube_end", top + Vector2(30, 40))
		var mid := top.lerp(end, 0.5) + Vector2(14, 10)
		var tube := PackedVector2Array([top, mid, end])
		ci.draw_polyline(tube, Color(0.1, 0.35, 0.33), 9.0, true)
		var inner := PlushArt.FLUID
		inner.a = 1.0 if surgery.get("connected", false) else 0.45
		ci.draw_polyline(tube, inner, 5.0, true)
		ci.draw_circle(end, 5.5, Color(0.75, 0.78, 0.82))
		if surgery.get("connected", false):
			var k := fmod(_t * 1.5, 1.0)
			ci.draw_circle(top.lerp(end, k), 3.0, Color(1, 1, 1, 0.8))
	if surgery.has("spill") and float(surgery["spill"]) > 0.0:
		var sp: Vector2 = surgery["spill_pos"]
		var c := PlushArt.FLUID
		c.a = 0.75 * float(surgery["spill"])
		PlushArt.ellipse(ci, sp, 32.0, 14.0, c, false)
		PlushArt.ellipse(ci, sp + Vector2(-20, 8), 14.0, 7.0, c, false)
	var points: Array = surgery.get("points", [])
	var done: int = surgery.get("done", 0)
	for i in points.size():
		var p: Vector2 = points[i]
		if i < done:
			if i > 0:
				ci.draw_line(points[i - 1], p, PlushArt.THREAD, 3.5, true)
			ci.draw_circle(p, 2.5, PlushArt.THREAD.darkened(0.3))
		elif i == done:
			var pulse := 9.0 + sin(_t * 6.0) * 2.5
			ci.draw_arc(p, pulse, 0, TAU, 16, Color(1, 0.95, 0.7, 0.95), 2.5, true)
			ci.draw_circle(p, 3.0, Color(1, 0.95, 0.7))
		else:
			ci.draw_circle(p, 3.0, Color(1, 1, 1, 0.45))
	if surgery.get("needle") != null and points.size() > 0:
		var n: Vector2 = surgery["needle"]
		if done > 0:
			ci.draw_line(points[done - 1], n, PlushArt.THREAD, 2.0, true)
		ci.draw_line(n, n + Vector2(18, -18), Color(0.85, 0.87, 0.9), 3.0, true)
		ci.draw_circle(n + Vector2(17, -17), 2.0, Color(0.3, 0.3, 0.35))
	if surgery.get("cloth") != null:
		var cpos: Vector2 = surgery["cloth"]
		var sq := PackedVector2Array([cpos + Vector2(-22, -16), cpos + Vector2(20, -20), cpos + Vector2(24, 16), cpos + Vector2(-18, 20)])
		ci.draw_colored_polygon(sq, Color(0.93, 0.93, 0.98))
		ci.draw_polyline(sq, Color(0.6, 0.62, 0.75), 2.0, true)
