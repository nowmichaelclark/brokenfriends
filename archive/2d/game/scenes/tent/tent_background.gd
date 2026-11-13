class_name TentBackground
extends Node2D
## The inside of the doctor's tent at the starting camp, lit by a
## lantern. Drawn in code; wide enough to fill any phone shape.

var floor_y := 890.0:
	set(v):
		floor_y = v
		queue_redraw()
		if _art:
			_art.refresh(floor_y)

const LANTERN := Vector2(585, 300)
## The part of the tent baked into a picture: everything a phone of any
## shape can see. Beyond it only plain colour is drawn.
const BAKED := Rect2(-400, -300, 1520, 1800)

var _art: Baked
var _glow: Baked
var _t := 0.0


func _ready() -> void:
	# The tent is baked into one picture (one draw call, one layer of
	# pixels to fill) and only re-baked if the floor moves. Beyond its
	# edges only plain colour is drawn.
	_art = Baked.new()
	add_child(_art)
	_art.setup(BAKED, _paint, 1.0)
	_art.refresh(floor_y)
	# The lantern's glow is its own small picture; only its opacity
	# flickers, so the tent itself is never redrawn.
	_glow = Baked.new()
	_glow.position = LANTERN
	add_child(_glow)
	_glow.setup(Rect2(-192, -192, 384, 384), _paint_glow, 1.0)
	_glow.refresh(0)


func _process(delta: float) -> void:
	_t += delta
	_glow.modulate.a = 0.85 + sin(_t * 7.0) * 0.08 + sin(_t * 13.0) * 0.05


func _paint_glow(ci: CanvasItem) -> void:
	for i in 6:
		ci.draw_circle(Vector2.ZERO, 190.0 - i * 28.0, Color(1.0, 0.82, 0.45, 0.035 + i * 0.012))


## Plain colour around the baked picture (seen only on very wide
## screens), never underneath it, so no pixel is filled twice.
func _draw() -> void:
	var roof := Color(0.36, 0.28, 0.2)
	var wall := Color(0.45, 0.36, 0.26)
	var ground := Color(0.36, 0.27, 0.2)
	var l := BAKED.position.x
	var r := BAKED.end.x
	draw_rect(Rect2(-2000, -2000, 4720, 2000 + BAKED.position.y), roof)
	draw_rect(Rect2(-2000, BAKED.end.y, 4720, 2000), ground)
	for x in [-2000.0, r]:
		var w: float = (l + 2000.0) if x < 0.0 else 2000.0
		draw_rect(Rect2(x, BAKED.position.y, w, -200.0 - BAKED.position.y), roof)
		draw_rect(Rect2(x, -200.0, w, floor_y + 200.0), wall)
		draw_rect(Rect2(x, floor_y, w, BAKED.end.y - floor_y), ground)


func _paint(ci: CanvasItem) -> void:
	ci.draw_rect(Rect2(-700, -700, 2120, 2800), Color(0.2, 0.14, 0.11))
	# Back wall, lit canvas
	var wall := PackedVector2Array([Vector2(90, 150), Vector2(630, 150), Vector2(820, floor_y), Vector2(-100, floor_y)])
	ci.draw_colored_polygon(wall, Color(0.8, 0.69, 0.52))
	for x in [-20.0, 180.0, 360.0, 540.0, 740.0]:
		var top_x := lerpf(90.0, 630.0, (x + 100.0) / 920.0)
		PlushArt.seam(ci, PackedVector2Array([Vector2(top_x, 160), Vector2(x, floor_y - 6)]), Color(0.62, 0.5, 0.36), 12.0, 2.0)
	# Side walls in shadow
	ci.draw_colored_polygon(PackedVector2Array([Vector2(-700, -200), Vector2(90, 150), Vector2(-100, floor_y), Vector2(-700, floor_y)]), Color(0.45, 0.36, 0.26))
	ci.draw_colored_polygon(PackedVector2Array([Vector2(1420, -200), Vector2(630, 150), Vector2(820, floor_y), Vector2(1420, floor_y)]), Color(0.45, 0.36, 0.26))
	ci.draw_colored_polygon(PackedVector2Array([Vector2(-700, -700), Vector2(1420, -700), Vector2(1420, -200), Vector2(630, 150), Vector2(90, 150), Vector2(-700, -200)]), Color(0.36, 0.28, 0.2))
	# Ridge pole
	ci.draw_line(Vector2(360, 150), Vector2(360, floor_y), Color(0.42, 0.29, 0.18), 14.0)
	ci.draw_line(Vector2(354, 150), Vector2(354, floor_y), Color(0.52, 0.37, 0.23), 3.0)
	# Floor and rug
	ci.draw_rect(Rect2(-700, floor_y, 2120, 1400), Color(0.36, 0.27, 0.2))
	PlushArt.ellipse(ci, Vector2(360, floor_y + 18), 320.0, 58.0, Color(0.55, 0.28, 0.24), false)
	PlushArt.ellipse(ci, Vector2(360, floor_y + 18), 270.0, 44.0, Color(0.68, 0.48, 0.3), false)
	PlushArt.ellipse(ci, Vector2(360, floor_y + 18), 210.0, 32.0, Color(0.55, 0.28, 0.24), false)
	# A crate (the supplies themselves are on the shelf)
	ci.draw_rect(Rect2(40, floor_y - 120, 130, 120), Color(0.5, 0.36, 0.22))
	ci.draw_rect(Rect2(40, floor_y - 120, 130, 120), Color(0.3, 0.2, 0.12), false, 3.0)
	ci.draw_line(Vector2(40, floor_y - 60), Vector2(170, floor_y - 60), Color(0.3, 0.2, 0.12), 3.0)
	ci.draw_rect(Rect2(34, floor_y - 128, 142, 10), Color(0.42, 0.3, 0.18))
	# Lantern and its glow
	var lp := LANTERN
	ci.draw_line(Vector2(585, 150), lp + Vector2(0, -30), Color(0.25, 0.2, 0.15), 3.0)
	ci.draw_rect(Rect2(lp.x - 20, lp.y - 30, 40, 54), Color(1.0, 0.86, 0.5))
	ci.draw_rect(Rect2(lp.x - 20, lp.y - 30, 40, 54), Color(0.3, 0.22, 0.14), false, 4.0)
	ci.draw_rect(Rect2(lp.x - 24, lp.y - 38, 48, 10), Color(0.3, 0.22, 0.14))
