class_name FriendArt
extends RefCounted
## Drawing a friend's generated look (decision 0037): head shape, ears,
## muzzle, nose, cheeks, belly, tail, from the front or the back
## (0031). Shared by the friend on screen, heads in the hand or pocket,
## and friends walking on the map, so a look is drawn the same way
## everywhere. Positions are in the head's or body's own units.

const HEAD_RX := 100.0
const HEAD_RY := 94.0
const EYE_R := 21.0


static func head_rx(f: FriendData) -> float:
	return HEAD_RX * float(f.hl("w"))


static func head_ry(f: FriendData) -> float:
	return HEAD_RY * float(f.hl("h"))


static func eye_pos(f: FriendData, side: float) -> Vector2:
	return Vector2(side * float(f.hl("eye_gap")), float(f.hl("eye_y")))


static func eye_radius(f: FriendData) -> float:
	return EYE_R * float(f.hl("eye_size"))


static func mouth_pos(f: FriendData) -> Vector2:
	var m := float(f.hl("muzzle"))
	return Vector2(0, 40.0 * float(f.hl("h")) + (10.0 * m if m > 0.0 else 0.0))


## Where an ear joins the head, and the way it points at rest (radians,
## 0 is straight up).
static func ear_base(f: FriendData, side: float) -> Vector2:
	var a := deg_to_rad(_ear_deg(f)) * side
	return Vector2(sin(a) * head_rx(f) * 0.86, -cos(a) * head_ry(f) * 0.86)


static func ear_angle(f: FriendData, side: float) -> float:
	match str(f.hl("ears")):
		"floppy": return side * PI * 0.62
		"long": return side * 0.12
		"antennae": return side * 0.3
		"horns": return side * 0.35
	return side * deg_to_rad(_ear_deg(f)) * 0.55


static func _ear_deg(f: FriendData) -> float:
	match str(f.hl("ears")):
		"long", "floppy": return 22.0
		"pointy": return 38.0
		"horns": return 32.0
		"antennae": return 18.0
	return 44.0


## How long an ear is, for hit tests and swinging.
static func ear_length(f: FriendData) -> float:
	var k := float(f.hl("ear_size"))
	match str(f.hl("ears")):
		"long", "floppy": return 140.0 * k
		"pointy": return 62.0 * k
		"horns": return 56.0 * k
		"antennae": return 70.0 * k
		"none": return 0.0
	return 50.0 * k


## One ear, its base at (0, 0), pointing up (-y).
static func draw_ear(ci: CanvasItem, f: FriendData, fabric: Color, trim: Color, back := false) -> void:
	var k := float(f.hl("ear_size"))
	var inner := fabric.darkened(0.15) if back else trim
	var edge := fabric.darkened(PlushArt.OUTLINE_DARKEN)
	match str(f.hl("ears")):
		"round":
			PlushArt.ellipse(ci, Vector2(0, -20 * k), 34.0 * k, 34.0 * k, fabric)
			PlushArt.ellipse(ci, Vector2(0, -20 * k), 19.0 * k, 19.0 * k, inner, false)
		"long", "floppy":
			PlushArt.ellipse(ci, Vector2(0, -70 * k), 24.0 * k, 72.0 * k, fabric)
			PlushArt.ellipse(ci, Vector2(0, -68 * k), 12.0 * k, 54.0 * k, inner, false)
		"pointy":
			var outer := PackedVector2Array([Vector2(-28 * k, 6), Vector2(0, -62 * k), Vector2(28 * k, 6)])
			ci.draw_colored_polygon(outer, fabric)
			ci.draw_polyline(PackedVector2Array([outer[0], outer[1], outer[2]]), edge, 3.0, true)
			if not back:
				ci.draw_colored_polygon(PackedVector2Array([Vector2(-14 * k, 0), Vector2(0, -40 * k), Vector2(14 * k, 0)]), inner)
		"horns":
			var horn := Color(0.92, 0.86, 0.7)
			var pts := PackedVector2Array([Vector2(-14 * k, 4), Vector2(-10 * k, -30 * k), Vector2(6 * k, -56 * k), Vector2(4 * k, -30 * k), Vector2(14 * k, 4)])
			ci.draw_colored_polygon(pts, horn)
			ci.draw_polyline(pts, horn.darkened(0.45), 3.0, true)
			for r in 3:
				var y := -8.0 - r * 12.0
				ci.draw_line(Vector2(-11 * k, y * k), Vector2(11 * k, y * k), horn.darkened(0.2), 2.0, true)
		"antennae":
			ci.draw_line(Vector2.ZERO, Vector2(0, -62 * k), edge, 5.0, true)
			ci.draw_circle(Vector2(0, -66 * k), 11.0 * k, trim)
			ci.draw_arc(Vector2(0, -66 * k), 11.0 * k, 0, TAU, 16, edge, 2.5, true)


## The head itself (no ears, no eyes or mouth): its shape, muzzle,
## nose, cheeks and patch; from the back, a seam instead of a face.
static func draw_head(ci: CanvasItem, f: FriendData, at: Vector2, fabric: Color, trim: Color, back := false) -> void:
	var rx := head_rx(f)
	var ry := head_ry(f)
	PlushArt.ellipse(ci, at, rx, ry, fabric)
	var seam_col := fabric.darkened(0.35)
	if back:
		PlushArt.seam(ci, PackedVector2Array([at + Vector2(0, -ry + 8), at + Vector2(0, ry - 8)]), seam_col)
		return
	PlushArt.seam(ci, PackedVector2Array([at + Vector2(0, -ry + 8), at + Vector2(0, -ry * 0.32)]), seam_col)
	if f.hl("patch"):
		PlushArt.ellipse(ci, at + Vector2(-rx * 0.38, -ry * 0.42), rx * 0.3, ry * 0.24, fabric.lightened(0.25), false)
		PlushArt.seam(ci, PackedVector2Array([at + Vector2(-rx * 0.66, -ry * 0.42), at + Vector2(-rx * 0.1, -ry * 0.42)]), seam_col, 6.0, 1.5)
	if f.hl("cheeks"):
		for side in [-1.0, 1.0]:
			ci.draw_circle(at + Vector2(side * rx * 0.6, ry * 0.32), 13.0, Color(0.95, 0.55, 0.6, 0.45))
	var m := float(f.hl("muzzle"))
	var h := float(f.hl("h"))
	if m > 0.0:
		PlushArt.ellipse(ci, at + Vector2(0, 38 * h), 48.0 * m, 33.0 * m, trim, false)
	var nose := at + Vector2(0, 24 * h)
	match str(f.hl("nose")):
		"button":
			PlushArt.ellipse(ci, nose, 12.0, 8.0, Color(0.22, 0.15, 0.13), false)
		"heart":
			var c := Color(0.85, 0.45, 0.5)
			ci.draw_circle(nose + Vector2(-4, -2), 5.0, c)
			ci.draw_circle(nose + Vector2(4, -2), 5.0, c)
			ci.draw_colored_polygon(PackedVector2Array([nose + Vector2(-9, 0), nose + Vector2(9, 0), nose + Vector2(0, 9)]), c)
		"triangle":
			ci.draw_colored_polygon(PackedVector2Array([nose + Vector2(-9, -4), nose + Vector2(9, -4), nose + Vector2(0, 6)]), Color(0.8, 0.45, 0.5))


## A head on its own (in the hand, the pocket, on the map): ears, head,
## open button eyes, a small mouth, and fluff where the neck tore when
## `torn`.
static func draw_loose_head(ci: CanvasItem, f: FriendData, at: Vector2, torn := true) -> void:
	var fabric := f.head_color()
	if torn:
		for t in 3:
			PlushArt.fluff(ci, at + Vector2(-30 + t * 30, head_ry(f) - 2.0), 16.0)
	for side in [-1.0, 1.0]:
		if str(f.hl("ears")) == "none":
			break
		var base := at + ear_base(f, side)
		var tr := Transform2D(ear_angle(f, side), base)
		ci.draw_set_transform_matrix(tr)
		draw_ear(ci, f, fabric, f.trim)
		ci.draw_set_transform_matrix(Transform2D.IDENTITY)
	draw_head(ci, f, at, fabric, f.trim)
	for slot in ["eye_l", "eye_r"]:
		var e: PartData = f.part(slot)
		var side := -1.0 if slot == "eye_l" else 1.0
		if e:
			PlushArt.button_eye(ci, at + eye_pos(f, side), eye_radius(f), e.fabric, 0.0, fabric, e.stitched)
	ci.draw_arc(at + mouth_pos(f) + Vector2(0, 6), 12.0, PI * 1.1, PI * 1.9, 8, Color(0.25, 0.15, 0.12), 3.0, true)


## The body: its shape and belly from the front; a seam and the tail
## from the back. `s` is the stuffing scale.
static func draw_body(ci: CanvasItem, f: FriendData, s: float, fabric: Color, trim: Color, back := false) -> void:
	var rx := 112.0 * s * float(f.bl("w"))
	var ry := 132.0 * float(f.bl("h"))
	PlushArt.ellipse(ci, Vector2.ZERO, rx, ry, fabric)
	if str(f.bl("plan")) == "ghosty":
		# A scalloped hem, like a plush ghost costume.
		for i in 7:
			var x := -rx * 0.8 + i * rx * 1.6 / 6.0
			ci.draw_circle(Vector2(x, ry * 0.86), rx * 0.13, fabric)
	var seam_col := fabric.darkened(0.35)
	if back:
		PlushArt.seam(ci, PackedVector2Array([Vector2(0, -ry + 10), Vector2(0, ry - 14)]), seam_col)
		if str(f.bl("tail")) != "none":
			_draw_tail(ci, f, Vector2(0, ry * 0.62), fabric, trim)
		return
	var b := float(f.bl("belly"))
	PlushArt.ellipse(ci, Vector2(0, 28), 68.0 * s * float(f.bl("w")) * b, 80.0 * b * float(f.bl("h")), trim, false)
	PlushArt.seam(ci, PackedVector2Array([Vector2(0, -ry + 10), Vector2(0, -60)]), seam_col)


static func _draw_tail(ci: CanvasItem, f: FriendData, at: Vector2, fabric: Color, trim: Color) -> void:
	match str(f.bl("tail")):
		"stub":
			PlushArt.fluff(ci, at, 22.0)
			PlushArt.ellipse(ci, at, 20.0, 18.0, trim.lerp(fabric, 0.3), false)
		"long":
			var pts := PackedVector2Array()
			for i in 9:
				var t := i / 8.0
				pts.append(at + Vector2(sin(t * 3.0) * 40.0 + t * 30.0, t * 90.0 - t * t * 60.0))
			ci.draw_polyline(pts, fabric.darkened(0.2), 18.0, true)
			ci.draw_polyline(pts, fabric, 12.0, true)
		"curly":
			var pts := PackedVector2Array()
			for i in 18:
				var a := i * 0.6
				pts.append(at + Vector2(cos(a), sin(a)) * (8.0 + i * 1.8) + Vector2(i * 1.5, 0))
			ci.draw_polyline(pts, fabric.darkened(0.25), 10.0, true)
