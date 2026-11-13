extends Node
## Driftloom's music (decision 0016), when the Driftloom extension is
## built (rust/driftloom-godot). Without it, the game is simply silent
## here, but still decides what would play (the tests read that).
##
## When it changes (decision 0038): at moments that matter, as in a
## console game, never on a timer. A piece repeats until something
## happens: going to another place, a friend made well, a friend home
## from a quest, a death, night falling. The pieces are Driftloom's six
## golden loops for now; Driftloom's generator, once it is in Rust, will
## write music for the friend, the mood and the hour instead.
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

const LOOPS := ["14-bloom", "11-haven", "01-glade", "10-shrine", "03-tide", "07-dust"]
const DIR := "res://addons/driftloom/loops/"
## Each loop's loudness evened out: the gain that brings it to about the
## same average level (measured on the lite mix; the loudest peak stays
## under 0.9, so nothing clips).
const LEVEL := {"11-haven": 1.9, "01-glade": 1.0, "10-shrine": 1.4, "03-tide": 0.75, "07-dust": 1.05, "14-bloom": 2.1}
## The music's volume under the friends' voices and sounds, in dB.
const VOLUME_DB := -1.0

## Which piece for which moment (guesses for Mikey to change).
const PIECE := {
	"tent": "11-haven",          # someone needs looking after
	"tent_well": "14-bloom",     # everyone in front is well
	"play": "01-glade",          # the play corner
	"play_long": "07-dust",      # after playing a while
	"map": "03-tide",            # the map
	"night": "10-shrine",        # late at night, at camp
	"home_well": "14-bloom",     # a friend home from a quest, well
	"home_hurt": "11-haven",     # ... home hurt
	"death": "10-shrine",        # a friend has died
}
## How long a moment's piece plays before the place's own comes back.
const MOMENT := 20.0
## Night, by the player's own clock (debug: `hour=N`).
const NIGHT_FROM := 22
const NIGHT_TO := 6

## True when the extension is present and music is playing.
var available := false
## "thread" (the audio thread pulls it) or "made" (loops made ahead).
var mode := ""
## The piece playing (or chosen, if it isn't ready yet).
var current := ""
## Every change, for the tests: [seconds since start, piece, reason].
var changes: Array = []

## Made-ahead way: how long it may work each frame, microseconds.
const MAKE_BUDGET_US := 3000

var _out: AudioStreamPlayer
var _place := "tent"
var _place_since := 0.0
var _moment := ""
var _moment_until := 0.0
var _clock := 0.0
var _decide_in := 0.0
var _fade: Tween
# Audio-thread way
var _stream: AudioStream
# Made-ahead way
var _player: RefCounted
var _made := {}
var _making := ""
var _bytes := PackedByteArray()
var _cost_us := 0.0
var _waiting := ""


func _ready() -> void:
	Events.place_changed.connect(_on_place)
	Events.quest_ended.connect(_on_quest_ended)
	Events.friend_died.connect(func(): _moment_now("death", "a friend died"))
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
		_out.stream = _stream
		mode = "thread"
	elif ClassDB.class_exists("DriftloomPlayer"):
		_player = ClassDB.instantiate("DriftloomPlayer")
		_player.lite = lite
		# A piece repeats until something happens.
		_out.finished.connect(func(): _out.play())
		mode = "made"
	else:
		return
	available = true
	print("Music: ", mode, " mode")


## The music's work for each 60 fps frame, ms (the perf overlay). On the
## audio thread it is not taken from the frame, but shown all the same.
func cost_ms() -> float:
	if mode == "thread":
		return _stream.cost_ms()
	return _cost_us / 1000.0


## How many loops are made and ready (made-ahead way).
func loops_made() -> int:
	return _made.size()


# --- Deciding ------------------------------------------------------------

func _on_place(place: String) -> void:
	_place = place
	_place_since = _clock
	_moment = ""
	_decide()


func _on_quest_ended(_where: String, result: String) -> void:
	match result:
		"well": _moment_now("home_well", "a friend came home well")
		"hurt": _moment_now("home_hurt", "a friend came home hurt")
		_: _moment_now("death", "a friend died on a quest")


func _moment_now(piece: String, reason: String) -> void:
	_moment = piece
	_moment_until = _clock + MOMENT
	_switch(PIECE[piece], reason)


func _is_night() -> bool:
	var hour := int(Flags.value("hour", str(Time.get_datetime_dict_from_system()["hour"])))
	return hour >= NIGHT_FROM or hour < NIGHT_TO


## What should play now, and why.
func _decide() -> void:
	if _moment != "" and _clock < _moment_until:
		return
	_moment = ""
	match _place:
		"map":
			_switch(PIECE["map"], "the map")
			return
		"play_room":
			if _clock - _place_since > 120.0:
				_switch(PIECE["play_long"], "playing a while")
			else:
				_switch(PIECE["play"], "the play room")
			return
	if _is_night():
		_switch(PIECE["night"], "night at camp")
		return
	var f := Save.friend
	var well := f != null and not f.dead and not f.headless and not f.asleep and f.wellbeing() > 0.85
	_switch(PIECE["tent_well"] if well else PIECE["tent"], "a friend made well" if well else "the tent")


# --- Playing --------------------------------------------------------------

func _switch(piece: String, reason: String) -> void:
	if piece == current:
		return
	current = piece
	changes.append([snappedf(_clock, 0.1), piece, reason])
	print("Music: %s (%s)" % [piece, reason])
	if not available:
		return
	if mode == "thread":
		_fade_out_then(func():
			_out.stop()
			_stream.queue_notes(_text(piece), LEVEL.get(piece, 1.0))
			_out.play()
			_fade_in())
	else:
		_waiting = piece
		_try_start_waiting()


func _try_start_waiting() -> void:
	if _waiting == "" or _made.get(_waiting) == null:
		return
	var wav: AudioStreamWAV = _made[_waiting]
	_waiting = ""
	if not _out.playing:
		_out.stream = wav
		_out.play()
		_fade_in()
		return
	_fade_out_then(func():
		_out.stream = wav
		_out.play()
		_fade_in())


func _fade_out_then(then: Callable) -> void:
	if _fade:
		_fade.kill()
	_fade = create_tween()
	_fade.tween_property(_out, "volume_db", -40.0, 0.6)
	_fade.tween_callback(then)


func _fade_in() -> void:
	_out.volume_db = -30.0
	var tw := create_tween()
	tw.tween_property(_out, "volume_db", VOLUME_DB, 0.8)


func _process(delta: float) -> void:
	_clock += delta
	_decide_in -= delta
	if _decide_in <= 0.0:
		_decide_in = 1.0
		_decide()
	if not available:
		return
	if mode == "thread":
		# The same piece again when it ends, with no gap.
		if _out.playing and not _stream.has_queued() and current != "":
			_stream.queue_notes(_text(current), LEVEL.get(current, 1.0))
		return
	var started := Time.get_ticks_usec()
	_make_some(started + MAKE_BUDGET_US)
	_cost_us = lerpf(_cost_us, float(Time.get_ticks_usec() - started), 0.05)
	_try_start_waiting()


func _text(loop_name: String) -> String:
	var text := FileAccess.get_file_as_string(DIR + loop_name + ".dlnotes")
	if text == "":
		push_warning("Music: can't read " + loop_name)
	return text


## Made-ahead way: work on the loops until `until`, the one wanted first.
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
	if _waiting != "" and not _made.has(_waiting):
		return _waiting
	# Then the ones most likely next: the places' pieces, then the rest.
	for key in ["tent", "tent_well", "play", "map", "night", "play_long"]:
		if not _made.has(PIECE[key]):
			return PIECE[key]
	for loop_name in LOOPS:
		if not _made.has(loop_name):
			return loop_name
	return ""
