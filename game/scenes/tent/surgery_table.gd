class_name SurgeryTable
extends Node2D
## The surgery table (decision 0031): its own place in the tent. The
## friend stands upright against a padded board, as in the grooming
## room, under a lamp (Mikey's playtest notes: "keep it vertical...
## just more zoomed in"). Baked once into one picture (its see-through
## lamp light, drawn every frame, cost as much as the friend); beyond
## it only plain colour.

const BAKED := Rect2(-400, -300, 1520, 1800)
const DARK := Color(0.17, 0.12, 0.1)


func _ready() -> void:
	var art := Baked.new()
	add_child(art)
	art.setup(BAKED, _paint, 1.0)
	art.refresh(0)


## Plain colour around the picture, never under it.
func _draw() -> void:
	draw_rect(Rect2(-3000, -3000, 6720, 3000 + BAKED.position.y), DARK)
	draw_rect(Rect2(-3000, BAKED.end.y, 6720, 3000), DARK)
	draw_rect(Rect2(-3000, BAKED.position.y, 3000 + BAKED.position.x, BAKED.size.y), DARK)
	draw_rect(Rect2(BAKED.end.x, BAKED.position.y, 3000, BAKED.size.y), DARK)


func _paint(ci: CanvasItem) -> void:
	ci.draw_rect(BAKED, DARK)
	# Lamp light on the board.
	for i in 5:
		ci.draw_circle(Vector2(360, 600), 560.0 - i * 90.0, Color(1.0, 0.9, 0.65, 0.05 + i * 0.02))
	# The padded board, standing, and its clean cloth.
	ci.draw_rect(Rect2(110, 230, 500, 830), Color(0.42, 0.3, 0.2))
	ci.draw_rect(Rect2(130, 250, 460, 790), Color(0.74, 0.84, 0.78))
	for y in range(0, 790, 38):
		ci.draw_line(Vector2(130, 250 + y), Vector2(590, 250 + y), Color(0.68, 0.79, 0.73), 2.0)
	ci.draw_rect(Rect2(130, 250, 460, 790), Color(0.5, 0.6, 0.55), false, 4.0)
	# The step the friend stands on, and the lamp above.
	ci.draw_rect(Rect2(150, 930, 420, 40), Color(0.5, 0.36, 0.24))
	ci.draw_rect(Rect2(150, 930, 420, 40), Color(0.3, 0.2, 0.12), false, 3.0)
	ci.draw_line(Vector2(360, 60), Vector2(360, 160), Color(0.3, 0.25, 0.2), 4.0)
	ci.draw_colored_polygon(PackedVector2Array([Vector2(310, 200), Vector2(410, 200), Vector2(385, 160), Vector2(335, 160)]), Color(0.55, 0.6, 0.58))
	ci.draw_circle(Vector2(360, 204), 14.0, Color(1.0, 0.95, 0.75))
