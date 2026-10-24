extends Node
## Renders Driftloom's golden loops inside the game and compares every
## hash with Driftloom's own (promise 3: same notes, same samples, on
## every device). Run: godot --path game -- --music-test
## On the web, open the page with #music-test at the end of the address.
## Prints MUSIC OK or MUSIC FAILED, with the render speed.

func _ready() -> void:
	await get_tree().process_frame
	var failures: Array[String] = []
	if not ClassDB.class_exists("DriftloomPlayer"):
		print("MUSIC FAILED: the Driftloom extension isn't loaded")
		_done(1)
		return
	var player: RefCounted = ClassDB.instantiate("DriftloomPlayer")
	print("music test: ", player.version(), " on ", OS.get_name())
	var expected := {}
	for line in FileAccess.get_file_as_string("res://addons/driftloom/loops/golden.hashes").split("\n"):
		if line.begins_with("#") or line.strip_edges() == "":
			continue
		expected[line.get_slice(" ", 0)] = line.strip_edges()
	var audio_s := 0.0
	var render_ms := 0.0
	for loop_name in Music.LOOPS:
		var text := FileAccess.get_file_as_string(Music.DIR + loop_name + ".dlnotes")
		var got: String = player.golden_line(text)
		var ms := float(got.get_slice(" ms=", 1))
		var line := got.get_slice(" ms=", 0)
		var ok: bool = expected.get(loop_name, "") == line
		var frames := float(line.get_slice("frames=", 1).get_slice(" ", 0))
		audio_s += frames / 48000.0
		render_ms += ms
		print(("  ok   " if ok else "  FAIL "), loop_name, "  ", "%.0f" % ms, " ms for ", "%.1f" % (frames / 48000.0), " s")
		if not ok:
			print("       got      ", line)
			print("       expected ", expected.get(loop_name, "(none)"))
			failures.append(loop_name)
	print("render speed: %.1fx real time (%.1f s of audio in %.0f ms)" % [audio_s * 1000.0 / maxf(render_ms, 1.0), audio_s, render_ms])
	# Now check the game's music keeps going while the game runs.
	if Music.mode == "thread":
		await get_tree().create_timer(5.0).timeout
		print("music on the audio thread: %.2f ms of work per 60 fps frame (%s)" % [Music.cost_ms(), Music.current])
		if not Music.available or Music.cost_ms() <= 0.0:
			failures.append("the audio thread isn't playing")
	else:
		# Made ahead: the first loop should be made and playing soon.
		var waited := 0.0
		while Music.loops_made() == 0 and waited < 20.0:
			await get_tree().create_timer(0.5).timeout
			waited += 0.5
		await get_tree().create_timer(1.0).timeout
		print("music made ahead: %d loop(s) ready after %.1f s, playing %s, %.2f ms of work per frame while making" % [Music.loops_made(), waited, Music.current, Music.cost_ms()])
		if Music.loops_made() == 0 or Music.current == "":
			failures.append("no loop was made and played")
	if failures.is_empty():
		print("MUSIC OK")
		_done(0)
	else:
		print("MUSIC FAILED: ", ", ".join(failures))
		_done(1)


func _done(code: int) -> void:
	if not OS.has_feature("web"):
		get_tree().quit(code)
