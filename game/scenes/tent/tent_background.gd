class_name TentBackground
extends Node2D
## The inside of the doctor's tent at the starting camp, lit by a
## lantern. Drawn in code; wide enough to fill any phone shape.

var floor_y := 890.0
var _t := 0.0


func _process(delta: float) -> void:
	_t += delta
	queue_redraw()


func _draw() -> void:
	draw_rect(Rect2(-700, -700, 2120, 2800), Color(0.2, 0.14, 0.11))
	# Back wall, lit canvas
	var wall := PackedVector2Array([Vector2(90, 150), Vector2(630, 150), Vector2(820, floor_y), Vector2(-100, floor_y)])
	draw_colored_polygon(wall, Color(0.8, 0.69, 0.52))
	for x in [-20.0, 180.0, 360.0, 540.0, 740.0]:
		var top_x := lerpf(90.0, 630.0, (x + 100.0) / 920.0)
		PlushArt.seam(self, PackedVector2Array([Vector2(top_x, 160), Vector2(x, floor_y - 6)]), Color(0.62, 0.5, 0.36), 12.0, 2.0)
	# Side walls in shadow
	draw_colored_polygon(PackedVector2Array([Vector2(-700, -200), Vector2(90, 150), Vector2(-100, floor_y), Vector2(-700, floor_y)]), Color(0.45, 0.36, 0.26))
	draw_colored_polygon(PackedVector2Array([Vector2(1420, -200), Vector2(630, 150), Vector2(820, floor_y), Vector2(1420, floor_y)]), Color(0.45, 0.36, 0.26))
	draw_colored_polygon(PackedVector2Array([Vector2(-700, -700), Vector2(1420, -700), Vector2(1420, -200), Vector2(630, 150), Vector2(90, 150), Vector2(-700, -200)]), Color(0.36, 0.28, 0.2))
	# Ridge pole
	draw_line(Vector2(360, 150), Vector2(360, floor_y), Color(0.42, 0.29, 0.18), 14.0)
	draw_line(Vector2(354, 150), Vector2(354, floor_y), Color(0.52, 0.37, 0.23), 3.0)
	# Floor and rug
	draw_rect(Rect2(-700, floor_y, 2120, 1400), Color(0.36, 0.27, 0.2))
	PlushArt.ellipse(self, Vector2(360, floor_y + 18), 320.0, 58.0, Color(0.55, 0.28, 0.24), false)
	PlushArt.ellipse(self, Vector2(360, floor_y + 18), 270.0, 44.0, Color(0.68, 0.48, 0.3), false)
	PlushArt.ellipse(self, Vector2(360, floor_y + 18), 210.0, 32.0, Color(0.55, 0.28, 0.24), false)
	# A crate of thread spools: the doctor's supplies
	draw_rect(Rect2(40, floor_y - 120, 130, 120), Color(0.5, 0.36, 0.22))
	draw_rect(Rect2(40, floor_y - 120, 130, 120), Color(0.3, 0.2, 0.12), false, 3.0)
	draw_line(Vector2(40, floor_y - 60), Vector2(170, floor_y - 60), Color(0.3, 0.2, 0.12), 3.0)
	for i in 3:
		var sx := 66.0 + i * 38.0
		draw_rect(Rect2(sx - 12, floor_y - 160, 24, 40), [Color(0.75, 0.2, 0.2), Color(0.25, 0.45, 0.7), Color(0.85, 0.7, 0.25)][i])
		draw_rect(Rect2(sx - 15, floor_y - 164, 30, 7), Color(0.6, 0.45, 0.3))
		draw_rect(Rect2(sx - 15, floor_y - 127, 30, 7), Color(0.6, 0.45, 0.3))
	# Lantern and its glow
	var lp := Vector2(585, 300)
	var flicker := 1.0 + sin(_t * 7.0) * 0.03 + sin(_t * 13.0) * 0.02
	for i in 6:
		var r := (190.0 - i * 28.0) * flicker
		draw_circle(lp, r, Color(1.0, 0.82, 0.45, 0.035 + i * 0.012))
	draw_line(Vector2(585, 150), lp + Vector2(0, -30), Color(0.25, 0.2, 0.15), 3.0)
	draw_rect(Rect2(lp.x - 20, lp.y - 30, 40, 54), Color(1.0, 0.86, 0.5))
	draw_rect(Rect2(lp.x - 20, lp.y - 30, 40, 54), Color(0.3, 0.22, 0.14), false, 4.0)
	draw_rect(Rect2(lp.x - 24, lp.y - 38, 48, 10), Color(0.3, 0.22, 0.14))
