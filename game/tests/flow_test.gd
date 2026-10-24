extends Node
## Plays through the first five minutes with real touch events and
## saves screenshots. Run with a display (or Xvfb):
##   godot --path game -- --flow-test=/tmp/shots
## Prints FLOW OK, or FLOW FAILED with what went wrong, then quits.

var out_dir := ""
var main: Node
var tent: Node2D
var failures: Array[String] = []


func _ready() -> void:
	await get_tree().process_frame
	main = get_parent()
	tent = main.tent
	await _run()


func _run() -> void:
	var c: FriendView = tent.friend
	await _wait(0.8)
	await _shot("01_start")

	# Wake them with a gentle poke on the body.
	await _tap(_screen(c.to_global(Vector2(40, 10))))
	await _wait(1.6)
	_check(not Save.friend.asleep, "poke wakes the friend")
	await _shot("02_awake")

	# Tickle spot.
	var spot := Vector2(Save.friend.tickle_spot.x * c.body_scale(), Save.friend.tickle_spot.y)
	await _tap(_screen(c.to_global(spot)))
	await _wait(0.3)
	await _shot("03_tickle")

	# Grab the rotting leg and pull it off.
	var leg_mid := c.to_global(c.socket("leg_l").lerp(c.tip("leg_l"), 0.6))
	var start := _screen(leg_mid)
	await _touch(start, true)
	var torn := false
	for i in 40:
		var p := start + Vector2(-6.0 * i, 6.0 * i)
		await _drag_to(p)
		if i == 14:
			await _shot("04_tugging")
		if tent.mode == "part":
			torn = true
			break
	_check(torn, "pulling the rotten leg tears it off")
	_check(Save.friend.is_open("leg_l"), "leg slot is an open wound")
	await _wait(0.2)
	await _shot("05_torn")

	# Into the bin.
	var bin: Vector2 = tent.hud.trash_center()
	await _drag_path(_last, bin, 12)
	await _shot("06_over_bin")
	await _touch(bin, false)
	await _wait(0.6)
	_check(tent.hud.spare_visible, "a spare leg waits on the side after the bin")
	await _shot("07_spare_waiting")

	# Drag the spare onto the wound.
	var pocket: Vector2 = tent.hud.pocket_center()
	await _touch(pocket, true)
	var wound := _screen(c.to_global(c.socket("leg_l") + Vector2(0, 30)))
	await _drag_path(pocket, wound, 14)
	await _touch(wound, false)
	await _wait(0.7)
	_check(tent.mode == "surgery", "dropping the spare on the wound starts surgery")
	await _shot("08_surgery_fluff")

	# Step 1: pluck the loose fluff.
	var tufts: Array = c.wounds.get("leg_l", []).duplicate()
	var sock: Vector2 = c.socket("leg_l")
	for t in tufts:
		var sp := _screen(c.to_global(sock + t))
		await _tap(sp)
	await _wait(0.2)
	_check(c.surgery.get("step", "") == "tube", "clearing the fluff moves on to the tube")
	await _shot("09_tube")

	# Step 2: connect the ligament tube.
	var end: Vector2 = c.surgery.get("tube_end", Vector2.ZERO)
	var port: Vector2 = c.socket("leg_l") + Vector2(10, 0)
	await _touch(_screen(c.to_global(end)), true)
	await _drag_path(_screen(c.to_global(end)), _screen(c.to_global(port)), 10)
	await _touch(_screen(c.to_global(port)), false)
	await _wait(0.3)
	_check(c.surgery.get("step", "") == "soak", "connecting the tube moves on to soaking")
	await _shot("10_spill")

	# Step 3: soak up the spill.
	var spill: Vector2 = c.surgery.get("spill_pos", Vector2.ZERO)
	var spill_s := _screen(c.to_global(spill))
	await _touch(spill_s, true)
	for i in 60:
		await _drag_to(spill_s + Vector2(sin(i * 0.7) * 40.0, cos(i * 0.5) * 14.0))
		if c.surgery.get("step", "") != "soak":
			break
	await _touch(_last, false)
	_check(c.surgery.get("step", "") == "sew", "soaking the spill moves on to sewing")
	await _wait(0.2)
	await _shot("11_sew")

	# Step 4: sew along the points.
	var pts: Array = c.surgery.get("points", [])
	if pts.size() > 0:
		var first := _screen(c.to_global(pts[0]))
		await _touch(first, true)
		for i in range(1, pts.size()):
			await _drag_path(_last, _screen(c.to_global(pts[i])), 5)
			if i == 3:
				await _shot("12_sewing")
		await _touch(_last, false)
	await _wait(0.8)
	var leg: PartData = Save.friend.part("leg_l")
	_check(leg != null and leg.attached and leg.stitched, "the spare leg is sewn on with stitches")
	await _shot("13_leg_sewn")

	# The dangling eye.
	var eye_at := _screen(c.to_global(c.eye_position()))
	await _touch(eye_at, true)
	var anchor := _screen(c.to_global(c.eye_anchor("eye_r")))
	await _drag_path(eye_at, anchor, 10)
	await _touch(anchor, false)
	await _wait(0.7)
	_check(tent.mode == "surgery", "putting the eye back starts a stitch")
	await _shot("14_eye_sew")
	pts = c.surgery.get("points", [])
	if pts.size() > 0:
		await _touch(_screen(c.to_global(pts[0])), true)
		for i in range(1, pts.size()):
			await _drag_path(_last, _screen(c.to_global(pts[i])), 5)
		await _touch(_last, false)
	await _wait(0.8)
	_check(Save.friend.part("eye_r").attached, "the eye is stitched back on")

	# Wash with the sponge: scrub each mud spot.
	await _scrub_mud(c)
	_check(Save.friend.mud.is_empty(), "the sponge washes off all the mud")
	await _shot("15_washed_wet")
	# Dry with the towel.
	await _use_tool(1, c, 160)
	_check(Save.friend.wetness < 0.3, "the towel dries them")
	# Feed stuffing twice and oil once.
	for i in 2:
		await _feed(2, c)
	await _feed(3, c)
	await _wait(1.2)
	_check(Save.friend.stuffing >= 0.9, "stuffing refills them")
	_check(Save.friend.fluid > 0.9, "oil loosens the joints")
	await _shot("16_all_good")
	print("wellbeing at end of tent: ", Save.friend.wellbeing())

	# Play corner.
	await _tap(tent.hud.play_rect().get_center())
	await _wait(1.0)
	var play: Node2D = main.play
	_check(play.visible, "the play corner opens")
	await _shot("17_play_corner")
	var ball: RigidBody2D = play.ball
	for throw in 4:
		await _wait(1.2)
		var b := _screen(ball.global_position)
		await _touch(b, true)
		var head := _screen(play.friend.to_global(play.friend.head_center()))
		var from := head + Vector2(260, 120)
		await _drag_path(b, from, 6)
		await _drag_path(from, head, 4)
		await _touch(head, false)
		await _wait(0.25)
		if throw == 1:
			await _shot("18_ball")
	await _wait(1.0)
	await _shot("19_after_play")
	print("catch skill after play: ", Save.friend.catch_skill)

	if failures.is_empty():
		print("FLOW OK")
	else:
		print("FLOW FAILED:")
		for f in failures:
			print("  - ", f)
	get_tree().quit(0 if failures.is_empty() else 1)


# --- Helpers ------------------------------------------------------------

var _last := Vector2.ZERO


func _screen(world: Vector2) -> Vector2:
	return get_viewport().get_canvas_transform() * world


func _check(ok: bool, what: String) -> void:
	print(("  ok   " if ok else "  FAIL ") + what)
	if not ok:
		failures.append(what)


func _wait(seconds: float) -> void:
	await get_tree().create_timer(seconds).timeout


func _touch(p: Vector2, pressed: bool) -> void:
	var ev := InputEventScreenTouch.new()
	ev.index = 0
	ev.position = p
	ev.pressed = pressed
	get_viewport().push_input(ev, true)
	_last = p
	await get_tree().process_frame


func _drag_to(p: Vector2) -> void:
	var ev := InputEventScreenDrag.new()
	ev.index = 0
	ev.position = p
	ev.relative = p - _last
	ev.velocity = (p - _last) * 60.0
	get_viewport().push_input(ev, true)
	_last = p
	await get_tree().process_frame


func _drag_path(a: Vector2, b: Vector2, steps: int) -> void:
	for i in range(1, steps + 1):
		await _drag_to(a.lerp(b, float(i) / steps))


func _tap(p: Vector2) -> void:
	await _touch(p, true)
	await _touch(p, false)
	await _wait(0.12)


func _use_tool(index: int, c: FriendView, strokes: int) -> void:
	var tc: Vector2 = tent.hud.tool_center(index)
	await _touch(tc, true)
	var offset := Vector2(0, 80)
	for i in strokes:
		var a := float(i) * 0.37
		var target := c.to_global(Vector2(sin(a * 1.7) * 100.0, -260.0 + fmod(i * 23.0, 380.0)))
		await _drag_to(_screen(target) + offset)
	await _touch(_last, false)


func _scrub_mud(c: FriendView) -> void:
	var tc: Vector2 = tent.hud.tool_center(0)
	await _touch(tc, true)
	var offset := Vector2(0, 80)
	for m in Save.friend.mud.duplicate():
		var y := float(m["y"])
		var at := Vector2(float(m["x"]) * (c.body_scale() if y > -110.0 else 1.0), y)
		for i in 26:
			var p := at + Vector2(sin(i * 1.3) * 14.0, cos(i * 1.1) * 10.0)
			await _drag_to(_screen(c.to_global(p)) + offset)
	await _touch(_last, false)


func _feed(index: int, c: FriendView) -> void:
	var tc: Vector2 = tent.hud.tool_center(index)
	await _touch(tc, true)
	var mouth := _screen(c.to_global(c.mouth_pos())) + Vector2(0, 80)
	await _drag_path(tc, mouth, 10)
	await _touch(mouth, false)
	await _wait(0.4)


func _shot(name: String) -> void:
	if out_dir == "":
		return
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	img.save_png(out_dir.path_join(name + ".png"))
