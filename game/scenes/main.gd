extends Node
## Holds the places the player can be and switches between them.
## For now: the tent, and the play corner inside it.

var tent: Node2D
var play: Node2D


func _ready() -> void:
	var flow_arg := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--flow-test"):
			flow_arg = arg
			Save.new_game()
	if Flags.has("empty"):
		# Debug: nothing but the engine, to measure its own cost per frame.
		add_child(load("res://debug/perf_overlay.gd").new())
		return
	tent = preload("res://scenes/tent/tent.gd").new()
	add_child(tent)
	tent.go_to.connect(go_to)
	if ResourceLoader.exists("res://scenes/play/play_corner.gd"):
		play = load("res://scenes/play/play_corner.gd").new()
		add_child(play)
		play.go_to.connect(go_to)
	go_to("tent")
	if Flags.has("perf"):
		add_child(load("res://debug/perf_overlay.gd").new())
	var music_test := "--music-test" in OS.get_cmdline_user_args()
	if OS.has_feature("web"):
		var page_hash = JavaScriptBridge.eval("window.location.hash", true)
		music_test = music_test or str(page_hash) == "#music-test"
	if music_test:
		add_child(load("res://tests/music_test.gd").new())
	if flow_arg != "":
		var test: Node = load("res://tests/flow_test.gd").new()
		test.out_dir = flow_arg.get_slice("=", 1) if "=" in flow_arg else ""
		add_child(test)


func go_to(place: String) -> void:
	tent.set_active(place == "tent")
	if play:
		play.set_active(place == "play_corner")
	Events.place_changed.emit(place)
