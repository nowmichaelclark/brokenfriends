class_name TentHud
extends Node2D
## Everything drawn over the tent in screen space: the tool tray, the
## pocket where a spare part waits, the bin, the buttons and the tool
## in the player's hand. Not an inventory screen (pillar 1): things sit
## on the side of the view, ready to grab.

const TOOLS := ["sponge", "towel", "stuffing", "oil", "spray"]
## On the shelf but not picked up: used up by sewing.
const SHOWN := ["thread"]
const LABELS := {"sponge": "Sponge", "towel": "Towel", "stuffing": "Stuffing", "oil": "Oil", "spray": "Mold spray", "thread": "Thread"}
## Water is free; these are counted (decision 0027).
const COUNTED := ["stuffing", "oil", "spray", "thread"]

## The camp's supplies, shown by how full each thing on the shelf looks.
var supplies: Dictionary = {}
## What to glow, showing what could be used next (no words, 0032): a
## shelf item's name, "pocket", "bin", "play" or "map". "" for none.
var glow := ""
var _glow_t := 0.0
## A shelf item to shake (tapped or needed when empty).
var shake_item := ""
var _shake := 0.0

var spare_visible := false
var spare_part: PartData
## How many parts wait in the pocket (they stack).
var spare_count := 0
## A head waiting in the pocket, shown before any part (0026).
var pocket_head: FriendData
## The pocket shows open while a part or head is carried, to drop it in.
var pocket_open := false
var trash_visible := false
var trash_hover := false
var held_tool := ""
var tool_pos := Vector2.ZERO
var reset_armed := false
var play_label := "Play corner"

var _trash_pop := 0.0


func screen_size() -> Vector2:
	return get_viewport_rect().size


func tool_center(i: int) -> Vector2:
	var s := screen_size()
	var n := TOOLS.size() + SHOWN.size()
	return Vector2(s.x * 0.5 + (i - (n - 1) * 0.5) * 112.0, s.y - 100.0)


## How much of a supply is left (water-based tools: always 1).
func amount(item: String) -> int:
	return int(supplies.get(item, 0)) if item in COUNTED else 1


## Shake something on the shelf: it's empty.
func shake(item: String) -> void:
	shake_item = item
	_shake = 0.5


func pocket_center() -> Vector2:
	var s := screen_size()
	return Vector2(s.x - 66.0, s.y * 0.42)


func trash_center() -> Vector2:
	var s := screen_size()
	return Vector2(s.x * 0.5, s.y - 300.0)


func glow_center() -> Vector2:
	match glow:
		"": return Vector2.INF
		"pocket": return pocket_center()
		"bin": return trash_center()
		"play": return play_rect().get_center()
		"map": return map_rect().get_center()
	var i: int = (TOOLS + SHOWN).find(glow)
	return tool_center(i) if i >= 0 else Vector2.INF


func play_rect() -> Rect2:
	return Rect2(screen_size().x - 104.0, 24.0, 80.0, 80.0)


func map_rect() -> Rect2:
	return Rect2(screen_size().x - 200.0, 24.0, 80.0, 80.0)


func reset_rect() -> Rect2:
	return Rect2(28.0, 28.0, 64.0, 64.0)


func over_pocket(p: Vector2) -> bool:
	return p.distance_to(pocket_center()) < 90.0


## What's under a screen point: "tool:<name>", "head", "spare", "play",
## "reset" or "".
func hit(p: Vector2) -> String:
	if play_rect().grow(10).has_point(p):
		return "play"
	if map_rect().grow(6).has_point(p):
		return "map"
	if reset_rect().grow(10).has_point(p):
		return "reset"
	if pocket_head and p.distance_to(pocket_center()) < 70.0:
		return "head"
	if spare_visible and p.distance_to(pocket_center()) < 70.0:
		return "spare"
	for i in TOOLS.size():
		if p.distance_to(tool_center(i)) < 70.0:
			return "tool:" + TOOLS[i]
	return ""


func over_trash(p: Vector2) -> bool:
	return trash_visible and p.distance_to(trash_center()) < 120.0


var _key = null


## Redraw only when something shown has changed (an idle shelf costs nothing).
func _process(delta: float) -> void:
	_trash_pop = move_toward(_trash_pop, 1.0 if trash_visible else 0.0, delta * 5.0)
	_shake = maxf(0.0, _shake - delta)
	_bake_static()
	if glow != "":
		_glow_t += delta
		queue_redraw()
	var key := [glow, pocket_head.get_instance_id() if pocket_head else 0, pocket_open, spare_count, spare_visible, spare_part.get_instance_id() if spare_part else 0, trash_visible,
		trash_hover, held_tool, Vector2i(tool_pos) if held_tool != "" else Vector2i.ZERO,
		reset_armed, play_label, screen_size(), int(_trash_pop * 20.0)]
	if key != _key:
		_key = key
		queue_redraw()


## The shelf and the buttons rarely change: each is baked into one
## picture (one draw call), drawn again only when something on it does.
var _shelf: Baked
var _buttons: Baked
var _baked_for := Vector2.ZERO


func _bake_static() -> void:
	var s := screen_size()
	if s != _baked_for:
		_baked_for = s
		for b in [_shelf, _buttons]:
			if b:
				b.queue_free()
		_shelf = Baked.new()
		add_child(_shelf)
		_shelf.setup(Rect2(0, s.y - 200.0, s.x, 200.0), _paint_shelf, 1.0)
		_buttons = Baked.new()
		add_child(_buttons)
		_buttons.setup(Rect2(0, 0, s.x, 110.0), _paint_buttons, 1.0)
		# Under what's drawn live (the pocket, the bin, the tool in hand).
		_shelf.show_behind_parent = true
		_buttons.show_behind_parent = true
	_shelf.refresh([supplies.duplicate(), held_tool, shake_item if _shake > 0.0 else "", int(_shake * 30.0)])
	_buttons.refresh([play_label, reset_armed])


func _paint_shelf(ci: CanvasItem) -> void:
	var s := screen_size()
	var font := ThemeDB.fallback_font
	# Tray: a wooden shelf along the bottom
	ci.draw_rect(Rect2(0, s.y - 190.0, s.x, 190.0), Color(0.33, 0.22, 0.14))
	ci.draw_rect(Rect2(0, s.y - 196.0, s.x, 10.0), Color(0.48, 0.33, 0.2))
	var items: Array = TOOLS + SHOWN
	for i in items.size():
		var tool_name: String = items[i]
		var c := tool_center(i)
		if tool_name == shake_item and _shake > 0.0:
			c.x += sin(_shake * 40.0) * 8.0
		if held_tool == tool_name:
			ci.draw_arc(c, 48.0, 0, TAU, 24, Color(1, 1, 1, 0.25), 3.0, true)
		else:
			draw_tool(ci, tool_name, c + Vector2(0, -12), 0.85, amount(tool_name))
		if Flags.has("hints"):
			ci.draw_string(font, c + Vector2(-56, 62), LABELS[tool_name], HORIZONTAL_ALIGNMENT_CENTER, 112, 19, Color(0.95, 0.88, 0.75, 0.9))


func _paint_buttons(ci: CanvasItem) -> void:
	Icons.button(ci, play_rect().get_center(), Color(0.42, 0.55, 0.38), "ball")
	Icons.button(ci, map_rect().get_center(), Color(0.5, 0.42, 0.3), "map")
	Icons.button(ci, reset_rect().get_center(), Color(0.55, 0.3, 0.27) if reset_armed else Color(0.3, 0.24, 0.2, 0.85),
		"again_armed" if reset_armed else "again", 32.0)


func _draw() -> void:
	# What could be used next, glowing softly (0032).
	var gc := glow_center()
	if gc != Vector2.INF:
		var pulse := 0.5 + 0.5 * sin(_glow_t * 4.0)
		draw_circle(gc, 54.0 + 6.0 * pulse, Color(1.0, 0.9, 0.55, 0.12 + 0.14 * pulse))
		draw_arc(gc, 54.0 + 6.0 * pulse, 0, TAU, 32, Color(1.0, 0.92, 0.55, 0.5 + 0.4 * pulse), 4.0, true)
	# Pocket with spare parts and heads in it
	if (spare_visible and spare_part) or pocket_head or pocket_open:
		var pc := pocket_center()
		var glow := 1.4 if pocket_open else 0.6
		draw_circle(pc, 76.0, Color(1.0, 0.9, 0.6, 0.12 * glow))
		var pocket := PackedVector2Array([pc + Vector2(-56, -10), pc + Vector2(56, -10), pc + Vector2(48, 66), pc + Vector2(-48, 66)])
		var trim := Color(0.94, 0.86, 0.75)
		if pocket_head:
			LooseHead.draw_head(self, pocket_head, Transform2D(0.0, Vector2(0.42, 0.42), 0.0, pc + Vector2(0, -30)))
		elif spare_visible and spare_part:
			for k in range(mini(spare_count, 3) - 1, -1, -1):
				draw_set_transform(pc + Vector2(-10 * k, -6 - 4 * k), 0.15 - 0.2 * k, Vector2(0.7, 0.7))
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
		draw_tool(self, held_tool, tool_pos, 1.1, amount(held_tool))


func _button(ci: CanvasItem, r: Rect2, text: String, col: Color) -> void:
	ci.draw_rect(r, col)
	ci.draw_rect(r, col.darkened(0.4), false, 3.0)
	var font := ThemeDB.fallback_font
	ci.draw_string(font, Vector2(r.position.x, r.position.y + r.size.y * 0.5 + 9.0), text, HORIZONTAL_ALIGNMENT_CENTER, r.size.x, 26, Color(1, 0.96, 0.88))


## Draw a shelf item; `left` shows how much of it remains (0 is empty).
func draw_tool(ci: CanvasItem, tool_name: String, c: Vector2, k: float, left := 1) -> void:
	ci.draw_set_transform(c, 0, Vector2(k, k))
	match tool_name:
		"sponge":
			ci.draw_rect(Rect2(-40, -26, 80, 52), Color(0.96, 0.82, 0.32))
			ci.draw_rect(Rect2(-40, -26, 80, 52), Color(0.7, 0.55, 0.18), false, 3.0)
			for h in [Vector2(-22, -10), Vector2(8, -14), Vector2(24, 6), Vector2(-6, 10), Vector2(-26, 12)]:
				ci.draw_circle(h, 5.0, Color(0.85, 0.66, 0.22))
		"towel":
			ci.draw_rect(Rect2(-42, -30, 84, 60), Color(0.88, 0.62, 0.66))
			for y in [-14.0, 0.0, 14.0]:
				ci.draw_line(Vector2(-42, y), Vector2(42, y), Color(0.97, 0.9, 0.9), 4.0)
			ci.draw_rect(Rect2(-42, -30, 84, 60), Color(0.6, 0.38, 0.42), false, 3.0)
		"stuffing":
			# A box with as many tufts as handfuls left (up to six shown).
			for t in mini(left, 6):
				PlushArt.fluff(ci, Vector2(-22 + (t % 3) * 22, -12 - (t / 3) * 14), 13.0)
			ci.draw_colored_polygon(PackedVector2Array([Vector2(-34, -6), Vector2(34, -6), Vector2(40, 34), Vector2(-40, 34)]), Color(0.66, 0.53, 0.36))
			ci.draw_line(Vector2(-36, -2), Vector2(36, -2), Color(0.45, 0.34, 0.22), 4.0)
		"oil":
			ci.draw_rect(Rect2(-28, -14, 56, 46), Color(0.55, 0.58, 0.6))
			# A window showing how much oil is left.
			ci.draw_rect(Rect2(-14, -6, 12, 32), Color(0.2, 0.2, 0.22))
			var lvl := clampf(left / 3.0, 0.0, 1.0) * 30.0
			ci.draw_rect(Rect2(-13, 25 - lvl, 10, lvl), Color(0.9, 0.72, 0.25))
			ci.draw_rect(Rect2(-28, -14, 56, 46), Color(0.3, 0.32, 0.35), false, 3.0)
			ci.draw_line(Vector2(20, -10), Vector2(46, -38), Color(0.45, 0.48, 0.5), 7.0)
			ci.draw_arc(Vector2(-28, 8), 16.0, PI * 0.5, PI * 1.5, 10, Color(0.3, 0.32, 0.35), 5.0)
			ci.draw_circle(Vector2(48, -36), 4.0, Color(0.9, 0.75, 0.3))
		"spray":
			# A green spray bottle; the liquid shows how much is left.
			ci.draw_rect(Rect2(-20, -16, 40, 50), Color(0.82, 0.9, 0.84))
			var lvl := clampf(left / 3.0, 0.0, 1.0) * 46.0
			ci.draw_rect(Rect2(-18, 32 - lvl, 36, lvl), Color(0.42, 0.7, 0.42))
			ci.draw_rect(Rect2(-20, -16, 40, 50), Color(0.3, 0.42, 0.32), false, 3.0)
			ci.draw_rect(Rect2(-8, -32, 16, 16), Color(0.3, 0.42, 0.32))
			ci.draw_rect(Rect2(8, -30, 18, 6), Color(0.3, 0.42, 0.32))
		"thread":
			# One spool per two lengths of thread; the last may be half wound.
			var spools := ceili(left / 2.0)
			for t in mini(spools, 3):
				var x := -30.0 + t * 30.0
				var full := 1.0 if (t < spools - 1 or left % 2 == 0) else 0.5
				ci.draw_rect(Rect2(x - 12, -18, 24, 40), Color(0.6, 0.45, 0.3))
				ci.draw_rect(Rect2(x - 10 * full, -12, 20 * full, 28), [Color(0.75, 0.2, 0.2), Color(0.25, 0.45, 0.7), Color(0.85, 0.7, 0.25)][t])
				ci.draw_rect(Rect2(x - 14, -22, 28, 6), Color(0.5, 0.36, 0.22))
				ci.draw_rect(Rect2(x - 14, 20, 28, 6), Color(0.5, 0.36, 0.22))
			if spools == 0:
				ci.draw_rect(Rect2(-14, -22, 28, 6), Color(0.5, 0.36, 0.22, 0.5))
				ci.draw_rect(Rect2(-14, 20, 28, 6), Color(0.5, 0.36, 0.22, 0.5))
				ci.draw_rect(Rect2(-5, -16, 10, 36), Color(0.6, 0.45, 0.3, 0.5))
	ci.draw_set_transform(Vector2.ZERO, 0, Vector2.ONE)
