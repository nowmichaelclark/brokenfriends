class_name MapHud
extends Node2D
## The map's buttons and the picture of the chosen route: who goes (the
## friend from the tent), the chance of coming back well (a pie: the
## green part), and how long it takes (a clock face filling up to four
## hours). No numbers to read (decision 0032).

var map: Node2D
var shake := 0.0


func screen_size() -> Vector2:
	return get_viewport_rect().size


func back_rect() -> Rect2:
	return Rect2(24.0, 24.0, 80.0, 80.0)


func go_center() -> Vector2:
	var s := screen_size()
	return Vector2(s.x - 110.0, s.y - 110.0)


func hit(p: Vector2) -> String:
	if back_rect().grow(10).has_point(p):
		return "back"
	if map.place != "" and p.distance_to(go_center()) < 80.0:
		return "go"
	return ""


func _process(delta: float) -> void:
	if shake > 0.0:
		shake = maxf(0.0, shake - delta)
		queue_redraw()


func _draw() -> void:
	var s := screen_size()
	Icons.button(self, back_rect().get_center(), Color(0.42, 0.55, 0.38), "tent")
	if map.place == "":
		return
	# The panel along the bottom.
	draw_rect(Rect2(0, s.y - 220.0, s.x, 220.0), Color(0.33, 0.22, 0.14, 0.92))
	var f := Save.friend
	if f:
		LooseHead.draw_head(self, f, Transform2D(0.0, Vector2(0.55, 0.55), 0.0, Vector2(110, s.y - 112.0)))
	# Chance: a pie, green for coming back well.
	var c: float = map.chance()
	var pc := Vector2(s.x * 0.5 - 60.0, s.y - 110.0)
	draw_circle(pc, 56.0, Color(0.62, 0.25, 0.22))
	var pie := PackedVector2Array([pc])
	for i in 41:
		var a := -PI * 0.5 + TAU * c * i / 40.0
		pie.append(pc + Vector2(cos(a), sin(a)) * 56.0)
	if c > 0.0:
		draw_colored_polygon(pie, Color(0.45, 0.68, 0.38))
	draw_arc(pc, 56.0, 0, TAU, 40, Color(0.2, 0.14, 0.1), 3.0, true)
	# Time: a clock face, filled by how long the trip takes (up to 4 h).
	var minutes: float = Quests.PLACES[map.place]["routes"][map.route]["minutes"]
	var cc := Vector2(s.x * 0.5 + 80.0, s.y - 110.0)
	draw_circle(cc, 44.0, Color(0.95, 0.9, 0.8))
	var fill := clampf(minutes / 240.0, 0.03, 1.0)
	var wedge := PackedVector2Array([cc])
	for i in 41:
		var a := -PI * 0.5 + TAU * fill * i / 40.0
		wedge.append(cc + Vector2(cos(a), sin(a)) * 40.0)
	draw_colored_polygon(wedge, Color(0.55, 0.62, 0.78))
	draw_arc(cc, 44.0, 0, TAU, 40, Color(0.2, 0.14, 0.1), 3.0, true)
	draw_line(cc, cc + Vector2(0, -30), Color(0.2, 0.14, 0.1), 3.0)
	# Go: footprints, dim if this friend can't be sent.
	var gc := go_center() + Vector2(sin(shake * 40.0) * 8.0 if shake > 0.0 else 0.0, 0)
	var ok: bool = map.can_send()
	draw_circle(gc, 70.0, Color(0.42, 0.55, 0.38, 1.0 if ok else 0.35))
	for k in 2:
		var at := gc + Vector2(-18 + k * 36, 16 - k * 28)
		PlushArt.ellipse(self, at, 13.0, 18.0, Color(0.95, 0.92, 0.82, 1.0 if ok else 0.4), false)
		for toe in 3:
			draw_circle(at + Vector2(-8 + toe * 8, -22), 4.5, Color(0.95, 0.92, 0.82, 1.0 if ok else 0.4))
