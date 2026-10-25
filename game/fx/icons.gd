class_name Icons
extends RefCounted
## Little pictures in place of words on buttons (decision 0032: the
## game plays without reading). Each is drawn around `c`, about 64 units
## across.

const INK := Color(1.0, 0.96, 0.88)


## A round button with an icon on it.
static func button(ci: CanvasItem, c: Vector2, col: Color, icon: String, r := 40.0) -> void:
	ci.draw_circle(c, r, col)
	ci.draw_arc(c, r, 0, TAU, 32, col.darkened(0.45), 3.0, true)
	draw(ci, icon, c, r / 40.0)


static func draw(ci: CanvasItem, icon: String, c: Vector2, k := 1.0) -> void:
	match icon:
		"ball":
			ci.draw_circle(c, 20.0 * k, Color(0.92, 0.84, 0.62))
			ci.draw_arc(c, 13.0 * k, 0, TAU, 20, Color(0.82, 0.32, 0.28), 6.0 * k, true)
			ci.draw_line(c + Vector2(-20, 0) * k, c + Vector2(20, 0) * k, Color(0.3, 0.5, 0.7), 4.0 * k, true)
		"map":
			var pts := PackedVector2Array()
			for i in 4:
				pts.append(c + Vector2(-24 + i * 16, -16 + (i % 2) * 6) * k)
			for i in range(3, -1, -1):
				pts.append(c + Vector2(-24 + i * 16, 18 + (i % 2) * 6) * k)
			ci.draw_colored_polygon(pts, Color(0.92, 0.85, 0.66))
			ci.draw_polyline(pts + PackedVector2Array([pts[0]]), Color(0.4, 0.3, 0.2), 2.5, true)
			for i in range(1, 3):
				ci.draw_line(c + Vector2(-24 + i * 16, -16 + (i % 2) * 6) * k, c + Vector2(-24 + i * 16, 18 + (i % 2) * 6) * k, Color(0.4, 0.3, 0.2, 0.6), 2.0)
			ci.draw_line(c + Vector2(-14, 8) * k, c + Vector2(10, -4) * k, Color(0.75, 0.3, 0.25), 3.0 * k, true)
		"tent":
			ci.draw_colored_polygon(PackedVector2Array([c + Vector2(-26, 18) * k, c + Vector2(0, -22) * k, c + Vector2(26, 18) * k]), Color(0.86, 0.75, 0.56))
			ci.draw_colored_polygon(PackedVector2Array([c + Vector2(-6, 18) * k, c + Vector2(0, 0) * k, c + Vector2(6, 18) * k]), Color(0.36, 0.28, 0.2))
		"needle":
			# Needle and thread: the surgery table.
			ci.draw_line(c + Vector2(-18, 20) * k, c + Vector2(18, -20) * k, Color(0.88, 0.9, 0.94), 5.0 * k, true)
			ci.draw_circle(c + Vector2(15, -17) * k, 3.0 * k, Color(0.3, 0.3, 0.35))
			ci.draw_polyline(PackedVector2Array([c + Vector2(15, -17) * k, c + Vector2(24, -4) * k, c + Vector2(10, 8) * k, c + Vector2(18, 20) * k]), Color(0.85, 0.25, 0.25), 3.0 * k, true)
		"sponge":
			# A sponge: the grooming screen.
			ci.draw_rect(Rect2(c + Vector2(-24, -16) * k, Vector2(48, 32) * k), Color(0.96, 0.82, 0.32))
			for h in [Vector2(-12, -6), Vector2(6, -8), Vector2(14, 6), Vector2(-4, 6)]:
				ci.draw_circle(c + h * k, 4.0 * k, Color(0.8, 0.62, 0.2))
		"gear":
			# The dev menu (0045).
			for i in 8:
				var a := TAU * i / 8.0
				ci.draw_line(c + Vector2(cos(a), sin(a)) * 12.0 * k, c + Vector2(cos(a), sin(a)) * 24.0 * k, INK, 7.0 * k)
			ci.draw_circle(c, 16.0 * k, INK)
			ci.draw_circle(c, 7.0 * k, Color(0.3, 0.3, 0.32))
		"room":
			# The play room: a little house with a ball in its door.
			ci.draw_rect(Rect2(c + Vector2(-22, -4) * k, Vector2(44, 26) * k), Color(0.86, 0.75, 0.56))
			ci.draw_colored_polygon(PackedVector2Array([c + Vector2(-28, -2) * k, c + Vector2(0, -26) * k, c + Vector2(28, -2) * k]), Color(0.75, 0.36, 0.3))
			ci.draw_circle(c + Vector2(0, 12) * k, 8.0 * k, Color(0.3, 0.5, 0.7))
		"again":
			# A circling arrow: start over.
			ci.draw_arc(c, 18.0 * k, -PI * 0.15, PI * 1.45, 24, INK, 5.0 * k, true)
			var tip := c + Vector2(cos(-PI * 0.15), sin(-PI * 0.15)) * 18.0 * k
			ci.draw_colored_polygon(PackedVector2Array([tip + Vector2(-8, -10) * k, tip + Vector2(10, -4) * k, tip + Vector2(2, 10) * k]), INK)
		"again_armed":
			draw(ci, "again", c, k)
			ci.draw_circle(c + Vector2(18, -18) * k, 8.0 * k, Color(1.0, 0.85, 0.3))
