extends Node
## Driftloom's music (decision 0016), when the Driftloom extension is
## built (rust/driftloom-godot). Without it, the game is simply silent here.
##
## Two ways to play, same notes and same samples:
## - Phones and computers: a `DriftloomStream` that Godot's own audio
##   thread pulls from, so the music never shares the screen's time
##   (Driftloom's first rule).
## - The browser: Godot's web engine has no audio thread, and music fed
##   to it frame by frame crackles on phones. So each loop is made whole
##   ahead of time by a `DriftloomPlayer`, a few milliseconds each frame,
##   and handed to the browser to play as one finished sound. Once all
##   six are made, the music costs the game nothing.
##
## For now it plays Driftloom's golden loops in turn: the core is the
## synthesizer, and the part that writes new notes is still JavaScript in
## the Driftloom app until its phase 2. Friend themes and music that
## follows the camp come once it moves to Rust.

## The shortest first, so in the browser the music starts soon.
const LOOPS := ["14-bloom", "11-haven", "01-glade", "10-shrine", "03-tide", "07-dust"]
const DIR := "res://addons/driftloom/loops/"
## Each loop's loudness evened out: the gain that brings it to about the
## same average level (measured on the lite mix; the loudest peak stays
## under 0.9, so nothing clips).
const LEVEL := {"11-haven": 1.9, "01-glade": 1.0, "10-shrine": 1.4, "03-tide": 0.75, "07-dust": 1.05, "14-bloom": 2.1}
## The music's volume under the friends' voices and sounds, in dB.
const VOLUME_DB := -1.0

## True when the extension is present and music is playing.
var available := false
## "thread" (the audio thread pulls it) or "made" (loops made ahead).
var mode := ""
## The loop playing (made ahead) or most recently queued (audio thread).
var current := ""

## Made-ahead way: how long it may work each frame, microseconds.
const MAKE_BUDGET_US := 3000

var _out: AudioStreamPlayer
var _index := 0
# Audio-thread way
var _stream: AudioStream
# Made-ahead way
var _player: RefCounted
var _made := {}
var _making := ""
var _bytes := PackedByteArray()
var _cost_us := 0.0


func _ready() -> void:
	if Flags.has("nomusic"):
		return
	# Phones and the browser get Driftloom's lite mix (half the reverb's
	# work, the same notes); computers the full one.
	var lite := OS.has_feature("web") or OS.has_feature("mobile")
	_out = AudioStreamPlayer.new()
	_out.volume_db = VOLUME_DB
	add_child(_out)
	if ClassDB.class_exists("DriftloomStream") and not OS.has_feature("web"):
		_stream = ClassDB.instantiate("DriftloomStream")
		_stream.set_lite(lite)
		if not _queue_next():
			return
		_out.stream = _stream
		_out.play()
		mode = "thread"
	elif ClassDB.class_exists("DriftloomPlayer"):
		_player = ClassDB.instantiate("DriftloomPlayer")
		_player.lite = lite
		_out.finished.connect(_play_next)
		mode = "made"
	else:
		return
	available = true
	print("Music: ", mode, " mode, starting with ", LOOPS[0])


## The music's work for each 60 fps frame, ms (the perf overlay). On the
## audio thread it is not taken from the frame, but shown all the same.
func cost_ms() -> float:
	if mode == "thread":
		return _stream.cost_ms()
	return _cost_us / 1000.0


## How many loops are made and ready (made-ahead way).
func loops_made() -> int:
	return _made.size()


func _process(_delta: float) -> void:
	if not available:
		return
	if mode == "thread":
		# Keep one loop waiting so the next starts with no gap.
		if not _stream.has_queued():
			_queue_next()
		return
	var started := Time.get_ticks_usec()
	_make_some(started + MAKE_BUDGET_US)
	_cost_us = lerpf(_cost_us, float(Time.get_ticks_usec() - started), 0.05)
	if not _out.playing and _made.has(LOOPS[_index]):
		_play_next()


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


## Made-ahead way: play the next loop if it is made (if not, _process
## starts it as soon as it is).
func _play_next() -> void:
	# Pass over any loop that failed to load.
	for i in LOOPS.size():
		if _made.get(LOOPS[_index], 0) != null:
			break
		_index = (_index + 1) % LOOPS.size()
	var loop_name: String = LOOPS[_index]
	if _made.get(loop_name) == null:
		# Not made yet (a slow phone): play the last one again meanwhile.
		if _out.stream:
			_out.play()
		return
	_out.stream = _made[loop_name]
	_out.play()
	current = loop_name
	print("Music: playing ", loop_name)
	_index = (_index + 1) % LOOPS.size()


## Made-ahead way: work on the loops in the order they play until `until`.
func _make_some(until: int) -> void:
	while Time.get_ticks_usec() < until:
		if _making == "":
			_making = _next_to_make()
			if _making == "":
				return
			var err: String = _player.load_notes(_text(_making))
			if err != "":
				push_warning("Music: " + _making + ": " + err)
				_made[_making] = null
				_making = ""
				continue
			_player.looping = false
			_player.gain = LEVEL.get(_making, 1.0)
			_bytes = PackedByteArray()
			continue
		var piece: PackedByteArray = _player.render_pcm16(512)
		if piece.is_empty():
			var wav := AudioStreamWAV.new()
			wav.format = AudioStreamWAV.FORMAT_16_BITS
			wav.mix_rate = int(_player.sample_rate())
			wav.stereo = false
			wav.data = _bytes
			_made[_making] = wav
			_bytes = PackedByteArray()
			_making = ""
		else:
			_bytes.append_array(piece)


func _next_to_make() -> String:
	for i in LOOPS.size():
		var loop_name: String = LOOPS[(_index + i) % LOOPS.size()]
		if not _made.has(loop_name):
			return loop_name
	return ""
