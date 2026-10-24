extends Node2D
## The tent at the starting camp: one creature, front-on, and the
## first five minutes from Mikey's recording (2026-10-24). Rip off the
## rotting leg, toss it in the bin, sew on the spare in a layered
## surgery, put the dangling eye back, wash, dry, feed stuffing and oil,
## and poke, tickle and pet them. Double-tap to zoom in or out.

signal go_to(place: String)

const CREATURE_POS := Vector2(360, 650)
const VIEW_CENTER := Vector2(360, 640)
## Tools are held a little above the finger so it doesn't hide them.
const TOOL_OFFSET := Vector2(0, -80)

var creature: CreatureView
var fx: FxLayer
var camera: Camera2D
var hud: TentHud
var layer: CanvasLayer
var hint: Label

## What the current touch is doing: "", "empty", "touch", "limb",
## "eye", "tool", "part" or "surgery".
var mode := ""
var tool := ""
var held: LoosePart
var loose_parts: Array = []
var zoomed := false

var _parts_layer: Node2D
var _now := 0.0
var _press_pos := Vector2.ZERO
var _press_time := 0.0
var _touch_moved := 0.0
var _last_tap_time := -10.0
var _last_tap_pos := Vector2.ZERO
var _last_world := Vector2.ZERO
var _drag_vel := Vector2.ZERO
var _sound_cd := 0.0
var _reset_timer := 0.0
var _hint_timer := 0.0
var _tube_drag := false
var _cam_tween: Tween


func _ready() -> void:
	var bg := TentBackground.new()
	add_child(bg)
	creature = CreatureView.new()
	creature.position = CREATURE_POS
	add_child(creature)
	_parts_layer = Node2D.new()
	add_child(_parts_layer)
	fx = FxLayer.new()
	add_child(fx)
	creature.fx = fx
	creature.setup(Save.creature)
	fx.floor_y = CREATURE_POS.y + creature.feet_y()
	bg.floor_y = CREATURE_POS.y + creature.feet_y() + 4.0
	camera = Camera2D.new()
	camera.position = VIEW_CENTER
	add_child(camera)

	layer = CanvasLayer.new()
	add_child(layer)
	hud = TentHud.new()
	layer.add_child(hud)
	hint = Label.new()
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	hint.add_theme_font_size_override("font_size", 30)
	hint.add_theme_color_override("font_color", Color(1.0, 0.95, 0.85))
	hint.add_theme_color_override("font_outline_color", Color(0.14, 0.09, 0.07))
	hint.add_theme_constant_override("outline_size", 12)
	hint.mouse_filter = Control.MOUSE_FILTER_IGNORE
	layer.add_child(hint)

	if Save.spare_available:
		_show_spare()
	_update_hint()
	# Debug: switch parts off to see what each costs (debug/flags.gd).
	bg.visible = not Flags.has("nobg")
	creature.visible = not Flags.has("nocreature")
	layer.visible = not Flags.has("nohud")


func set_active(on: bool) -> void:
	visible = on
	layer.visible = on
	process_mode = Node.PROCESS_MODE_INHERIT if on else Node.PROCESS_MODE_DISABLED
	if on:
		camera.make_current()


func to_world(screen: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform().affine_inverse() * screen


# --- Input --------------------------------------------------------------

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventScreenTouch and event.index == 0:
		if event.pressed:
			_press(event.position)
		else:
			_release(event.position)
	elif event is InputEventScreenDrag and event.index == 0:
		_drag(event.position, event.relative, event.velocity)


func _press(sp: Vector2) -> void:
	var w := to_world(sp)
	var l := creature.to_local(w)
	_press_pos = sp
	_press_time = _now
	_touch_moved = 0.0
	_last_world = w
	_drag_vel = Vector2.ZERO
	if mode == "surgery":
		_surgery_press(l)
		return
	var h: String = hud.hit(sp)
	if h == "play":
		go_to.emit("play_corner")
		return
	if h == "reset":
		if hud.reset_armed:
			Save.new_game()
			get_tree().reload_current_scene()
		else:
			hud.reset_armed = true
			_reset_timer = 3.0
		return
	if h.begins_with("tool:"):
		mode = "tool"
		tool = h.substr(5)
		hud.held_tool = tool
		hud.tool_pos = sp + TOOL_OFFSET
		Voice.pop()
		return
	if h == "spare":
		held = _spawn_loose(hud.spare_part, w, true)
		hud.spare_visible = false
		mode = "part"
		Voice.pop()
		return
	for lp in loose_parts:
		if lp.hits(w):
			held = lp
			lp.pick_up()
			mode = "part"
			hud.trash_visible = true
			return
	if _now - _last_tap_time < 0.32 and sp.distance_to(_last_tap_pos) < 70.0:
		_last_tap_time = -10.0
		mode = ""
		_toggle_zoom(w)
		return
	var slot := creature.part_at(l)
	if slot == "eye_r" and not Save.creature.asleep:
		creature.grab_eye()
		mode = "eye"
	elif slot != "" and not Save.creature.asleep:
		creature.grab_limb(slot)
		mode = "limb"
	elif creature.is_on_creature(l):
		mode = "touch"
	else:
		mode = "empty"


func _drag(sp: Vector2, rel: Vector2, vel: Vector2) -> void:
	var w := to_world(sp)
	var l := creature.to_local(w)
	var moved := (w - _last_world).length()
	_drag_vel = vel / camera.zoom.x
	_touch_moved += rel.length()
	match mode:
		"surgery":
			_surgery_drag(l, moved)
		"limb":
			var torn := creature.drag_limb(l, get_process_delta_time())
			if torn:
				held = _spawn_loose(torn, w, false)
				mode = "part"
				hud.trash_visible = true
		"eye":
			creature.drag_eye(l)
		"touch":
			if _touch_moved > 30.0:
				creature.pet(l, moved)
		"tool":
			hud.tool_pos = sp + TOOL_OFFSET
			_use_tool(creature.to_local(to_world(hud.tool_pos)))
		"part":
			held.position = w
			hud.trash_hover = not held.from_pocket and hud.over_trash(sp)
	_last_world = w


func _release(sp: Vector2) -> void:
	var w := to_world(sp)
	var l := creature.to_local(w)
	match mode:
		"surgery":
			_surgery_release(l)
			return
		"limb":
			creature.release_limb()
		"eye":
			if creature.release_eye():
				_start_eye_sew()
				return
		"touch":
			if _touch_moved < 30.0 and _now - _press_time < 0.45:
				creature.poke(l)
				_last_tap_time = _now
				_last_tap_pos = sp
			creature.end_pet()
		"empty":
			_last_tap_time = _now
			_last_tap_pos = sp
		"tool":
			_finish_tool(creature.to_local(to_world(hud.tool_pos)))
			hud.held_tool = ""
		"part":
			_release_part(sp, w)
	if mode != "surgery":
		mode = ""


# --- Tools --------------------------------------------------------------

func _use_tool(l: Vector2) -> void:
	match tool:
		"sponge":
			if creature.sponge(l) and _sound_cd <= 0.0:
				Voice.splash()
				_sound_cd = 0.35
		"towel":
			if creature.towel(l) and _sound_cd <= 0.0:
				Voice.squish()
				_sound_cd = 0.3
		"stuffing", "oil":
			if l.distance_to(creature.mouth_pos()) < 80.0 and not Save.creature.asleep:
				creature.set_expression("open", "open", 0.15)


func _finish_tool(l: Vector2) -> void:
	match tool:
		"stuffing", "oil":
			if l.distance_to(creature.mouth_pos()) < 80.0:
				creature.wake()
				if tool == "stuffing":
					creature.feed_stuffing()
				else:
					creature.feed_oil()
		"sponge":
			if Save.creature.mud.is_empty():
				Events.cared.emit("washed")
		"towel":
			if Save.creature.wetness < 0.05:
				Events.cared.emit("dried")
	tool = ""


# --- Loose parts, the bin and the spare ---------------------------------

func _spawn_loose(part: PartData, w: Vector2, pocket: bool) -> LoosePart:
	var lp := LoosePart.new()
	lp.part = part
	lp.trim = Save.creature.trim
	lp.from_pocket = pocket
	lp.floor_y = fx.floor_y
	lp.position = w
	_parts_layer.add_child(lp)
	if not pocket:
		loose_parts.append(lp)
	return lp


func _release_part(sp: Vector2, w: Vector2) -> void:
	var lp := held
	held = null
	hud.trash_hover = false
	hud.trash_visible = false
	if lp == null:
		return
	if lp.from_pocket:
		var slot := _open_slot_near(w, lp.part.kind)
		lp.queue_free()
		if slot != "":
			start_surgery(slot, lp.part)
		else:
			_show_spare()
			Voice.pop()
		return
	if hud.over_trash(sp) or sp.distance_to(hud.trash_center()) < 120.0:
		_trash(lp)
	else:
		lp.drop(_drag_vel * 0.6)
		Voice.thud()


func _open_slot_near(w: Vector2, kind: String) -> String:
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		if not slot.begins_with(kind):
			continue
		if Save.creature.is_open(slot) and creature.to_global(creature.socket(slot)).distance_to(w) < 140.0:
			return slot
	return ""


func _trash(lp: LoosePart) -> void:
	loose_parts.erase(lp)
	var target := to_world(hud.trash_center())
	var tw := create_tween().set_parallel()
	tw.tween_property(lp, "position", target, 0.25).set_trans(Tween.TRANS_SINE)
	tw.tween_property(lp, "scale", Vector2(0.15, 0.15), 0.25)
	tw.chain().tween_callback(lp.queue_free)
	Voice.clunk()
	Events.part_changed.emit(lp.part.slot, "trashed")
	_offer_spare()


## Once the rotten leg is gone, a spare waits on the side.
func _offer_spare() -> void:
	if Save.spare_available:
		return
	for slot in ["leg_l", "leg_r"]:
		if Save.creature.is_open(slot):
			Save.spare_available = true
			Save.mark_dirty()
			_show_spare()
			return


func _show_spare() -> void:
	hud.spare_part = CreatureData.spare_leg()
	hud.spare_visible = true


# --- Surgery ------------------------------------------------------------

## Sew a part onto an open wound. Four layers, in the spirit of Trauma
## Center: pull away loose fluff, connect the hydraulic ligament, soak
## up the spill, then sew along the glowing points.
func start_surgery(slot: String, part: PartData) -> void:
	mode = "surgery"
	hud.spare_visible = false
	creature.surgery = {"slot": slot, "part": part, "step": "fluff", "points": [], "done": 0}
	_zoom_to(creature.to_global(creature.socket(slot) + Vector2(0, 40)), 2.3)
	Voice.pop()


func _start_eye_sew() -> void:
	mode = "surgery"
	creature.snap_eye()
	var a := creature.eye_anchor("eye_r")
	creature.surgery = {
		"slot": "eye_r", "eye": true, "step": "sew", "done": 0,
		"points": [a + Vector2(-15, -15), a + Vector2(15, 15), a + Vector2(15, -15), a + Vector2(-15, 15)],
	}
	_zoom_to(creature.to_global(a), 3.0)


func _tol() -> float:
	return 40.0 / camera.zoom.x


func _port(slot: String) -> Vector2:
	return creature.socket(slot) + Vector2(10, 0)


func _tube_rest(slot: String) -> Vector2:
	return creature.socket(slot) + Vector2(62, 70)


func _surgery_press(l: Vector2) -> void:
	var s := creature.surgery
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


func _surgery_drag(l: Vector2, moved: float) -> void:
	var s := creature.surgery
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


func _surgery_release(l: Vector2) -> void:
	var s := creature.surgery
	if s.is_empty():
		mode = ""
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
					s["spill_pos"] = creature.socket(slot) + Vector2(36, 28)
					s["step"] = "soak"
					creature.fx.spawn("fluid", fx.to_local(creature.to_global(_port(slot))), 10, 6.0)
					Voice.pop()
					Events.surgery_step.emit("ligament")
				else:
					s["tube_end"] = _tube_rest(slot)
		"soak":
			s["cloth"] = null
		"sew":
			s["needle"] = null


func _pluck(l: Vector2) -> void:
	var s := creature.surgery
	var slot: String = s["slot"]
	var tufts: Array = creature.wounds.get(slot, [])
	var sock := creature.socket(slot)
	for i in range(tufts.size() - 1, -1, -1):
		var p: Vector2 = sock + tufts[i]
		if p.distance_to(l) < _tol():
			tufts.remove_at(i)
			fx.spawn("fluff", fx.to_local(creature.to_global(p)), 4, 6.0)
			Voice.poof()
	if tufts.is_empty():
		s["step"] = "tube"
		s["tube_end"] = _tube_rest(slot)
		Events.surgery_step.emit("fluff_cleared")


func _soak(l: Vector2, moved: float) -> void:
	var s := creature.surgery
	var sp: Vector2 = s.get("spill_pos", Vector2.ZERO)
	if l.distance_to(sp) < 56.0:
		s["spill"] = maxf(0.0, float(s["spill"]) - moved * 0.012)
		if _sound_cd <= 0.0:
			Voice.squish()
			_sound_cd = 0.25
	if float(s["spill"]) <= 0.03:
		s["spill"] = 0.0
		s["cloth"] = null
		s["step"] = "sew"
		s["points"] = _seam_points(s["slot"])
		s["done"] = 0
		Events.surgery_step.emit("soaked")


func _seam_points(slot: String) -> Array:
	var sock := creature.socket(slot)
	var a := sock + Vector2(-30, 10)
	var b := sock + Vector2(30, 10)
	var pts: Array = []
	for i in 7:
		pts.append(a.lerp(b, i / 6.0) + Vector2(0, -10.0 if i % 2 == 0 else 12.0))
	return pts


func _sew(l: Vector2) -> void:
	var s := creature.surgery
	var pts: Array = s["points"]
	var done: int = s["done"]
	if done >= pts.size():
		return
	var target: Vector2 = pts[done]
	if l.distance_to(target) < _tol():
		s["done"] = done + 1
		Voice.pip()
		fx.spawn("sparkle", fx.to_local(creature.to_global(target)), 1, 2.0)
		if done + 1 >= pts.size():
			_finish_sew()


func _finish_sew() -> void:
	var s := creature.surgery
	var slot: String = s["slot"]
	if s.get("eye", false):
		var eye: PartData = Save.creature.part("eye_r")
		eye.attached = true
		eye.stitched = true
	else:
		var part: PartData = s["part"]
		part.attached = true
		part.stitched = true
		Save.creature.parts[slot] = part
		creature.wounds.erase(slot)
		Save.spare_available = false
	Events.part_changed.emit(slot, "sewn")
	Events.surgery_step.emit("sewn")
	creature.surgery = {}
	mode = ""
	_zoom_reset()
	creature.set_expression("squint", "open", 1.6)
	Events.creature_reacted.emit("cheer")
	var at := creature.eye_anchor("eye_r") if slot == "eye_r" else creature.socket(slot)
	fx.spawn("sparkle", fx.to_local(creature.to_global(at)), 8, 20.0)
	Save.save_now()


# --- Camera -------------------------------------------------------------

func _zoom_to(world: Vector2, z: float) -> void:
	zoomed = true
	_tween_camera(world, z)


func _zoom_reset() -> void:
	zoomed = false
	_tween_camera(VIEW_CENTER, 1.0)


func _toggle_zoom(world: Vector2) -> void:
	if zoomed:
		_zoom_reset()
	else:
		_zoom_to(world, 2.0)


func _tween_camera(pos: Vector2, z: float) -> void:
	if _cam_tween:
		_cam_tween.kill()
	_cam_tween = create_tween().set_parallel().set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	_cam_tween.tween_property(camera, "position", pos, 0.45)
	_cam_tween.tween_property(camera, "zoom", Vector2(z, z), 0.45)


# --- Every frame --------------------------------------------------------

func _process(delta: float) -> void:
	_now += delta
	_sound_cd -= delta
	if hud.reset_armed:
		_reset_timer -= delta
		if _reset_timer <= 0.0:
			hud.reset_armed = false
	_hint_timer -= delta
	if _hint_timer <= 0.0:
		_hint_timer = 0.25
		_update_hint()
	var vs := get_viewport_rect().size
	hint.position = Vector2(40, 118)
	hint.size = Vector2(vs.x - 80, 120)


func _update_hint() -> void:
	hint.text = current_hint()


## Gentle guidance for the first five minutes. (Later the body alone
## should say all of this; pillar 1.)
func current_hint() -> String:
	var c := Save.creature
	if mode == "surgery":
		var s := creature.surgery
		if s.get("eye", false):
			return "Stitch the eye back on: follow the glowing points."
		match s.get("step", ""):
			"fluff": return "Pull away the loose fluff around the wound."
			"tube": return "Connect the ligament tube to the port."
			"soak": return "Soak up the spilled fluid with the cloth."
			"sew": return "Sew it on: follow the glowing points."
	if c.asleep:
		return "Your friend isn't well. Poke them gently to wake them."
	if held and not held.from_pocket:
		return "Toss it in the bin."
	if held and held.from_pocket:
		return "Bring it to the wound."
	var leg: PartData = c.part("leg_l")
	if leg and leg.rot > 0.5:
		return "That leg is rotting. Grab it and pull hard."
	if not loose_parts.is_empty():
		return "Throw the rotten leg in the bin."
	if hud.spare_visible:
		return "A spare leg is waiting on the right. Drag it onto the wound."
	var eye: PartData = c.part("eye_r")
	if eye and not eye.attached:
		return "An eye is hanging by a thread. Drag it back into place."
	if not c.mud.is_empty():
		return "Rub the mud off with the sponge."
	if c.wetness > 0.3:
		return "They're soaked. Dry them with the towel."
	if c.stuffing < 0.9:
		return "They lost stuffing. Drag some to their mouth."
	if c.fluid < 0.6:
		return "Stiff, squeaky joints. Give them a little oil."
	return "All patched up. Poke, tickle or pet them, or visit the play corner."
