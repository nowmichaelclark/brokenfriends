class_name TentHud
extends Node2D
## Everything drawn over the tent in screen space: the tool tray, the
## pocket where a spare part waits, the bin, the buttons and the tool
## in the player's hand. Not an inventory screen (pillar 1): things sit
## on the side of the view, ready to grab.

const TOOLS := ["sponge", "towel", "stuffing", "oil"]
const LABELS := {"sponge": "Sponge", "towel": "Towel", "stuffing": "Stuffing", "oil": "Oil"}

var spare_visible := false
var spare_part: PartData
var trash_visible := false
var trash_hover := false
var held_tool := ""
var tool_pos := Vector2.ZERO
var reset_armed := false
var play_label := "Play corner"

var _t := 0.0
var _trash_pop := 0.0


func screen_size() -> Vector2:
	return get_viewport_rect().size


func tool_center(i: int) -> Vector2:
	var s := screen_size()
	return Vector2(s.x * 0.5 + (i - 1.5) * 160.0, s.y - 100.0)


func pocket_center() -> Vector2:
	var s := screen_size()
	return Vector2(s.x - 66.0, s.y * 0.42)


func trash_center() -> Vector2:
	var s := screen_size()
	return Vector2(s.x * 0.5, s.y - 300.0)


func play_rect() -> Rect2:
	return Rect2(screen_size().x - 236.0, 28.0, 208.0, 64.0)


func reset_rect() -> Rect2:
	return Rect2(28.0, 28.0, 188.0, 56.0)


## What's under a screen point: "tool:<name>", "spare", "play",
## "reset" or "".
func hit(p: Vector2) -> String:
	if play_rect().grow(10).has_point(p):
		return "play"
	if reset_rect().grow(10).has_point(p):
		return "reset"
	if spare_visible and p.distance_to(pocket_center()) < 70.0:
		return "spare"
	for i in TOOLS.size():
		if p.distance_to(tool_center(i)) < 70.0:
			return "tool:" + TOOLS[i]
	return ""


func over_trash(p: Vector2) -> bool:
	return trash_visible and p.distance_to(trash_center()) < 120.0


func _process(delta: float) -> void:
	_t += delta
	_trash_pop = move_toward(_trash_pop, 1.0 if trash_visible else 0.0, delta * 5.0)
	queue_redraw()


func _draw() -> void:
	var s := screen_size()
	var font := ThemeDB.fallback_font
	# Tray: a wooden shelf along the bottom
	draw_rect(Rect2(0, s.y - 190.0, s.x, 190.0), Color(0.33, 0.22, 0.14))
	draw_rect(Rect2(0, s.y - 196.0, s.x, 10.0), Color(0.48, 0.33, 0.2))
	for i in TOOLS.size():
		var tool_name: String = TOOLS[i]
		var c := tool_center(i)
		if held_tool == tool_name:
			draw_arc(c, 54.0, 0, TAU, 24, Color(1, 1, 1, 0.25), 3.0, true)
		else:
			draw_tool(tool_name, c + Vector2(0, -12), 1.0)
		draw_string(font, c + Vector2(-70, 62), LABELS[tool_name], HORIZONTAL_ALIGNMENT_CENTER, 140, 22, Color(0.95, 0.88, 0.75, 0.9))
	# Buttons
	_button(play_rect(), play_label, Color(0.42, 0.55, 0.38))
	_button(reset_rect(), "Tap again" if reset_armed else "Start over", Color(0.55, 0.3, 0.27) if reset_armed else Color(0.3, 0.24, 0.2, 0.85))
	# Pocket with the spare part
	if spare_visible and spare_part:
		var pc := pocket_center()
		var glow := 0.5 + sin(_t * 3.0) * 0.25
		draw_circle(pc, 76.0, Color(1.0, 0.9, 0.6, 0.12 * glow))
		var pocket := PackedVector2Array([pc + Vector2(-56, -10), pc + Vector2(56, -10), pc + Vector2(48, 66), pc + Vector2(-48, 66)])
		var trim := Color(0.94, 0.86, 0.75)
		draw_set_transform(pc + Vector2(0, -6), 0.15, Vector2(0.7, 0.7))
		PlushArt.limb(self, Vector2(0, -50), Vector2(0, 50), 50.0, spare_part, trim, 0.0)
		draw_set_transform(Vector2.ZERO, 0, Vector2.ONE)
		draw_colored_polygon(pocket, Color(0.55, 0.43, 0.3))
		draw_polyline(PackedVector2Array([pocket[0], pocket[1], pocket[2], pocket[3], pocket[0]]), Color(0.3, 0.22, 0.14), 3.0, true)
		PlushArt.seam(self, PackedVector2Array([pocket[0] + Vector2(6, 8), pocket[1] + Vector2(-6, 8)]), Color(0.85, 0.75, 0.55))
	# Bin
	if _trash_pop > 0.01:
		var tc := trash_center()
		var k := _trash_pop
		draw_set_transform(tc, 0, Vector2(k, k))
		var body := PackedVector2Array([Vector2(-62, -40), Vector2(62, -40), Vector2(50, 80), Vector2(-50, 80)])
		draw_colored_polygon(body, Color(0.4, 0.45, 0.38))
		for x in [-30.0, 0.0, 30.0]:
			draw_line(Vector2(x, -28), Vector2(x * 0.85, 70), Color(0.3, 0.34, 0.28), 4.0)
		var lid_ang := -0.6 if trash_hover else 0.0
		draw_set_transform(tc + Vector2(-66, -42).rotated(0) * k, lid_ang, Vector2(k, k))
		draw_rect(Rect2(0, -10, 132, 14), Color(0.32, 0.36, 0.3))
		draw_rect(Rect2(52, -22, 28, 12), Color(0.32, 0.36, 0.3))
		draw_set_transform(Vector2.ZERO, 0, Vector2.ONE)
	# The tool in hand
	if held_tool != "":
		draw_tool(held_tool, tool_pos, 1.25)


func _button(r: Rect2, text: String, col: Color) -> void:
	draw_rect(r, col)
	draw_rect(r, col.darkened(0.4), false, 3.0)
	var font := ThemeDB.fallback_font
	draw_string(font, Vector2(r.position.x, r.position.y + r.size.y * 0.5 + 9.0), text, HORIZONTAL_ALIGNMENT_CENTER, r.size.x, 26, Color(1, 0.96, 0.88))


func draw_tool(tool_name: String, c: Vector2, k: float) -> void:
	draw_set_transform(c, 0, Vector2(k, k))
	match tool_name:
		"sponge":
			draw_rect(Rect2(-40, -26, 80, 52), Color(0.96, 0.82, 0.32))
			draw_rect(Rect2(-40, -26, 80, 52), Color(0.7, 0.55, 0.18), false, 3.0)
			for h in [Vector2(-22, -10), Vector2(8, -14), Vector2(24, 6), Vector2(-6, 10), Vector2(-26, 12)]:
				draw_circle(h, 5.0, Color(0.85, 0.66, 0.22))
		"towel":
			draw_rect(Rect2(-42, -30, 84, 60), Color(0.88, 0.62, 0.66))
			for y in [-14.0, 0.0, 14.0]:
				draw_line(Vector2(-42, y), Vector2(42, y), Color(0.97, 0.9, 0.9), 4.0)
			draw_rect(Rect2(-42, -30, 84, 60), Color(0.6, 0.38, 0.42), false, 3.0)
		"stuffing":
			draw_colored_polygon(PackedVector2Array([Vector2(-34, -6), Vector2(34, -6), Vector2(40, 34), Vector2(-40, 34)]), Color(0.66, 0.53, 0.36))
			draw_line(Vector2(-36, -2), Vector2(36, -2), Color(0.45, 0.34, 0.22), 4.0)
			PlushArt.fluff(self, Vector2(0, -14), 22.0)
		"oil":
			draw_rect(Rect2(-28, -14, 56, 46), Color(0.55, 0.58, 0.6))
			draw_rect(Rect2(-28, -14, 56, 46), Color(0.3, 0.32, 0.35), false, 3.0)
			draw_line(Vector2(20, -10), Vector2(46, -38), Color(0.45, 0.48, 0.5), 7.0)
			draw_arc(Vector2(-28, 8), 16.0, PI * 0.5, PI * 1.5, 10, Color(0.3, 0.32, 0.35), 5.0)
			draw_circle(Vector2(48, -36), 4.0, Color(0.9, 0.75, 0.3))
	draw_set_transform(Vector2.ZERO, 0, Vector2.ONE)
