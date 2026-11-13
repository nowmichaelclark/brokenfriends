class_name Guide
extends Node2D
## Shows, without words, what could be done next (decision 0032): a
## soft pulsing ring where to touch, and a dotted line where to drag.
## In the tent's world; the shelf's own glow is drawn by the HUD.

## Where to touch, or Vector2.INF for nothing.
var target := Vector2.INF
var radius := 46.0
## Where to drag to from `target`, or Vector2.INF.
var to := Vector2.INF

var _t := 0.0


func show_cue(at: Vector2, r := 46.0, drag_to := Vector2.INF) -> void:
	target = at
	radius = r
	to = drag_to


func clear() -> void:
	target = Vector2.INF
	to = Vector2.INF


func _process(delta: float) -> void:
	_t += delta
	if target != Vector2.INF:
		queue_redraw()


func _draw() -> void:
	if target == Vector2.INF:
		return
	var pulse := 0.5 + 0.5 * sin(_t * 4.0)
	var col := Color(1.0, 0.92, 0.55, 0.35 + 0.4 * pulse)
	draw_arc(target, radius * (0.9 + 0.15 * pulse), 0, TAU, 40, col, 5.0, true)
	if to != Vector2.INF:
		var d := to - target
		var n := int(d.length() / 22.0)
		var phase := fmod(_t * 1.5, 1.0)
		for i in n:
			var k := (float(i) + phase) / maxf(1.0, n)
			draw_circle(target + d * k, 5.0, Color(1.0, 0.92, 0.55, 0.8 * (1.0 - k * 0.5)))
		draw_arc(to, 22.0, 0, TAU, 24, Color(1.0, 0.92, 0.55, 0.6), 3.0, true)
