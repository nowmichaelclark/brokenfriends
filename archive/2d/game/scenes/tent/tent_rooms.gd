class_name TentRooms
extends RefCounted
## The tent's two rooms and its camera (Mikey's playtest notes,
## 2026-10-25): the grooming screen, where a friend stands to be washed,
## dried, fed and petted, and the surgery table, where parts come off
## and go on. On the table the friend stands upright too, as in the
## grooming screen, only closer (decision 0031: its own place); surgery
## comes in closer still on the wound.

const FRIEND_POS := Vector2(360, 650)
const VIEW_CENTER := Vector2(360, 640)
## The table: where the friend stands, and the camera on them.
const TABLE_POS := Vector2(360, 700)
const TABLE_VIEW := Vector2(360, 610)
const TABLE_ZOOM := 1.35

var t: Tent
## "grooming" or "table".
var room := "grooming"
var zoomed := false

var _cam_tween: Tween


func on_table() -> bool:
	return room == "table"


## Go to a room, with a quick fade, as Trauma Center does between views.
## `instant` skips the fade (on arriving from another place).
func set_room(r: String, instant := false) -> void:
	if r == room and not instant:
		return
	var swap := func():
		room = r
		var table := r == "table"
		t.table_art.visible = table
		for n in [t.bg, t.traces, t.bench_layer]:
			n.visible = not table
		t.parts_layer.visible = table
		t.friend.position = TABLE_POS if table else FRIEND_POS
		# Closer on the table: sharper pictures (decision 0031).
		t.friend.set_detail(3.0 if table else 2.0)
		t.hud.room = r
		t.parts.show_spare()
		_snap_camera()
	if instant:
		swap.call()
	else:
		_fade_through(swap)


## The camera where the room has it, at once.
func _snap_camera() -> void:
	zoomed = false
	if _cam_tween:
		_cam_tween.kill()
	t.camera.position = TABLE_VIEW if on_table() else VIEW_CENTER
	var z := TABLE_ZOOM if on_table() else 1.0
	t.camera.zoom = Vector2(z, z)


## Come in close on `focus` (a point on the friend) for surgery.
func surgery_zoom(focus: Vector2, z: float) -> void:
	zoomed = true
	_tween_camera(t.friend.to_global(focus), z)


## Back to the room's own view once surgery is done.
func surgery_done() -> void:
	zoom_reset()


func _fade_through(swap: Callable) -> void:
	var tw := t.create_tween()
	tw.tween_property(t.fade, "color:a", 1.0, 0.12)
	tw.tween_callback(swap)
	tw.tween_property(t.fade, "color:a", 0.0, 0.22)


func zoom_reset() -> void:
	zoomed = false
	var z := TABLE_ZOOM if on_table() else 1.0
	_tween_camera(TABLE_VIEW if on_table() else VIEW_CENTER, z)


## Double-tap: in close on that spot, or back out.
func toggle_zoom(world: Vector2) -> void:
	if zoomed:
		zoom_reset()
	else:
		zoomed = true
		_tween_camera(world, (TABLE_ZOOM if on_table() else 1.0) * 2.0)


func _tween_camera(pos: Vector2, z: float) -> void:
	if _cam_tween:
		_cam_tween.kill()
	_cam_tween = t.create_tween().set_parallel().set_trans(Tween.TRANS_SINE).set_ease(Tween.EASE_IN_OUT)
	_cam_tween.tween_property(t.camera, "position", pos, 0.45)
	_cam_tween.tween_property(t.camera, "zoom", Vector2(z, z), 0.45)
