class_name SurgeryTable
extends Node2D
## The surgery table (decision 0031): its own place in the tent, where a
## friend lies down to be operated on, under a lamp, on a clean cloth.
## Drawn once.


func _draw() -> void:
	draw_rect(Rect2(-1500, -1500, 3720, 4280), Color(0.17, 0.12, 0.1))
	# Lamp light on the table.
	for i in 5:
		draw_circle(Vector2(360, 640), 520.0 - i * 80.0, Color(1.0, 0.9, 0.65, 0.05 + i * 0.02))
	# The table and its cloth.
	draw_rect(Rect2(-40, 430, 800, 430), Color(0.42, 0.3, 0.2))
	draw_rect(Rect2(-20, 450, 760, 390), Color(0.74, 0.84, 0.78))
	for x in range(0, 760, 38):
		draw_line(Vector2(-20 + x, 450), Vector2(-20 + x, 840), Color(0.68, 0.79, 0.73), 2.0)
	draw_rect(Rect2(-20, 450, 760, 390), Color(0.5, 0.6, 0.55), false, 4.0)
