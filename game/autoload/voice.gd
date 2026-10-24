extends Node
## Every sound in the game is synthesized here, no audio files. The
## creatures "talk" in soft blips that sound like communication without
## words (Mikey's recording: R2-D2-like, but not robotic). Driftloom's
## music will join later through the Events bus.

const RATE := 22050
const POOL_SIZE := 8

var _players: Array[AudioStreamPlayer] = []
var _next := 0
var _rng := RandomNumberGenerator.new()


func _ready() -> void:
	_rng.randomize()
	for i in POOL_SIZE:
		var p := AudioStreamPlayer.new()
		p.bus = "Master"
		add_child(p)
		_players.append(p)
	Events.creature_reacted.connect(_on_creature_reacted)


func _on_creature_reacted(kind: String) -> void:
	match kind:
		"boop": blip(1.0)
		"giggle": giggle()
		"dislike": dislike()
		"purr": purr()
		"wake": wake()
		"wince": wince()
		"cheer": cheer()
		"chomp": chomp()
		"glug": glug()
		"bonk": giggle(); thud()
		"catch": cheer()


# --- Creature voice ---------------------------------------------------

## One chirpy syllable. `mood` > 1 is brighter, < 1 is lower.
func blip(mood := 1.0) -> void:
	var n := _rng.randi_range(1, 3)
	var notes: Array = []
	var base := 520.0 * mood * _rng.randf_range(0.9, 1.15)
	for i in n:
		var a := base * _rng.randf_range(0.85, 1.3)
		notes.append([a, a * _rng.randf_range(0.8, 1.35), 0.075, 0.025])
	_play(_voice(notes), -4.0)


func giggle() -> void:
	var notes: Array = []
	var f := 620.0 * _rng.randf_range(0.95, 1.1)
	for i in 5:
		notes.append([f, f * 1.25, 0.05, 0.03])
		f *= 1.09
	_play(_voice(notes), -5.0)


func dislike() -> void:
	_play(_voice([[330.0, 300.0, 0.14, 0.05], [290.0, 220.0, 0.2, 0.0]]), -4.0)


func wake() -> void:
	_play(_voice([[260.0, 420.0, 0.35, 0.0]]), -6.0)


func wince() -> void:
	_play(_voice([[700.0, 380.0, 0.16, 0.0]]), -6.0)


func cheer() -> void:
	_play(_voice([[440.0, 560.0, 0.09, 0.02], [560.0, 700.0, 0.09, 0.02], [700.0, 900.0, 0.16, 0.0]]), -4.0)


func purr() -> void:
	var dur := 0.9
	var n := int(RATE * dur)
	var data := PackedFloat32Array()
	data.resize(n)
	for i in n:
		var t := float(i) / RATE
		var env := sin(PI * t / dur)
		var am := 0.55 + 0.45 * sin(TAU * 22.0 * t)
		data[i] = env * am * (0.6 * sin(TAU * 95.0 * t) + 0.3 * sin(TAU * 190.0 * t))
	_play(_to_wav(data), -8.0)


# --- Material sounds ----------------------------------------------------

## Fabric tearing: crackly filtered noise. `amount` 0..1 sets length.
func tear(amount := 1.0) -> void:
	var dur := 0.12 + 0.35 * amount
	var n := int(RATE * dur)
	var data := PackedFloat32Array()
	data.resize(n)
	var lp := 0.0
	for i in n:
		var t := float(i) / RATE
		var crackle := 1.0 if _rng.randf() < 0.25 else 0.25
		lp += (_rng.randf_range(-1.0, 1.0) * crackle - lp) * 0.45
		data[i] = lp * (1.0 - t / dur) * 0.9
	_play(_to_wav(data), -6.0)


## Soft fluffy whoosh.
func poof() -> void:
	_play(_noise(0.25, 0.08, 0.5), -12.0)


## Dry joints.
func squeak() -> void:
	_play(_voice([[1250.0, 950.0, 0.07, 0.02], [1350.0, 1050.0, 0.06, 0.0]]), -14.0)


func pip() -> void:
	_play(_voice([[1500.0, 1700.0, 0.035, 0.0]]), -10.0)


func pop() -> void:
	_play(_voice([[300.0, 900.0, 0.06, 0.0]]), -8.0)


func thud() -> void:
	_play(_voice([[150.0, 70.0, 0.12, 0.0]]), -2.0)


func squish() -> void:
	_play(_noise(0.12, 0.2, 0.9), -10.0)


func chomp() -> void:
	thud()
	_play(_noise(0.08, 0.3, 0.8), -9.0)


func glug() -> void:
	_play(_voice([[180.0, 260.0, 0.08, 0.03], [200.0, 300.0, 0.08, 0.03], [220.0, 330.0, 0.1, 0.0]]), -6.0)


func clunk() -> void:
	_play(_voice([[220.0, 120.0, 0.08, 0.0], [160.0, 90.0, 0.14, 0.0]]), -3.0)


func splash() -> void:
	_play(_noise(0.3, 0.05, 0.3), -10.0)


# --- Synthesis helpers ------------------------------------------------

## notes: [[start_hz, end_hz, seconds, gap_seconds], ...]. A soft
## triangle-ish tone through a gentle formant, so it reads as a voice.
func _voice(notes: Array) -> AudioStreamWAV:
	var data := PackedFloat32Array()
	var phase := 0.0
	for note in notes:
		var f0: float = note[0]
		var f1: float = note[1]
		var dur: float = note[2]
		var n := int(RATE * dur)
		for i in n:
			var k := float(i) / maxi(1, n - 1)
			var f := lerpf(f0, f1, k)
			phase += f / RATE
			var tri := 1.0 - 4.0 * absf(fmod(phase, 1.0) - 0.5)
			var sine := sin(TAU * phase)
			var vowel := sin(TAU * phase * 2.0) * 0.25
			var env := minf(1.0, k * 12.0) * minf(1.0, (1.0 - k) * 6.0)
			data.append((0.45 * tri + 0.55 * sine + vowel) * env * 0.7)
		for i in int(RATE * float(note[3])):
			data.append(0.0)
	return _to_wav(data)


## Filtered noise. `smooth` 0..1: lower is darker.
func _noise(dur: float, attack: float, smooth: float) -> AudioStreamWAV:
	var n := int(RATE * dur)
	var data := PackedFloat32Array()
	data.resize(n)
	var lp := 0.0
	for i in n:
		var t := float(i) / RATE
		lp += (_rng.randf_range(-1.0, 1.0) - lp) * smooth * 0.5
		var env := minf(1.0, t / maxf(0.001, attack)) * (1.0 - t / dur)
		data[i] = lp * env
	return _to_wav(data)


func _to_wav(data: PackedFloat32Array) -> AudioStreamWAV:
	var bytes := PackedByteArray()
	bytes.resize(data.size() * 2)
	for i in data.size():
		bytes.encode_s16(i * 2, int(clampf(data[i], -1.0, 1.0) * 32000.0))
	var wav := AudioStreamWAV.new()
	wav.format = AudioStreamWAV.FORMAT_16_BITS
	wav.mix_rate = RATE
	wav.stereo = false
	wav.data = bytes
	return wav


func _play(stream: AudioStream, volume_db: float) -> void:
	var p := _players[_next]
	_next = (_next + 1) % POOL_SIZE
	p.stream = stream
	p.volume_db = volume_db
	p.pitch_scale = 1.0
	p.play()
