extends Node
## Every sound in the game is synthesized here, no audio files. The
## friends "talk" in soft blips that sound like communication without
## words (Mikey's recording: R2-D2-like, but not robotic). Driftloom's
## music will join later through the Events bus.

const RATE := 22050
const POOL_SIZE := 8

var _players: Array[AudioStreamPlayer] = []
var _next := 0
var _rng := RandomNumberGenerator.new()


## How many versions of each sound to make, so repeats don't sound
## identical. Each also gets a slight random pitch when played.
const VARIANTS := 4

## Every sound is made ahead of time, a little each frame while the game
## starts, and then only played back: making a sound mid-game costs a
## weak phone a stutter (Driftloom: no heavy work while playing).
## name -> [how many versions, maker]
var _makers := {
	"blip": [VARIANTS, _make_blip],
	"giggle": [VARIANTS, _make_giggle],
	"dislike": [1, _voice.bind([[330.0, 300.0, 0.14, 0.05], [290.0, 220.0, 0.2, 0.0]])],
	"wake": [1, _voice.bind([[260.0, 420.0, 0.35, 0.0]])],
	"wince": [1, _voice.bind([[700.0, 380.0, 0.16, 0.0]])],
	"cheer": [1, _voice.bind([[440.0, 560.0, 0.09, 0.02], [560.0, 700.0, 0.09, 0.02], [700.0, 900.0, 0.16, 0.0]])],
	"purr": [1, _make_purr],
	"tear": [VARIANTS, _make_tear.bind(1.0)],
	"tear_small": [VARIANTS, _make_tear.bind(0.15)],
	"poof": [VARIANTS, _noise.bind(0.25, 0.08, 0.5)],
	"squeak": [1, _voice.bind([[1250.0, 950.0, 0.07, 0.02], [1350.0, 1050.0, 0.06, 0.0]])],
	"pip": [1, _voice.bind([[1500.0, 1700.0, 0.035, 0.0]])],
	"pop": [1, _voice.bind([[300.0, 900.0, 0.06, 0.0]])],
	"thud": [1, _voice.bind([[150.0, 70.0, 0.12, 0.0]])],
	"squish": [VARIANTS, _noise.bind(0.12, 0.2, 0.9)],
	"crunch": [VARIANTS, _noise.bind(0.08, 0.3, 0.8)],
	"glug": [1, _voice.bind([[180.0, 260.0, 0.08, 0.03], [200.0, 300.0, 0.08, 0.03], [220.0, 330.0, 0.1, 0.0]])],
	"clunk": [1, _voice.bind([[220.0, 120.0, 0.08, 0.0], [160.0, 90.0, 0.14, 0.0]])],
	"splash": [VARIANTS, _noise.bind(0.3, 0.05, 0.3)],
}
## name -> Array of made sounds
var _bank := {}
var _to_make: Array = []


func _ready() -> void:
	_rng.randomize()
	for i in POOL_SIZE:
		var p := AudioStreamPlayer.new()
		p.bus = "Master"
		add_child(p)
		_players.append(p)
	Events.friend_reacted.connect(_on_friend_reacted)
	for sound in _makers:
		for i in int(_makers[sound][0]):
			_to_make.append(sound)


var _making_us := 0


## Make sounds for at most 2 ms a frame until all are made.
func _process(_delta: float) -> void:
	var started := Time.get_ticks_usec()
	while not _to_make.is_empty() and Time.get_ticks_usec() < started + 2000:
		_make(_to_make.pop_back())
	_making_us += Time.get_ticks_usec() - started
	if _to_make.is_empty():
		set_process(false)
		if Flags.has("perf"):
			print("PERF sounds made ahead: %d ms of work in all" % (_making_us / 1000))


func _make(sound: String) -> void:
	var made: AudioStreamWAV = _makers[sound][1].call()
	if not _bank.has(sound):
		_bank[sound] = []
	_bank[sound].append(made)


## Play a ready-made sound; if it isn't made yet, make one now.
func _say(sound: String, volume_db: float, pitch_jitter := 0.04) -> void:
	if not _bank.has(sound):
		_to_make.erase(sound)
		_make(sound)
	var versions: Array = _bank[sound]
	_play(versions[_rng.randi() % versions.size()], volume_db, 1.0 + _rng.randf_range(-pitch_jitter, pitch_jitter))


func _on_friend_reacted(kind: String) -> void:
	match kind:
		"boop": blip()
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


# --- Friend voice ---------------------------------------------------

## One chirpy syllable.
func blip() -> void:
	_say("blip", -4.0, 0.08)


func giggle() -> void:
	_say("giggle", -5.0, 0.05)


func dislike() -> void:
	_say("dislike", -4.0)


func wake() -> void:
	_say("wake", -6.0)


func wince() -> void:
	_say("wince", -6.0)


func cheer() -> void:
	_say("cheer", -4.0)


func purr() -> void:
	_say("purr", -8.0)


# --- Material sounds ----------------------------------------------------

## Fabric tearing. `amount` 0..1: a small pluck or a full rip.
func tear(amount := 1.0) -> void:
	_say("tear" if amount > 0.5 else "tear_small", -6.0)


## Soft fluffy whoosh.
func poof() -> void:
	_say("poof", -12.0)


## Dry joints.
func squeak() -> void:
	_say("squeak", -14.0)


func pip() -> void:
	_say("pip", -10.0)


func pop() -> void:
	_say("pop", -8.0)


func thud() -> void:
	_say("thud", -2.0)


func squish() -> void:
	_say("squish", -10.0)


func chomp() -> void:
	thud()
	_say("crunch", -9.0)


func glug() -> void:
	_say("glug", -6.0)


func clunk() -> void:
	_say("clunk", -3.0)


func splash() -> void:
	_say("splash", -10.0)


# --- Sound makers -------------------------------------------------------

func _make_blip() -> AudioStreamWAV:
	var n := _rng.randi_range(1, 3)
	var notes: Array = []
	var base := 520.0 * _rng.randf_range(0.9, 1.15)
	for i in n:
		var a := base * _rng.randf_range(0.85, 1.3)
		notes.append([a, a * _rng.randf_range(0.8, 1.35), 0.075, 0.025])
	return _voice(notes)


func _make_giggle() -> AudioStreamWAV:
	var notes: Array = []
	var f := 620.0 * _rng.randf_range(0.95, 1.1)
	for i in 5:
		notes.append([f, f * 1.25, 0.05, 0.03])
		f *= 1.09
	return _voice(notes)


func _make_purr() -> AudioStreamWAV:
	var dur := 0.9
	var n := int(RATE * dur)
	var data := PackedFloat32Array()
	data.resize(n)
	for i in n:
		var t := float(i) / RATE
		var env := sin(PI * t / dur)
		var am := 0.55 + 0.45 * sin(TAU * 22.0 * t)
		data[i] = env * am * (0.6 * sin(TAU * 95.0 * t) + 0.3 * sin(TAU * 190.0 * t))
	return _to_wav(data)


## Crackly filtered noise. `amount` 0..1 sets length.
func _make_tear(amount: float) -> AudioStreamWAV:
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
	return _to_wav(data)


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


func _play(stream: AudioStream, volume_db: float, pitch := 1.0) -> void:
	var p := _players[_next]
	_next = (_next + 1) % POOL_SIZE
	p.stream = stream
	p.volume_db = volume_db
	p.pitch_scale = pitch
	p.play()
