class_name PlushArt
extends RefCounted
## Drawing helpers for stitched, stuffed things. Everything is drawn in
## code from data, so new creatures and parts need no new image files
## (Q-006: Mikey wants much of the art generated, like Driftloom's covers).

const THREAD := Color(0.62, 0.16, 0.16)
const FLUFF := Color(0.97, 0.95, 0.9)
const FLUID := Color(0.2, 0.82, 0.76)
const OUTLINE_DARKEN := 0.55
const ROT_TINT := Color(0.33, 0.36, 0.18)
const MOLD := Color(0.62, 0.72, 0.42)


static func ellipse_points(c: Vector2, rx: float, ry: float, n := 40) -> PackedVector2Array:
	var pts := PackedVector2Array()
	for i in n:
		var a := TAU * i / n
		pts.append(c + Vector2(cos(a) * rx, sin(a) * ry))
	return pts


static func ellipse(ci: CanvasItem, c: Vector2, rx: float, ry: float, col: Color, outline := true) -> void:
	var pts := ellipse_points(c, rx, ry)
	ci.draw_colored_polygon(pts, col)
	if outline:
		var closed := pts.duplicate()
		closed.append(pts[0])
		ci.draw_polyline(closed, col.darkened(OUTLINE_DARKEN), 3.0, true)


## A dashed line of stitches along a polyline.
static func seam(ci: CanvasItem, pts: PackedVector2Array, col: Color, dash := 9.0, width := 2.5) -> void:
	for i in pts.size() - 1:
		var a := pts[i]
		var b := pts[i + 1]
		var d := a.distance_to(b)
		var steps := int(d / dash)
		for s in range(0, steps, 2):
			var p0 := a.lerp(b, float(s) / maxf(1, steps))
			var p1 := a.lerp(b, float(mini(s + 1, steps)) / maxf(1, steps))
			ci.draw_line(p0, p1, col, width, true)


## Zigzag surgical stitches across a seam from a to b.
static func zigzag(ci: CanvasItem, a: Vector2, b: Vector2, height := 9.0, count := 6, col := THREAD) -> void:
	var dir := (b - a)
	var n := dir.orthogonal().normalized() * height
	var prev := a + n
	for i in range(1, count + 1):
		var p := a + dir * (float(i) / count) + (n if i % 2 == 0 else -n)
		ci.draw_line(prev, p, col, 3.0, true)
		prev = p


## A soft clump of fluff.
static func fluff(ci: CanvasItem, p: Vector2, r: float, alpha := 1.0) -> void:
	var c := FLUFF
	c.a = alpha * 0.55
	ci.draw_circle(p + Vector2(-r * 0.4, r * 0.1), r * 0.75, c)
	ci.draw_circle(p + Vector2(r * 0.45, 0), r * 0.7, c)
	c.a = alpha
	ci.draw_circle(p + Vector2(0, -r * 0.25), r * 0.8, c)
	ci.draw_circle(p + Vector2(r * 0.1, r * 0.3), r * 0.6, c)


## A fabric limb from `a` (where it meets the body) to `b` (the paw).
static func limb(ci: CanvasItem, a: Vector2, b: Vector2, w: float, part: PartData, trim: Color, wet := 0.0) -> void:
	if a.distance_to(b) < 1.0:
		b = a + Vector2(0, 1)
	var dir := (b - a).normalized()
	var n := dir.orthogonal() * (w * 0.5)
	var base := fabric_color(part.fabric, wet, part.rot)
	var body := PackedVector2Array([a + n, b + n * 0.92, b - n * 0.92, a - n])
	ci.draw_colored_polygon(body, base)
	ci.draw_circle(a, w * 0.5, base)
	ci.draw_circle(b, w * 0.46, base)
	var edge := base.darkened(OUTLINE_DARKEN)
	ci.draw_line(a + n, b + n * 0.92, edge, 3.0, true)
	ci.draw_line(a - n, b - n * 0.92, edge, 3.0, true)
	ci.draw_arc(b, w * 0.46, dir.angle() - PI * 0.5, dir.angle() + PI * 0.5, 16, edge, 3.0, true)
	# Pattern
	var length := a.distance_to(b)
	match part.pattern:
		1:
			var s := 0.22
			while s < 0.92:
				var c := a.lerp(b, s)
				ci.draw_line(c + n * 0.92, c - n * 0.92, base.lightened(0.28), 6.0, true)
				s += 18.0 / length
		2:
			var s := 0.2
			var flip := 1.0
			while s < 0.9:
				ci.draw_circle(a.lerp(b, s) + n * 0.35 * flip, w * 0.08, base.lightened(0.35))
				s += 16.0 / length
				flip = -flip
		3:
			var c := a.lerp(b, 0.5)
			ci.draw_colored_polygon(PackedVector2Array([c + n * 0.7 - dir * 12, c + n * 0.7 + dir * 14, c - n * 0.5 + dir * 14, c - n * 0.5 - dir * 12]), base.lightened(0.2))
	# Paw pad
	ellipse(ci, b + dir * 2, w * 0.3, w * 0.22, trim, false)
	# Seam down the side
	seam(ci, PackedVector2Array([a.lerp(b, 0.15) + n * 0.55, b + n * 0.4]), edge.lerp(base, 0.4), 7.0, 2.0)
	# Rot and mold
	if part.rot > 0.0:
		var rc := ROT_TINT
		rc.a = 0.55 * part.rot
		ci.draw_circle(a.lerp(b, 0.45) + n * 0.2, w * 0.38, rc)
		ci.draw_circle(a.lerp(b, 0.7) - n * 0.25, w * 0.3, rc)
		for m in part.mold:
			var mp: Vector2 = a.lerp(b, float(m[0])) + n * 2.0 * float(m[1])
			var mr: float = float(m[2]) * w
			var mc := MOLD
			mc.a = 0.85 * part.rot
			ci.draw_circle(mp, mr, mc)
			for k in 5:
				var ang := TAU * k / 5.0 + mp.x
				ci.draw_circle(mp + Vector2(cos(ang), sin(ang)) * mr * 0.9, mr * 0.35, mc.lightened(0.3))
	if part.torn:
		var tc := a.lerp(b, 0.55)
		var slit := PackedVector2Array([tc - n * 0.6, tc - n * 0.2 + dir * 5, tc + n * 0.1 - dir * 4, tc + n * 0.5 + dir * 3])
		ci.draw_polyline(slit, Color(0.12, 0.08, 0.06), 6.0, true)
		fluff(ci, tc - n * 0.15 + dir * 4, w * 0.18)
		fluff(ci, tc + n * 0.3, w * 0.14)
	if part.stitched:
		zigzag(ci, a - n * 0.9 + dir * 6, a + n * 0.9 + dir * 6, 7.0, 7)


static func fabric_color(base: Color, wet: float, rot := 0.0) -> Color:
	var c := base
	if rot > 0.0:
		c = c.lerp(ROT_TINT, rot * 0.35)
	return c.darkened(wet * 0.28)


## A button eye: dark disc, lighter rim, four holes, thread through
## them. `lid` 0 open .. 1 closed (a fabric lid drawn over the button).
static func button_eye(ci: CanvasItem, p: Vector2, r: float, col: Color, lid: float, lid_col: Color, stitched := false, happy := false) -> void:
	ci.draw_circle(p, r, col.lightened(0.18))
	ci.draw_circle(p, r * 0.82, col)
	for h in [Vector2(-1, -1), Vector2(1, -1), Vector2(-1, 1), Vector2(1, 1)]:
		ci.draw_circle(p + h * r * 0.28, r * 0.12, col.lightened(0.45))
	ci.draw_line(p + Vector2(-1, -1) * r * 0.28, p + Vector2(1, 1) * r * 0.28, THREAD.lightened(0.2), 2.0, true)
	ci.draw_line(p + Vector2(1, -1) * r * 0.28, p + Vector2(-1, 1) * r * 0.28, THREAD.lightened(0.2), 2.0, true)
	ci.draw_circle(p + Vector2(-r * 0.35, -r * 0.4), r * 0.16, Color(1, 1, 1, 0.45))
	if stitched:
		ci.draw_line(p + Vector2(-r, -r) * 1.05, p + Vector2(r, r) * 1.05, THREAD, 3.0, true)
		ci.draw_line(p + Vector2(r, -r) * 1.05, p + Vector2(-r, r) * 1.05, THREAD, 3.0, true)
	if lid > 0.02:
		var top := p.y - r * 1.15
		var bottom := lerpf(top, p.y + r * 1.15, clampf(lid, 0.0, 1.0))
		var pts := PackedVector2Array([Vector2(p.x - r * 1.2, top), Vector2(p.x + r * 1.2, top), Vector2(p.x + r * 1.2, bottom), Vector2(p.x - r * 1.2, bottom)])
		ci.draw_colored_polygon(pts, lid_col)
		var edge := lid_col.darkened(0.5)
		if happy:
			ci.draw_arc(Vector2(p.x, bottom + r * 0.3), r * 0.9, PI * 1.15, PI * 1.85, 12, edge, 3.0, true)
		else:
			ci.draw_arc(Vector2(p.x, bottom - r * 0.6), r * 1.0, PI * 0.15, PI * 0.85, 12, edge, 3.0, true)
