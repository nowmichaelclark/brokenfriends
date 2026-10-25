extends Node
## Holds the places the player can be and switches between them: the
## entrance (the starting screen), the tent's grooming screen and surgery
## table, the play room, the map, and the physics test (from the dev
## menu).

var entrance: Entrance
var tent: Tent
var play: PlayRoom
var map: Node2D
## The physics test, while it's open (made fresh each time).
var physics: PhysicsPlay


func _ready() -> void:
	var flow_arg := ""
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--flow-test"):
			flow_arg = arg
			Save.new_game()
	var gallery := Flags.value("gallery", "none")
	if gallery != "none" or Flags.has("gallery"):
		# Test: twenty generated friends side by side (tests/gallery.gd).
		var g: Node2D = load("res://tests/gallery.gd").new()
		g.out_dir = gallery if gallery != "none" else ""
		add_child(g)
		return
	if Flags.has("physics"):
		# The physics test (prototype 4), on its own.
		add_child(PhysicsPlay.new())
		if Flags.has("perf"):
			add_child(load("res://debug/perf_overlay.gd").new())
		return
	if Flags.has("empty"):
		# Debug: nothing but the engine, to measure its own cost per frame.
		add_child(load("res://debug/perf_overlay.gd").new())
		return
	entrance = Entrance.new()
	add_child(entrance)
	entrance.go_to.connect(go_to)
	tent = preload("res://scenes/tent/tent.gd").new()
	add_child(tent)
	tent.go_to.connect(go_to)
	play = PlayRoom.new()
	add_child(play)
	play.go_to.connect(go_to)
	map = load("res://scenes/map/map.gd").new()
	add_child(map)
	map.go_to.connect(go_to)
	# `#place=table` (or grooming, play_room, map) opens straight there;
	# `#crowd=N` opens the play room with N more friends.
	go_to(Flags.value("place", "play_room" if Flags.has("crowd") else "entrance"))
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


## "entrance", "grooming", "table", "play_room", "map" or "physics".
func go_to(place: String) -> void:
	entrance.set_active(place == "entrance")
	tent.set_active(place in ["grooming", "table"], place if place == "table" else "grooming")
	play.set_active(place == "play_room")
	map.set_active(place == "map")
	if place == "physics" and physics == null:
		physics = PhysicsPlay.new()
		physics.in_game = true
		physics.go_to.connect(go_to)
		add_child(physics)
	elif place != "physics" and physics:
		physics.queue_free()
		physics = null
	Events.place_changed.emit(place)
