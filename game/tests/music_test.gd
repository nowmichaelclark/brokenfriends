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
	# The golden renders above block the game for seconds, so the stream
	# underran then. Now check it keeps up while the game runs normally.
	await get_tree().create_timer(1.0).timeout
	var before: int = Music.skips()
	await get_tree().create_timer(4.0).timeout
	var after: int = Music.skips()
	print("stream underruns in 4 s of play: ", after - before, " (playing ", Music.current, ")")
	if after - before > 0:
		failures.append("streaming underran")
	if failures.is_empty():
		print("MUSIC OK")
		_done(0)
	else:
		print("MUSIC FAILED: ", ", ".join(failures))
		_done(1)


func _done(code: int) -> void:
	if not OS.has_feature("web"):
		get_tree().quit(code)
