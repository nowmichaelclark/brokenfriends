class_name Tent
extends Node2D
## The tent at the starting camp, in two rooms (TentRooms). In the
## grooming screen the friend being looked after stands front-on, the
## camp's other friends sitting further back (tap one to bring them
## forward): wash, dry, feed stuffing and oil, spray mold, and poke,
## tickle and pet them. On the surgery table: rip off the rotting leg,
## toss it in the bin, sew on the spare in a layered surgery or sew the
## hole shut, put the dangling eye back, swap heads. The first five
## minutes from Mikey's recording (2026-10-24). Double-tap to zoom.

signal go_to(place: String)

## Tools are held a little above the finger so it doesn't hide them.
const TOOL_OFFSET := Vector2(0, -80)

var friend: FriendView
## The camp's other friends, sitting further back: index in
## Save.camp.friends -> BenchFriend.
var bench: Dictionary = {}
## The wordless pointer at what could be done next (0032).
var guide: Guide
## Footprints and bugs on the floor: what happened at camp (0029).
var traces: CampTraces
var fx: FxLayer
var camera: Camera2D
var hud: TentHud
var layer: CanvasLayer
var hint: Label
## The rooms and the camera; the shelf's tools.
var rooms := TentRooms.new()
var tools := TentTools.new()

## What the current touch is doing: "", "empty", "touch", "limb",
## "eye", "tool", "part" or "surgery".
var mode := ""
var tool := ""
var held: LoosePart
## A head in the player's hand (0026).
var held_head: LooseHead
var loose_parts: Array = []
## Sewing on the table, and parts and heads in the hand.
var surgery := TentSurgery.new()
var parts := TentParts.new()

var parts_layer: Node2D
var bg: Node2D
var table_art: Node2D
var fade: ColorRect
var bench_layer: Node2D
var _now := 0.0
var _press_pos := Vector2.ZERO
var _press_time := 0.0
var _touch_moved := 0.0
var _last_tap_time := -10.0
var _last_tap_pos := Vector2.ZERO
var _last_world := Vector2.ZERO
var drag_vel := Vector2.ZERO
var sound_cd := 0.0
var _hint_timer := 0.0
var _press_region := ""
var _press_world := Vector2.ZERO
var _friend_count := 0


func _ready() -> void:
	surgery.t = self
	parts.t = self
	rooms.t = self
	tools.t = self
	bg = TentBackground.new()
	add_child(bg)
	table_art = SurgeryTable.new()
	table_art.visible = false
	add_child(table_art)
	traces = CampTraces.new()
	add_child(traces)
	traces.setup(Save.camp)
	bench_layer = Node2D.new()
	add_child(bench_layer)
	friend = FriendView.new()
	friend.position = TentRooms.FRIEND_POS
	add_child(friend)
	parts_layer = Node2D.new()
	add_child(parts_layer)
	fx = FxLayer.new()
	add_child(fx)
	guide = Guide.new()
	add_child(guide)
	friend.fx = fx
	friend.setup(Save.friend)
	fx.floor_y = TentRooms.FRIEND_POS.y + friend.feet_y()
	bg.floor_y = TentRooms.FRIEND_POS.y + friend.feet_y() + 4.0
	camera = Camera2D.new()
	camera.position = TentRooms.VIEW_CENTER
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
	# Words only for the developer (#hints); the game shows instead (0032).
	hint.visible = Flags.has("hints")
	fade = ColorRect.new()
	fade.color = Color(0.08, 0.05, 0.04, 0.0)
	fade.mouse_filter = Control.MOUSE_FILTER_IGNORE
	fade.set_anchors_preset(Control.PRESET_FULL_RECT)
	layer.add_child(fade)

	hud.supplies = Save.camp.supplies
	parts.show_spare()
	build_bench()
	rooms.set_room("grooming", true)
	Events.friend_arrived.connect(_on_friend_arrived)
	Events.quest_ended.connect(_on_quest_ended)
	_update_hint()
	# Debug: switch parts off to see what each costs (debug/flags.gd).
	bg.visible = not Flags.has("nobg")
	friend.visible = not Flags.has("nofriend")
	layer.visible = not Flags.has("nohud")


## Shown or not; `room` is "grooming" or "table".
func set_active(on: bool, room := "grooming") -> void:
	visible = on
	layer.visible = on and not Flags.has("nohud")
	process_mode = Node.PROCESS_MODE_INHERIT if on else Node.PROCESS_MODE_DISABLED
	if on:
		if friend.data != Save.friend and mode != "surgery":
			friend.setup(Save.friend)
		build_bench()
		rooms.set_room(room, true)
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
	drag_vel = Vector2.ZERO
	if mode == "surgery":
		surgery.press(l)
		return
	var h: String = hud.hit(sp)
	if h == "entrance":
		go_to.emit("entrance")
		return
	if h == "room":
		go_to.emit("grooming" if rooms.on_table() else "table")
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
		held_head = parts.spawn_head(Save.camp.heads[0], w)
		mode = "head_held"
		hud.trash_visible = true
		Voice.pop()
		return
	if h == "spare":
		held = parts.spawn_loose(hud.spare_part, w, true)
		hud.spare_visible = false
		mode = "part"
		Voice.pop()
		return
	var table := rooms.on_table()
	var sitting := -1 if table else _bench_at(w)
	if sitting >= 0 and not friend.is_on_friend(l) and held == null:
		choose_friend(sitting)
		return
	if held == null and not table and traces.shoo(w):
		Voice.squish()
		Save.mark_dirty()
		mode = "empty"
		return
	if (Save.friend.dead or Save.friend.headless) and held == null:
		# A friend who has died doesn't answer any more (0011), and a body
		# without a head is only a body; their parts can still be taken
		# on the table (0026).
		var part_slot := friend.part_at(l) if table else ""
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
		rooms.toggle_zoom(w)
		return
	# Parts are pulled on the table, not in the grooming screen.
	var slot := friend.part_at(l) if table else ""
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
	drag_vel = vel / camera.zoom.x
	_touch_moved += rel.length()
	match mode:
		"surgery":
			surgery.drag(l, moved)
		"limb":
			var torn := friend.drag_limb(l, get_process_delta_time())
			if torn:
				held = parts.spawn_loose(torn, w, false)
				mode = "part"
				hud.trash_visible = true
		"eye":
			friend.drag_eye(l)
		"touch":
			# Pulling hard on the head, away from the body: the neck
			# takes the strain (0026). Otherwise it's petting.
			if _press_region in ["head", "head_top"] and not Save.friend.asleep and rooms.on_table() \
					and (w - _press_world).length() > 90.0 and (w - _press_world).y < 0.0:
				friend.end_pet()
				friend.grab_head()
				mode = "neck"
			elif _touch_moved > 30.0:
				friend.pet(l, moved)
		"neck":
			if friend.drag_head(l, get_process_delta_time()):
				parts.head_off(w)
		"head_held":
			held_head.position = w
			hud.trash_hover = hud.over_trash(sp)
		"tool":
			hud.tool_pos = sp + TOOL_OFFSET
			tools.use(tool, friend.to_local(to_world(hud.tool_pos)))
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
			surgery.release(l)
			return
		"limb":
			friend.release_limb()
		"neck":
			friend.release_head()
		"head_held":
			parts.release_head(sp, w)
		"eye":
			if friend.release_eye():
				if Save.camp.use("thread"):
					Save.mark_dirty()
					surgery.start_eye()
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
			tools.finish(tool, friend.to_local(to_world(hud.tool_pos)))
			tool = ""
			hud.held_tool = ""
		"part":
			parts.release_part(sp, w)
	if mode != "surgery":
		mode = ""


# --- The camp's other friends -----------------------------------------

## Where friends sit further back, in the order they fill up.
const BENCH_X := [110.0, 610.0, 228.0, 492.0]
const BENCH_SCALE := 0.42


func _bench_pos(n: int) -> Vector2:
	var floor_y := TentRooms.FRIEND_POS.y + friend.feet_y()
	return Vector2(BENCH_X[n], floor_y - 26.0 - friend.feet_y() * BENCH_SCALE)


## Sit every friend but the one being looked after further back.
## Everyone shown here has now been seen (FriendData.met).
func build_bench() -> void:
	if visible:
		for f in Save.camp.friends:
			f.met = true
	for v in bench.values():
		v.queue_free()
	bench.clear()
	var n := 0
	for i in Save.camp.friends.size():
		if i == Save.camp.current or n >= BENCH_X.size():
			continue
		var v := BenchFriend.new()
		v.position = _bench_pos(n)
		bench_layer.add_child(v)
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
	build_bench()
	friend.scale = Vector2(0.85, 0.85)
	create_tween().tween_property(friend, "scale", Vector2.ONE, 0.25).set_trans(Tween.TRANS_BACK).set_ease(Tween.EASE_OUT)
	if rooms.zoomed:
		rooms.zoom_reset()
	Voice.pop()
	Save.mark_dirty()
	Events.friend_chosen.emit(i)
	_update_hint()


## A new friend walks in and sits down.
func _on_friend_arrived(i: int) -> void:
	build_bench()
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
	build_bench()
	parts.show_spare()
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
			and TentCues.hint(self).begins_with("All patched up"):
		camp.next_arrival = Save.now() + 120.0
		Save.mark_dirty()


# --- Every frame --------------------------------------------------------

func _process(delta: float) -> void:
	_now += delta
	sound_cd -= delta
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
			build_bench()
			parts.show_spare()
	TentCues.show(self)
	var vs := get_viewport_rect().size
	hint.position = Vector2(40, 118)
	hint.size = Vector2(vs.x - 80, 120)


func _update_hint() -> void:
	hint.text = TentCues.hint(self)
	_maybe_schedule_arrival()
