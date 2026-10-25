extends Node
## What the playthrough test (flow_test.gd) is made of: real touch
## events, waits, checks and screenshots.

var out_dir := ""
var main: Node
var tent: Tent
var failures: Array[String] = []
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


## Sew along the glowing points of a surgery.
func _sew_points(c: FriendView) -> void:
	await _wait(0.2)
	var pts: Array = c.surgery.get("points", [])
	if pts.size() > 0:
		await _touch(_screen(c.to_global(pts[0])), true)
		for i in range(1, pts.size()):
			await _drag_path(_last, _screen(c.to_global(pts[i])), 5)
		await _touch(_last, false)
	await _wait(0.8)


func _shot(name: String) -> void:
	if out_dir == "":
		return
	await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	img.save_png(out_dir.path_join(name + ".png"))
