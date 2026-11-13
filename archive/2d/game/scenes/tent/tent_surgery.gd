class_name TentSurgery
extends RefCounted
## Sewing in the tent, on the surgery table (decision 0031): a part
## onto a wound, a hole sewn shut, the dangling eye back in place, a
## head onto a body. The friend stays upright; the camera comes close.

var t: Tent

var _tube_drag := false


## Sew a head onto the body without one in front: the friend lives on
## with this body (0026). Takes a length of thread.
func start_head(head: FriendData) -> void:
	if not Save.camp.use("thread"):
		t.hud.shake("thread")
		Voice.thud()
		if not head in Save.camp.heads:
			Save.camp.heads.append(head)
		t.parts.show_spare()
		return
	Save.camp.heads.erase(head)
	var body := Save.friend
	head.join_body(body)
	Save.camp.friends[Save.camp.current] = head
	t.friend.setup(head)
	t.friend.wounds.erase("neck")
	t.mode = "surgery"
	var neck := t.friend.socket("neck")
	t.friend.surgery = {"slot": "neck", "neck": true, "step": "sew", "done": 0, "points": _seam_points("neck")}
	t.rooms.surgery_zoom(neck, 3.2)
	t.parts.show_spare()
	t.build_bench()
	Save.mark_dirty()
	Voice.pop()


## After one head goes on, the other head and the other body are taken
## through the same way (Mikey, 2026-10-25).
func _continue_swap() -> void:
	var bodies := Save.camp.headless_bodies()
	if Save.camp.heads.is_empty() or bodies.is_empty():
		return
	t.choose_friend(bodies[0])
	start_head(Save.camp.heads.back())


# --- Surgery ------------------------------------------------------------

## Sew a part onto an open wound. Four layers, in the spirit of Trauma
## Center: pull away loose fluff, connect the hydraulic ligament, soak
## up the spill, then sew along the glowing points.
func start(slot: String, part: PartData) -> void:
	t.mode = "surgery"
	t.hud.spare_visible = false
	# Onto a hole sewn shut: the old stitches are cut open first.
	if not t.friend.wounds.has(slot):
		t.friend.wounds[slot] = t.friend.make_tufts()
	t.friend.surgery = {"slot": slot, "part": part, "step": "fluff", "points": [], "done": 0}
	t.rooms.surgery_zoom(t.friend.socket(slot) + Vector2(0, 40), 3.2)
	Voice.pop()


## Sew a hole shut where a part is missing (Mikey's playtest notes):
## pull away the loose fluff, then sew across. The friend lives on
## without that part, and nothing leaks or rots there any more.
func start_shut(slot: String) -> void:
	t.mode = "surgery"
	t.friend.surgery = {"slot": slot, "shut": true, "step": "fluff", "points": [], "done": 0}
	t.rooms.surgery_zoom(t.friend.socket(slot), 3.2)
	Voice.pop()


func start_eye() -> void:
	t.mode = "surgery"
	t.friend.snap_eye()
	var a := t.friend.eye_anchor("eye_r")
	t.friend.surgery = {
		"slot": "eye_r", "eye": true, "step": "sew", "done": 0,
		"points": [a + Vector2(-15, -15), a + Vector2(15, 15), a + Vector2(15, -15), a + Vector2(-15, 15)],
	}
	t.rooms.surgery_zoom(a, 4.0)


func _tol() -> float:
	return 40.0 / t.camera.zoom.x


func _port(slot: String) -> Vector2:
	return t.friend.socket(slot) + Vector2(10, 0)


func _tube_rest(slot: String) -> Vector2:
	return t.friend.socket(slot) + Vector2(62, 70)


func press(l: Vector2) -> void:
	var s := t.friend.surgery
	match s["step"]:
		"fluff":
			_pluck(l)
		"tube":
			var end: Vector2 = s["tube_end"]
			_tube_drag = l.distance_to(end) < _tol() * 1.3
		"soak":
			s["cloth"] = l
		"sew":
			s["needle"] = l
			_sew(l)


func drag(l: Vector2, moved: float) -> void:
	var s := t.friend.surgery
	match s["step"]:
		"fluff":
			_pluck(l)
		"tube":
			if _tube_drag:
				s["tube_end"] = l
		"soak":
			s["cloth"] = l
			_soak(l, moved)
		"sew":
			s["needle"] = l
			_sew(l)


func release(l: Vector2) -> void:
	var s := t.friend.surgery
	if s.is_empty():
		t.mode = ""
		return
	match s["step"]:
		"tube":
			if _tube_drag:
				_tube_drag = false
				var slot: String = s["slot"]
				if l.distance_to(_port(slot)) < _tol():
					s["tube_end"] = _port(slot)
					s["connected"] = true
					s["spill"] = 1.0
					s["spill_pos"] = t.friend.socket(slot) + Vector2(36, 28)
					s["step"] = "soak"
					t.friend.fx.spawn("fluid", t.fx.to_local(t.friend.to_global(_port(slot))), 10, 6.0)
					Voice.pop()
					Events.surgery_step.emit("ligament")
				else:
					s["tube_end"] = _tube_rest(slot)
		"soak":
			s["cloth"] = null
		"sew":
			s["needle"] = null


func _pluck(l: Vector2) -> void:
	var s := t.friend.surgery
	var slot: String = s["slot"]
	var tufts: Array = t.friend.wounds.get(slot, [])
	var sock := t.friend.socket(slot)
	for i in range(tufts.size() - 1, -1, -1):
		var p: Vector2 = sock + tufts[i]
		if p.distance_to(l) < _tol():
			tufts.remove_at(i)
			t.fx.spawn("fluff", t.fx.to_local(t.friend.to_global(p)), 4, 6.0)
			Voice.poof()
	if tufts.is_empty():
		if s.get("shut", false):
			s["step"] = "sew"
			s["points"] = _shut_points(slot)
			s["done"] = 0
		else:
			s["step"] = "tube"
			s["tube_end"] = _tube_rest(slot)
		Events.surgery_step.emit("fluff_cleared")


func _soak(l: Vector2, moved: float) -> void:
	var s := t.friend.surgery
	var sp: Vector2 = s.get("spill_pos", Vector2.ZERO)
	if l.distance_to(sp) < 56.0:
		s["spill"] = maxf(0.0, float(s["spill"]) - moved * 0.012)
		if t.sound_cd <= 0.0:
			Voice.squish()
			t.sound_cd = 0.25
	if float(s["spill"]) <= 0.03:
		s["spill"] = 0.0
		s["cloth"] = null
		s["step"] = "sew"
		s["points"] = _seam_points(s["slot"])
		s["done"] = 0
		Events.surgery_step.emit("soaked")


func _seam_points(slot: String) -> Array:
	var sock := t.friend.socket(slot)
	var a := sock + Vector2(-30, 10)
	var b := sock + Vector2(30, 10)
	var pts: Array = []
	for i in 7:
		pts.append(a.lerp(b, i / 6.0) + Vector2(0, -10.0 if i % 2 == 0 else 12.0))
	return pts


## Across the hole, side to side: a zigzag that pulls it closed.
func _shut_points(slot: String) -> Array:
	var sock := t.friend.socket(slot)
	var pts: Array = []
	for i in 7:
		pts.append(sock + Vector2(-27.0 + i * 9.0, -14.0 if i % 2 == 0 else 14.0))
	return pts


func _sew(l: Vector2) -> void:
	var s := t.friend.surgery
	var pts: Array = s["points"]
	var done: int = s["done"]
	if done >= pts.size():
		return
	var target: Vector2 = pts[done]
	if l.distance_to(target) < _tol():
		s["done"] = done + 1
		Voice.pip()
		t.fx.spawn("sparkle", t.fx.to_local(t.friend.to_global(target)), 1, 2.0)
		if done + 1 >= pts.size():
			_finish()


func _finish() -> void:
	var s := t.friend.surgery
	var slot: String = s["slot"]
	if s.get("eye", false):
		var eye: PartData = Save.friend.part("eye_r")
		eye.attached = true
		eye.stitched = true
	elif s.get("neck", false):
		Events.part_changed.emit("head", "sewn")
		_continue_swap.call_deferred()
	elif s.get("shut", false):
		Save.friend.close_hole(slot)
		t.friend.wounds.erase(slot)
		Events.part_changed.emit(slot, "closed")
	else:
		var part: PartData = s["part"]
		part.attached = true
		part.stitched = true
		Save.friend.parts[slot] = part
		Save.friend.part_sewn(slot)
		t.friend.wounds.erase(slot)
		Save.camp.spares.erase(part)
	if not s.get("shut", false):
		Events.part_changed.emit(slot, "sewn")
	Events.surgery_step.emit("sewn")
	t.friend.surgery = {}
	t.mode = ""
	t.rooms.surgery_done()
	t.friend.set_expression("squint", "open", 1.6)
	Events.friend_reacted.emit("cheer")
	var at := t.friend.eye_anchor("eye_r") if slot == "eye_r" else t.friend.socket(slot)
	if slot == "neck":
		at = t.friend.socket("neck")
	t.fx.spawn("sparkle", t.fx.to_local(t.friend.to_global(at)), 8, 20.0)
	Save.save_now()
