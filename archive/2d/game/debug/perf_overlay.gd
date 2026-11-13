extends CanvasLayer
## Frame timings on screen (and in the console every 2 s), for checking
## the game on weak phones. Turned on by the `perf` flag (debug/flags.gd).

var _label: Label
var _frames := 0
var _worst := 0.0
var _sum := 0.0
var _t := 0.0
var _log_t := 0.0


func _ready() -> void:
	layer = 100
	_label = Label.new()
	_label.position = Vector2(12, 100)
	_label.add_theme_font_size_override("font_size", 22)
	_label.add_theme_color_override("font_color", Color(1, 1, 0.6))
	_label.add_theme_color_override("font_outline_color", Color.BLACK)
	_label.add_theme_constant_override("outline_size", 6)
	add_child(_label)


func _process(delta: float) -> void:
	_frames += 1
	_sum += delta
	_worst = maxf(_worst, delta)
	_t += delta
	_log_t += delta
	if _t >= 1.0:
		var avg_ms := _sum / _frames * 1000.0
		var text := "fps %d  avg %.1f ms  worst %.1f ms\nscript %.1f ms  physics %.1f ms  music %.2f ms/frame\ndraw calls %d  objects %d" % [
			Engine.get_frames_per_second(), avg_ms, _worst * 1000.0,
			Performance.get_monitor(Performance.TIME_PROCESS) * 1000.0,
			Performance.get_monitor(Performance.TIME_PHYSICS_PROCESS) * 1000.0,
			Music.cost_ms(),
			Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME),
			Performance.get_monitor(Performance.OBJECT_COUNT)]
		_label.text = text
		if _log_t >= 2.0:
			_log_t = 0.0
			print("PERF ", text.replace("\n", "  "))
		_frames = 0
		_sum = 0.0
		_worst = 0.0
		_t = 0.0
