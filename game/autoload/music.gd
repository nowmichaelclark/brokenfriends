extends Node
## Driftloom's music (decision 0016). When the Driftloom extension is
## built (rust/driftloom-godot), this streams the core's audio through an
## AudioStreamGenerator. Without it, the game is simply silent here.
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

## True when the extension is present and a loop is playing.
var available := false
var player: RefCounted
var current := ""

var _out: AudioStreamPlayer
var _playback: AudioStreamGeneratorPlayback
var _index := 0


var _cost_us := 0.0


## Average time the music takes per frame, ms (for the perf overlay).
func cost_ms() -> float:
	return _cost_us / 1000.0


func _ready() -> void:
	if not ClassDB.class_exists("DriftloomPlayer") or Flags.has("nomusic"):
		return
	player = ClassDB.instantiate("DriftloomPlayer")
	# Driftloom's lite mix (half the reverb's work) on phones and in the
	# browser; the full mix on computers.
	player.lite = OS.has_feature("web") or OS.has_feature("mobile")
	if not _load(LOOPS[0]):
		return
	var gen := AudioStreamGenerator.new()
	gen.mix_rate = player.sample_rate()
	gen.buffer_length = 0.5 if OS.has_feature("web") else 0.25
	_out = AudioStreamPlayer.new()
	_out.stream = gen
	_out.volume_db = VOLUME_DB
	# The web's default "sample" playback can't stream generated audio.
	_out.playback_type = AudioServer.PLAYBACK_TYPE_STREAM
	add_child(_out)
	_out.play()
	_playback = _out.get_stream_playback()
	available = true
	print("Music: ", player.version(), ", playing ", current)


func _load(loop_name: String) -> bool:
	var text := FileAccess.get_file_as_string(DIR + loop_name + ".dlnotes")
	if text == "":
		push_warning("Music: can't read " + loop_name)
		return false
	var err: String = player.load_notes(text)
	if err != "":
		push_warning("Music: " + loop_name + ": " + err)
		return false
	player.looping = false
	player.gain = LEVEL.get(loop_name, 1.0)
	current = loop_name
	return true


## Underruns since the music started: frames the speakers asked for
## that weren't ready. Should stay at 0 while the game is idle.
func skips() -> int:
	return _playback.get_skips() if available else -1


func _process(_delta: float) -> void:
	if not available:
		return
	var started := Time.get_ticks_usec()
	if player.is_finished():
		_index = (_index + 1) % LOOPS.size()
		_load(LOOPS[_index])
	var n := _playback.get_frames_available()
	if n > 0:
		_playback.push_buffer(player.render_stereo(n))
	_cost_us = lerpf(_cost_us, float(Time.get_ticks_usec() - started), 0.05)
