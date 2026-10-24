extends Node2D
## The tent at the starting camp: the friend being looked after,
## front-on, with the camp's other friends sitting further back (tap
## one to bring them forward), and the first five minutes from Mikey's
## recording (2026-10-24). Rip off the
## rotting leg, toss it in the bin, sew on the spare in a layered
## surgery, put the dangling eye back, wash, dry, feed stuffing and oil,
## and poke, tickle and pet them. Double-tap to zoom in or out.

signal go_to(place: String)

const FRIEND_POS := Vector2(360, 650)
const VIEW_CENTER := Vector2(360, 640)
## Tools are held a little above the finger so it doesn't hide them.
const TOOL_OFFSET := Vector2(0, -80)

var friend: FriendView
## The camp's other friends, sitting further back: index in
## Save.camp.friends -> BenchFriend.
var bench: Dictionary = {}
## Footprints and bugs on the floor: what happened at camp (0029).
var traces: CampTraces
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
## A head in the player's hand (0026).
var held_head: LooseHead
var loose_parts: Array = []
var zoomed := false

var _parts_layer: Node2D
var _bench_layer: Node2D
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
var _sprayed := false
var _press_region := ""
var _press_world := Vector2.ZERO
var _friend_count := 0
var _cam_tween: Tween


func _ready() -> void:
	var bg := TentBackground.new()
	add_child(bg)
	traces = CampTraces.new()
	add_child(traces)
	traces.setup(Save.camp)
	_bench_layer = Node2D.new()
	add_child(_bench_layer)
	friend = FriendView.new()
	friend.position = FRIEND_POS
	add_child(friend)
	_parts_layer = Node2D.new()
	add_child(_parts_layer)
	fx = FxLayer.new()
	add_child(fx)
	friend.fx = fx
	friend.setup(Save.friend)
	fx.floor_y = FRIEND_POS.y + friend.feet_y()
	bg.floor_y = FRIEND_POS.y + friend.feet_y() + 4.0
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

	hud.supplies = Save.camp.supplies
	_show_spare()
	_build_bench()
	Events.friend_arrived.connect(_on_friend_arrived)
	Events.quest_ended.connect(_on_quest_ended)
	_update_hint()
	# Debug: switch parts off to see what each costs (debug/flags.gd).
	bg.visible = not Flags.has("nobg")
	friend.visible = not Flags.has("nofriend")
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
	var l := friend.to_local(w)
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
		if not Save.friend.dead and not Save.friend.headless:
			go_to.emit("play_corner")
		return
	if h == "map":
		go_to.emit("map")
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
		if hud.amount(h.substr(5)) <= 0:
			# Nothing left: the empty thing on the shelf shakes (0042).
			hud.shake(h.substr(5))
			Voice.thud()
			return
		mode = "tool"
		tool = h.substr(5)
		hud.held_tool = tool
		hud.tool_pos = sp + TOOL_OFFSET
		Voice.pop()
		return
	if h == "head":
		held_head = _spawn_head(Save.camp.heads[0], w)
		mode = "head_held"
		hud.trash_visible = true
		Voice.pop()
		return
	if h == "spare":
		held = _spawn_loose(hud.spare_part, w, true)
		hud.spare_visible = false
		mode = "part"
		Voice.pop()
		return
	var sitting := _bench_at(w)
	if sitting >= 0 and not friend.is_on_friend(l) and held == null:
		choose_friend(sitting)
		return
	if held == null and traces.shoo(w):
		Voice.squish()
		Save.mark_dirty()
		mode = "empty"
		return
	if (Save.friend.dead or Save.friend.headless) and held == null:
		# A friend who has died doesn't answer any more (0011), and a body
		# without a head is only a body; their parts can still be taken
		# (0026).
		var part_slot := friend.part_at(l)
		if part_slot.begins_with("arm") or part_slot.begins_with("leg"):
			friend.grab_limb(part_slot)
			mode = "limb"
		else:
			mode = "empty"
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
	var slot := friend.part_at(l)
	if slot == "eye_r" and not Save.friend.asleep:
		friend.grab_eye()
		mode = "eye"
	elif slot != "" and not Save.friend.asleep:
		friend.grab_limb(slot)
		mode = "limb"
	elif friend.is_on_friend(l):
		mode = "touch"
		_press_region = friend.region_at(l)
		_press_world = w
	else:
		mode = "empty"


func _drag(sp: Vector2, rel: Vector2, vel: Vector2) -> void:
	var w := to_world(sp)
	var l := friend.to_local(w)
	var moved := (w - _last_world).length()
	_drag_vel = vel / camera.zoom.x
	_touch_moved += rel.length()
	match mode:
		"surgery":
			_surgery_drag(l, moved)
		"limb":
			var torn := friend.drag_limb(l, get_process_delta_time())
			if torn:
				held = _spawn_loose(torn, w, false)
				mode = "part"
				hud.trash_visible = true
		"eye":
			friend.drag_eye(l)
		"touch":
			# Pulling hard on the head, away from the body: the neck
			# takes the strain (0026). Otherwise it's petting.
			if _press_region in ["head", "head_top"] and not Save.friend.asleep \
					and (w - _press_world).length() > 90.0 and (w - _press_world).y < 0.0:
				friend.end_pet()
				friend.grab_head()
				mode = "neck"
			elif _touch_moved > 30.0:
				friend.pet(l, moved)
		"neck":
			if friend.drag_head(l, get_process_delta_time()):
				_head_off(w)
		"head_held":
			held_head.position = w
			hud.trash_hover = hud.over_trash(sp)
		"tool":
			hud.tool_pos = sp + TOOL_OFFSET
			_use_tool(friend.to_local(to_world(hud.tool_pos)))
		"part":
			held.position = w
			hud.trash_hover = not held.from_pocket and hud.over_trash(sp)
			hud.pocket_open = not held.from_pocket
	_last_world = w


func _release(sp: Vector2) -> void:
	var w := to_world(sp)
	var l := friend.to_local(w)
	match mode:
		"surgery":
			_surgery_release(l)
			return
		"limb":
			friend.release_limb()
		"neck":
			friend.release_head()
		"head_held":
			_release_head(sp, w)
		"eye":
			if friend.release_eye():
				if Save.camp.use("thread"):
					Save.mark_dirty()
					_start_eye_sew()
					return
				hud.shake("thread")
				Voice.thud()
		"touch":
			if _touch_moved < 30.0 and _now - _press_time < 0.45:
				friend.poke(l)
				_last_tap_time = _now
				_last_tap_pos = sp
			friend.end_pet()
		"empty":
			_last_tap_time = _now
			_last_tap_pos = sp
		"tool":
			_finish_tool(friend.to_local(to_world(hud.tool_pos)))
			hud.held_tool = ""
		"part":
			_release_part(sp, w)
	if mode != "surgery":
		mode = ""


# --- The camp's other friends -----------------------------------------

## Where friends sit further back, in the order they fill up.
const BENCH_X := [110.0, 610.0, 228.0, 492.0]
const BENCH_SCALE := 0.42


func _bench_pos(n: int) -> Vector2:
	var floor_y := FRIEND_POS.y + friend.feet_y()
	return Vector2(BENCH_X[n], floor_y - 26.0 - friend.feet_y() * BENCH_SCALE)


## Sit every friend but the one being looked after further back.
func _build_bench() -> void:
	for v in bench.values():
		v.queue_free()
	bench.clear()
	var n := 0
	for i in Save.camp.friends.size():
		if i == Save.camp.current or n >= BENCH_X.size():
			continue
		var v := BenchFriend.new()
		v.position = _bench_pos(n)
		_bench_layer.add_child(v)
		v.setup(Save.camp.friends[i], BENCH_SCALE)
		bench[i] = v
		n += 1


## Which sitting friend is under a world point, or -1.
func _bench_at(w: Vector2) -> int:
	for i in bench:
		var v: BenchFriend = bench[i]
		if v.hits(w):
			return i
	return -1


## Bring a sitting friend forward to be looked after.
func choose_friend(i: int) -> void:
	if i == Save.camp.current or i < 0 or i >= Save.camp.friends.size():
		return
	Save.camp.current = i
	friend.setup(Save.friend)
	_build_bench()
	friend.scale = Vector2(0.85, 0.85)
	create_tween().tween_property(friend, "scale", Vector2.ONE, 0.25).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	if zoomed:
		_zoom_reset()
	Voice.pop()
	Save.mark_dirty()
	Events.friend_chosen.emit(i)
	_update_hint()


## A new friend walks in and sits down.
func _on_friend_arrived(i: int) -> void:
	_build_bench()
	if bench.has(i):
		var v: BenchFriend = bench[i]
		var to := v.position
		v.position = Vector2(-160.0, to.y)
		create_tween().tween_property(v, "position", to, 1.6).set_trans(Tween.TRANS_SINE)
	Events.friend_reacted.emit("wake")
	Save.mark_dirty()


## Someone came back from a quest (or didn't): the camp shows it. Their
## body shows what happened; what they found is on the shelf and in the
## pocket (0034).
func _on_quest_ended(_place: String, result: String) -> void:
	_build_bench()
	_show_spare()
	hud.supplies = Save.camp.supplies
	fx.spawn("sparkle", fx.to_local(to_world(hud.tool_center(2))), 8, 30.0)
	match result:
		"well": Events.friend_reacted.emit("cheer")
		"hurt": Events.friend_reacted.emit("wince")
		_: Voice.thud()


## The second friend of the first day turns up a couple of minutes
## after the first is patched up (until the arrivals system, step B6).
func _maybe_schedule_arrival() -> void:
	var camp := Save.camp
	if camp.arrivals == 0 and camp.next_arrival == 0.0 and camp.friends.size() == 1 \
			and current_hint().begins_with("All patched up"):
		camp.next_arrival = Save.now() + 120.0
		Save.mark_dirty()


# --- Tools --------------------------------------------------------------

func _use_tool(l: Vector2) -> void:
	if tool == "sponge" and not friend.is_on_friend(l):
		# Scrub footprints off the floor.
		if traces.scrub(friend.to_global(l), 0.08) and _sound_cd <= 0.0:
			Voice.squish()
			_sound_cd = 0.35
			Save.mark_dirty()
		return
	if Save.friend.dead:
		return
	match tool:
		"spray":
			if friend.spray(l):
				_sprayed = true
				Voice.poof()
		"sponge":
			if friend.sponge(l) and _sound_cd <= 0.0:
				Voice.splash()
				_sound_cd = 0.35
		"towel":
			if friend.towel(l) and _sound_cd <= 0.0:
				Voice.squish()
				_sound_cd = 0.3
		"stuffing", "oil":
			if l.distance_to(friend.mouth_pos()) < 80.0 and not Save.friend.asleep:
				friend.set_expression("open", "open", 0.15)


func _finish_tool(l: Vector2) -> void:
	if Save.friend.dead:
		tool = ""
		return
	match tool:
		"stuffing", "oil":
			if l.distance_to(friend.mouth_pos()) < 80.0:
				friend.wake()
				if not Save.camp.use(tool):
					hud.shake(tool)
				elif tool == "stuffing":
					friend.feed_stuffing()
				else:
					friend.feed_oil()
		"spray":
			if _sprayed:
				Save.camp.use("spray")
				Save.mark_dirty()
			_sprayed = false
		"sponge":
			if Save.friend.mud.is_empty():
				Events.cared.emit("washed")
		"towel":
			if Save.friend.wetness < 0.05:
				Events.cared.emit("dried")
	tool = ""


# --- Loose parts, the bin and the spare ---------------------------------

func _spawn_loose(part: PartData, w: Vector2, pocket: bool) -> LoosePart:
	var lp := LoosePart.new()
	lp.part = part
	lp.trim = Save.friend.trim
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
	hud.pocket_open = false
	if lp == null:
		return
	if not lp.from_pocket and hud.over_pocket(sp):
		if lp.part.is_dead():
			# Past repair (0025): only the bin will take it.
			hud.shake("")
			lp.drop(Vector2.ZERO)
			Voice.thud()
			return
		# Keep it for another friend (donation, 0025).
		loose_parts.erase(lp)
		lp.part.pulls = 0
		Save.camp.spares.push_front(lp.part)
		lp.queue_free()
		Save.mark_dirty()
		_show_spare()
		Voice.pop()
		return
	if lp.from_pocket:
		var slot := _open_slot_near(w, lp.part.kind)
		lp.queue_free()
		if slot != "" and Save.camp.use("thread"):
			Save.mark_dirty()
			start_surgery(slot, lp.part)
		elif slot != "":
			# No thread: the spare goes back to the pocket (0042).
			hud.shake("thread")
			_show_spare()
			Voice.thud()
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
		if Save.friend.is_open(slot) and friend.to_global(friend.socket(slot)).distance_to(w) < 140.0:
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


## Once the first rotten leg is gone, a spare waits on the side. It
## comes once: after that, parts are found on quests (decision 0027).
func _offer_spare() -> void:
	if Save.camp.spare_given:
		return
	for slot in ["leg_l", "leg_r"]:
		if Save.friend.is_open(slot):
			Save.camp.spare_given = true
			Save.camp.spares.append(FriendData.spare_leg())
			Save.mark_dirty()
			_show_spare()
			return


## Show the first spare part in the pocket, if there is one, and any
## head kept there.
func _show_spare() -> void:
	var spares := Save.camp.spares
	hud.spare_part = spares[0] if not spares.is_empty() else null
	hud.spare_visible = not spares.is_empty()
	hud.spare_count = spares.size()
	hud.pocket_head = Save.camp.heads[0] if not Save.camp.heads.is_empty() else null


# --- Heads (decision 0026) ----------------------------------------------

func _spawn_head(f: FriendData, w: Vector2) -> LooseHead:
	var lh := LooseHead.new()
	lh.friend = f
	lh.position = w
	_parts_layer.add_child(lh)
	return lh


## The head has come off: the friend lives on in the head, now in the
## player's hand; what stays is a body without a head.
func _head_off(w: Vector2) -> void:
	var head := Save.friend
	var body := head.take_head(Save.now())
	Save.camp.friends[Save.camp.current] = body
	friend.setup(body)
	fx.spawn("fluff", fx.to_local(friend.to_global(friend.socket("neck"))), 20, 22.0)
	Voice.tear(1.0)
	Events.part_changed.emit("head", "torn")
	held_head = _spawn_head(head, w)
	mode = "head_held"
	hud.trash_visible = true
	hud.pocket_open = true
	_build_bench()
	Save.mark_dirty()


func _release_head(sp: Vector2, w: Vector2) -> void:
	var lh := held_head
	held_head = null
	hud.trash_hover = false
	hud.trash_visible = false
	hud.pocket_open = false
	mode = ""
	if lh == null:
		return
	var head := lh.friend
	lh.queue_free()
	var heads := Save.camp.heads
	if hud.over_trash(sp) or sp.distance_to(hud.trash_center()) < 120.0:
		# Thrown away: that friend is gone (0026).
		heads.erase(head)
		Voice.clunk()
		Events.part_changed.emit("head", "trashed")
	elif Save.friend.headless and w.distance_to(friend.to_global(friend.socket("neck"))) < 150.0:
		start_head_surgery(head)
	elif not head in heads:
		heads.append(head)
		Voice.pop()
	Save.mark_dirty()
	_show_spare()


## Sew a head onto the body without one in front: the friend lives on
## with this body (0026). Takes a length of thread.
func start_head_surgery(head: FriendData) -> void:
	if not Save.camp.use("thread"):
		hud.shake("thread")
		Voice.thud()
		if not head in Save.camp.heads:
			Save.camp.heads.append(head)
		_show_spare()
		return
	Save.camp.heads.erase(head)
	var body := Save.friend
	head.join_body(body)
	Save.camp.friends[Save.camp.current] = head
	friend.setup(head)
	friend.wounds.erase("neck")
	mode = "surgery"
	var neck := friend.socket("neck")
	friend.surgery = {"slot": "neck", "neck": true, "step": "sew", "done": 0, "points": _seam_points("neck")}
	_zoom_to(friend.to_global(neck), 2.2)
	_show_spare()
	_build_bench()
	Save.mark_dirty()
	Voice.pop()


## After one head goes on, the other head and the other body are taken
## through the same way (Mikey, 2026-10-25).
func _continue_swap() -> void:
	var bodies := Save.camp.headless_bodies()
	if Save.camp.heads.is_empty() or bodies.is_empty():
		return
	choose_friend(bodies[0])
	start_head_surgery(Save.camp.heads.back())


# --- Surgery ------------------------------------------------------------

## Sew a part onto an open wound. Four layers, in the spirit of Trauma
## Center: pull away loose fluff, connect the hydraulic ligament, soak
## up the spill, then sew along the glowing points.
func start_surgery(slot: String, part: PartData) -> void:
	mode = "surgery"
	hud.spare_visible = false
	friend.surgery = {"slot": slot, "part": part, "step": "fluff", "points": [], "done": 0}
	_zoom_to(friend.to_global(friend.socket(slot) + Vector2(0, 40)), 2.3)
	Voice.pop()


func _start_eye_sew() -> void:
	mode = "surgery"
	friend.snap_eye()
	var a := friend.eye_anchor("eye_r")
	friend.surgery = {
		"slot": "eye_r", "eye": true, "step": "sew", "done": 0,
		"points": [a + Vector2(-15, -15), a + Vector2(15, 15), a + Vector2(15, -15), a + Vector2(-15, 15)],
	}
	_zoom_to(friend.to_global(a), 3.0)


func _tol() -> float:
	return 40.0 / camera.zoom.x


func _port(slot: String) -> Vector2:
	return friend.socket(slot) + Vector2(10, 0)


func _tube_rest(slot: String) -> Vector2:
	return friend.socket(slot) + Vector2(62, 70)


func _surgery_press(l: Vector2) -> void:
	var s := friend.surgery
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
	var s := friend.surgery
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
	var s := friend.surgery
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
					s["spill_pos"] = friend.socket(slot) + Vector2(36, 28)
					s["step"] = "soak"
					friend.fx.spawn("fluid", fx.to_local(friend.to_global(_port(slot))), 10, 6.0)
					Voice.pop()
					Events.surgery_step.emit("ligament")
				else:
					s["tube_end"] = _tube_rest(slot)
		"soak":
			s["cloth"] = null
		"sew":
			s["needle"] = null


func _pluck(l: Vector2) -> void:
	var s := friend.surgery
	var slot: String = s["slot"]
	var tufts: Array = friend.wounds.get(slot, [])
	var sock := friend.socket(slot)
	for i in range(tufts.size() - 1, -1, -1):
		var p: Vector2 = sock + tufts[i]
		if p.distance_to(l) < _tol():
			tufts.remove_at(i)
			fx.spawn("fluff", fx.to_local(friend.to_global(p)), 4, 6.0)
			Voice.poof()
	if tufts.is_empty():
		s["step"] = "tube"
		s["tube_end"] = _tube_rest(slot)
		Events.surgery_step.emit("fluff_cleared")


func _soak(l: Vector2, moved: float) -> void:
	var s := friend.surgery
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
	var sock := friend.socket(slot)
	var a := sock + Vector2(-30, 10)
	var b := sock + Vector2(30, 10)
	var pts: Array = []
	for i in 7:
		pts.append(a.lerp(b, i / 6.0) + Vector2(0, -10.0 if i % 2 == 0 else 12.0))
	return pts


func _sew(l: Vector2) -> void:
	var s := friend.surgery
	var pts: Array = s["points"]
	var done: int = s["done"]
	if done >= pts.size():
		return
	var target: Vector2 = pts[done]
	if l.distance_to(target) < _tol():
		s["done"] = done + 1
		Voice.pip()
		fx.spawn("sparkle", fx.to_local(friend.to_global(target)), 1, 2.0)
		if done + 1 >= pts.size():
			_finish_sew()


func _finish_sew() -> void:
	var s := friend.surgery
	var slot: String = s["slot"]
	if s.get("eye", false):
		var eye: PartData = Save.friend.part("eye_r")
		eye.attached = true
		eye.stitched = true
	elif s.get("neck", false):
		Events.part_changed.emit("head", "sewn")
		_continue_swap.call_deferred()
	else:
		var part: PartData = s["part"]
		part.attached = true
		part.stitched = true
		Save.friend.parts[slot] = part
		friend.wounds.erase(slot)
		Save.camp.spares.erase(part)
	Events.part_changed.emit(slot, "sewn")
	Events.surgery_step.emit("sewn")
	friend.surgery = {}
	mode = ""
	_zoom_reset()
	friend.set_expression("squint", "open", 1.6)
	Events.friend_reacted.emit("cheer")
	var at := friend.eye_anchor("eye_r") if slot == "eye_r" else friend.socket(slot)
	if slot == "neck":
		at = friend.socket("neck")
	fx.spawn("sparkle", fx.to_local(friend.to_global(at)), 8, 20.0)
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
		traces.refresh()
		hud.supplies = Save.camp.supplies
		# A friend laid to rest (CampData) leaves the tent.
		if Save.camp.friends.size() != _friend_count or friend.data != Save.friend:
			_friend_count = Save.camp.friends.size()
			if friend.data != Save.friend and mode != "surgery":
				friend.setup(Save.friend)
			_build_bench()
			_show_spare()
	var vs := get_viewport_rect().size
	hint.position = Vector2(40, 118)
	hint.size = Vector2(vs.x - 80, 120)


func _update_hint() -> void:
	hint.text = current_hint()
	_maybe_schedule_arrival()


## Gentle guidance for the first five minutes. (Later the body alone
## should say all of this; pillar 1.)
func current_hint() -> String:
	var c := Save.friend
	if held_head:
		return "Put the head on a body without one, keep it in the pocket, or bin it."
	if c.dead:
		return "This friend has died. Pull off any parts worth keeping and drop them in the pocket."
	if c.headless:
		return "A body without a head. Sew a head on, or take its parts."
	if held and not held.from_pocket and held.part and not held.part.is_dead() and held.part.rot < 0.5:
		return "Drop it in the pocket to keep it for another friend, or bin it."
	if Save.camp.bugs > 0:
		return "Bugs! Tap them to chase them off."
	if not Save.camp.tracks.is_empty() and mode != "surgery" and c.part("leg_l") and c.part("leg_l").rot < 0.5:
		return "Muddy footprints. Scrub the floor with the sponge."
	if mode == "surgery":
		var s := friend.surgery
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
