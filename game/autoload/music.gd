extends Node
## Driftloom's music (decision 0016), when the Driftloom extension is
## built (rust/driftloom-godot). Without it, the game is simply silent here.
##
## Two ways to play, same notes and same samples:
## - Phones and computers: a `DriftloomStream` that Godot's own audio
##   thread pulls from, so the music never shares the screen's time
##   (Driftloom's first rule).
## - The browser: Godot's web engine has no threads, so a
##   `DriftloomPlayer` fills an AudioStreamGenerator a little each frame,
##   with half a second of sound ready in case a frame is slow.
##
## For now it plays Driftloom's golden loops in turn: the core is the
## synthesizer, and the part that writes new notes is still JavaScript in
## the Driftloom app until its phase 2. Creature themes and music that
## follows the camp come once it moves to Rust.

const LOOPS := ["11-haven", "01-glade", "10-shrine", "03-tide", "07-dust", "14-bloom"]
const DIR := "res://addons/driftloom/loops/"
## Each loop's loudness evened out: the gain that brings it to about the
## same average level (measured on the lite mix; the loudest peak stays
## under 0.9, so nothing clips).
const LEVEL := {"11-haven": 1.9, "01-glade": 1.0, "10-shrine": 1.4, "03-tide": 0.75, "07-dust": 1.05, "14-bloom": 2.1}
## The music's volume under the creatures' voices and sounds, in dB.
const VOLUME_DB := -1.0

## True when the extension is present and music is playing.
var available := false
## "thread" (the audio thread pulls it) or "frames" (pushed each frame).
var mode := ""
## The loop most recently loaded (each-frame way) or queued (audio thread).
var current := ""

var _out: AudioStreamPlayer
var _index := 0
# Audio-thread way
var _stream: AudioStream
# Each-frame way
var _player: RefCounted
var _playback: AudioStreamGeneratorPlayback
var _cost_us := 0.0


func _ready() -> void:
	if Flags.has("nomusic"):
		return
	# Phones and the browser get Driftloom's lite mix (half the reverb's
	# work, the same notes); computers the full one.
	var lite := OS.has_feature("web") or OS.has_feature("mobile")
	_out = AudioStreamPlayer.new()
	_out.volume_db = VOLUME_DB
	if ClassDB.class_exists("DriftloomStream") and not OS.has_feature("web"):
		_stream = ClassDB.instantiate("DriftloomStream")
		_stream.set_lite(lite)
		if not _queue_next():
			return
		_out.stream = _stream
		mode = "thread"
	elif ClassDB.class_exists("DriftloomPlayer"):
		_player = ClassDB.instantiate("DriftloomPlayer")
		_player.lite = lite
		if not _load(LOOPS[0]):
			return
		var gen := AudioStreamGenerator.new()
		gen.mix_rate = _player.sample_rate()
		gen.buffer_length = 0.5 if OS.has_feature("web") else 0.25
		_out.stream = gen
		# The web's default "sample" playback can't stream generated audio.
		_out.playback_type = AudioServer.PLAYBACK_TYPE_STREAM
		mode = "frames"
	else:
		return
	add_child(_out)
	_out.play()
	if mode == "frames":
		_playback = _out.get_stream_playback()
	available = true
	print("Music: ", mode, " mode, starting with ", LOOPS[0])


## The music's work for each 60 fps frame, ms (the perf overlay). On the
## audio thread it is not taken from the frame, but shown all the same.
func cost_ms() -> float:
	if mode == "thread":
		return _stream.cost_ms()
	return _cost_us / 1000.0


## Underruns since the music started, each-frame way only: moments the
## speakers asked for sound that wasn't ready. -1 when not counted.
func skips() -> int:
	return _playback.get_skips() if mode == "frames" else -1


func _process(_delta: float) -> void:
	if not available:
		return
	if mode == "thread":
		# Keep one loop waiting so the next starts with no gap.
		if not _stream.has_queued():
			_queue_next()
		return
	var started := Time.get_ticks_usec()
	if _player.is_finished():
		_index = (_index + 1) % LOOPS.size()
		_load(LOOPS[_index])
	var n := _playback.get_frames_available()
	if n > 0:
		_playback.push_buffer(_player.render_stereo(n))
	_cost_us = lerpf(_cost_us, float(Time.get_ticks_usec() - started), 0.05)


func _text(loop_name: String) -> String:
	var text := FileAccess.get_file_as_string(DIR + loop_name + ".dlnotes")
	if text == "":
		push_warning("Music: can't read " + loop_name)
	return text


## Audio-thread way: hand the next loop to the stream.
func _queue_next() -> bool:
	var loop_name: String = LOOPS[_index]
	_index = (_index + 1) % LOOPS.size()
	var err: String = _stream.queue_notes(_text(loop_name), LEVEL.get(loop_name, 1.0))
	if err != "":
		push_warning("Music: " + loop_name + ": " + err)
		return false
	current = loop_name
	return true


## Each-frame way: load a loop into the player.
func _load(loop_name: String) -> bool:
	var err: String = _player.load_notes(_text(loop_name))
	if err != "":
		push_warning("Music: " + loop_name + ": " + err)
		return false
	_player.looping = false
	_player.gain = LEVEL.get(loop_name, 1.0)
	current = loop_name
	return true
