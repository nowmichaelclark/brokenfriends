class_name FriendData
extends Resource
## A friend, as data. Mostly fluff inside, with hydraulic ligaments,
## a battery heart and a chip brain (Mikey's recording, 2026-10-24).
## The view draws whatever this says; nothing here knows about screens.

const SLOTS := ["leg_l", "leg_r", "arm_l", "arm_r", "eye_l", "eye_r"]

@export var name := "Friend"
@export var look_seed := 1
@export var body_fabric := Color(0.72, 0.51, 0.35)
@export var trim := Color(0.94, 0.86, 0.75)
## Stuffing inside. 1.0 is well filled; lower looks thin and floppy,
## higher looks plump. (Assumption 8, waiting on Mikey: weight is stuffing.)
@export var stuffing := 1.0
## Hydraulic fluid and oil. Low means stiff, squeaky joints.
@export var fluid := 1.0
## 0..1 soaked.
@export var wetness := 0.0
## Mud splotches on the body: [{x, y, r, a}] in body-local units.
@export var mud: Array = []
## Asleep or passed out; the first poke wakes them.
@export var asleep := false
## Where the tickle spot is, in body-local units.
@export var tickle_spot := Vector2(-35, 45)
## Grows by playing with the ball (decision 0017: play, not train).
@export var catch_skill := 0.05
@export var parts: Dictionary = {}
## The chip brain, the friend's personality (decision 0026). 1 is
## well; it fails only when the body is falling apart (0028).
@export var brain := 1.0
## False for a newcomer the player hasn't seen yet: it waits, no worse,
## until then, so nobody is lost to an absence (pillar 4).
@export var met := true
## Grows a little with each trip that goes well (0034): a friend who
## has been out more gets about better.
@export var stamina := 0.0
## Death is final (0011).
@export var dead := false
## The head's own fabric (it can end up on a body of another colour,
## decision 0026).
@export var head_fabric := Color(0, 0, 0, 0)
## A body whose head has been taken off: not a friend any more, only a
## body waiting for a head (0026).
@export var headless := false
## A head without a body, kept in the pocket: the friend lives on in it
## for about a day (0026). `off_at` is when it came off (game time).
@export var head_only := false
@export var off_at := 0.0


func head_color() -> Color:
	return head_fabric if head_fabric.a > 0.0 else body_fabric


func part(slot: String) -> PartData:
	return parts.get(slot)


## A slot with no part in it: an open wound.
func is_open(slot: String) -> bool:
	return parts.get(slot) == null


## How well they are, 0..1. Shown only on the body, never as a number
## (pillar 1: the body is the interface).
func wellbeing() -> float:
	var w := 1.0
	for slot in SLOTS:
		var p: PartData = parts.get(slot)
		if p == null:
			w -= 0.22
		else:
			w -= p.rot * 0.35
			if not p.attached:
				w -= 0.1
	var mud_total := 0.0
	for m in mud:
		mud_total += float(m["a"])
	w -= minf(0.2, mud_total * 0.03)
	w -= clampf(1.0 - stuffing, 0.0, 1.0) * 0.25
	w -= clampf(1.0 - fluid, 0.0, 1.0) * 0.12
	w -= wetness * 0.08
	w -= (1.0 - brain) * 0.5
	if dead:
		return 0.0
	return clampf(w, 0.0, 1.0)


## Take this friend's head off (0026). This friend becomes the head
## (`head_only`); the returned data is the body left behind, headless.
## The eyes stay with the head; arms, legs, stuffing and the rest stay
## with the body.
func take_head(now: float) -> FriendData:
	var body := FriendData.new()
	body.name = "Body"
	body.headless = true
	body.look_seed = look_seed
	body.body_fabric = body_fabric
	body.trim = trim
	body.stuffing = stuffing
	body.fluid = fluid
	body.wetness = wetness
	body.mud = mud.filter(func(m): return float(m["y"]) > -110.0)
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		body.parts[slot] = parts.get(slot)
		parts[slot] = null
	body.parts["eye_l"] = null
	body.parts["eye_r"] = null
	head_fabric = head_color()
	mud = mud.filter(func(m): return float(m["y"]) <= -110.0)
	head_only = true
	off_at = now
	asleep = false
	return body


## Sew this head onto a headless body: the friend (this) lives on with
## the body's arms, legs, stuffing and fabric (0026). The body's data is
## used up.
func join_body(body: FriendData) -> void:
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		parts[slot] = body.parts.get(slot)
	body_fabric = body.body_fabric
	trim = body.trim
	stuffing = body.stuffing
	fluid = body.fluid
	wetness = body.wetness
	mud = mud + body.mud
	head_only = false
	off_at = 0.0


## How far gone the body is, 0 (whole) to about 1 (falling apart):
## rot, missing limbs, lost stuffing, soaking. The brain fails only
## above about half (decision 0028).
func decay() -> float:
	var d := 0.0
	for slot in SLOTS:
		var part: PartData = parts.get(slot)
		var weight := 0.5 if slot.begins_with("eye") else 1.0
		if part == null:
			d += 0.25 if weight == 1.0 else 0.05
		else:
			d += part.rot * weight / 5.0
	d += clampf(0.8 - stuffing, 0.0, 0.8) * 0.5
	d += wetness * 0.15
	return d


func mud_amount() -> float:
	var total := 0.0
	for m in mud:
		total += float(m["a"])
	return total


## True if this friend can pass mold on: a rotting part with mold on it.
func spreads_mold() -> bool:
	for slot in SLOTS:
		var part: PartData = parts.get(slot)
		if part and part.rot > 0.6 and not part.mold.is_empty():
			return true
	return false


## Mold lands on one of this friend's parts and starts it rotting.
func catch_mold(rng: RandomNumberGenerator) -> void:
	var limbs: Array = []
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if parts.get(slot) != null:
			limbs.append(slot)
	if limbs.is_empty():
		return
	var part: PartData = parts[limbs[rng.randi() % limbs.size()]]
	part.rot = minf(1.0, part.rot + 0.15)
	part.mold.append([rng.randf_range(0.25, 0.85), rng.randf_range(-0.4, 0.4), rng.randf_range(0.1, 0.18)])


## Time passing for this friend, `hours` at a time (`CampData.advance`).
## Rot needs a source (0011): a tear, mold, soaking, mud, a rotting part
## feeding itself, an open wound nearby; a dirty camp and hunger speed
## it up (0028). A healthy, clean friend doesn't rot at all. Rates are
## guesses (game/README, "Assumed in this build").
func step(hours: float, dirt: float, rng: RandomNumberGenerator) -> void:
	var days := hours / 24.0
	if not met:
		return
	if dead or headless:
		for slot in SLOTS:
			var part: PartData = parts.get(slot)
			if part:
				part.rot = minf(1.0, part.rot + 0.004 * hours)
		return
	stuffing = maxf(0.35, stuffing - days * 0.1)
	fluid = maxf(0.2, fluid - days * 0.12)
	var open := 0
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if parts.get(slot) == null:
			open += 1
	# Open wounds leak stuffing.
	stuffing = maxf(0.2, stuffing - open * 0.02 * hours)
	var hunger := clampf((0.7 - stuffing) / 0.35, 0.0, 1.0)
	var speed := (1.0 + 0.5 * dirt) * (1.0 + 0.5 * hunger)
	var mud_src := minf(mud_amount(), 3.0) / 3.0
	for slot in SLOTS:
		var part: PartData = parts.get(slot)
		if part == null or not part.attached and not slot.begins_with("eye"):
			continue
		var src := 0.0
		if part.torn:
			src += 0.004
		if not part.mold.is_empty():
			src += 0.004
		if part.rot > 0.4:
			src += 0.0015
		src += 0.0015 * maxf(0.0, wetness - 0.3)
		src += 0.001 * mud_src
		src += 0.001 * open
		part.rot = minf(1.0, part.rot + src * speed * hours)
		if part.rot > 0.5 and part.mold.size() < 6 and rng.randf() < 0.02 * hours:
			part.mold.append([rng.randf_range(0.2, 0.9), rng.randf_range(-0.4, 0.4), rng.randf_range(0.1, 0.2)])
	# Mold creeps from a rotting part to another part of the same friend.
	if spreads_mold() and rng.randf() < 0.01 * hours:
		catch_mold(rng)
	# The brain fails only when the body is falling apart.
	var d := decay()
	if d > 0.55:
		brain = maxf(0.0, brain - 0.004 * (d - 0.55) * speed * hours)
		if brain <= 0.0:
			dead = true
			asleep = false


func to_dict() -> Dictionary:
	var pd := {}
	for slot in parts:
		var p: PartData = parts[slot]
		pd[slot] = p.to_dict() if p else null
	return {
		"name": name, "seed": look_seed,
		"body_fabric": body_fabric.to_html(), "trim": trim.to_html(),
		"stuffing": stuffing, "fluid": fluid, "wetness": wetness,
		"mud": mud, "asleep": asleep,
		"tickle_spot": [tickle_spot.x, tickle_spot.y],
		"catch_skill": catch_skill, "parts": pd,
		"stamina": stamina, "met": met,
		"brain": brain, "dead": dead, "head_fabric": head_color().to_html(),
		"headless": headless, "head_only": head_only, "off_at": off_at,
	}


static func from_dict(d: Dictionary) -> FriendData:
	var c := FriendData.new()
	c.name = d.get("name", "Friend")
	c.look_seed = int(d.get("seed", 1))
	c.body_fabric = Color.html(d.get("body_fabric", "b7835a"))
	c.trim = Color.html(d.get("trim", "f0dcc0"))
	c.stuffing = d.get("stuffing", 1.0)
	c.fluid = d.get("fluid", 1.0)
	c.wetness = d.get("wetness", 0.0)
	c.mud = d.get("mud", [])
	c.asleep = d.get("asleep", false)
	var ts: Array = d.get("tickle_spot", [-35, 45])
	c.tickle_spot = Vector2(ts[0], ts[1])
	c.catch_skill = d.get("catch_skill", 0.05)
	c.brain = d.get("brain", 1.0)
	c.stamina = d.get("stamina", 0.0)
	c.met = d.get("met", true)
	c.dead = d.get("dead", false)
	c.head_fabric = Color.html(d.get("head_fabric", c.body_fabric.to_html()))
	c.headless = d.get("headless", false)
	c.head_only = d.get("head_only", false)
	c.off_at = float(d.get("off_at", 0.0))
	var pd: Dictionary = d.get("parts", {})
	for slot in SLOTS:
		var v = pd.get(slot)
		c.parts[slot] = PartData.from_dict(v) if v is Dictionary else null
	return c


## The friend from the first five minutes: passed out, muddy, a bit
## thin, dry joints, a rotting left leg torn open, and the right eye
## hanging by a thread.
static func first_friend() -> FriendData:
	var c := FriendData.new()
	c.name = "First friend"
	c.look_seed = 1
	c.asleep = true
	c.stuffing = 0.72
	c.fluid = 0.35
	var brown := Color(0.72, 0.51, 0.35)
	for slot in SLOTS:
		c.parts[slot] = PartData.make(slot, brown, 0, 0.5)
	var eye_color := Color(0.17, 0.13, 0.19)
	c.parts["eye_l"].fabric = eye_color
	c.parts["eye_r"].fabric = eye_color
	c.parts["eye_r"].attached = false
	var leg: PartData = c.parts["leg_l"]
	leg.rot = 0.9
	leg.torn = true
	leg.fabric = Color(0.55, 0.52, 0.36)
	leg.mold = [[0.35, -0.4, 0.16], [0.6, 0.3, 0.2], [0.8, -0.15, 0.13], [0.2, 0.35, 0.11]]
	c.mud = [
		{"x": 40.0, "y": -40.0, "r": 26.0, "a": 1.0},
		{"x": -60.0, "y": 70.0, "r": 30.0, "a": 1.0},
		{"x": 70.0, "y": 60.0, "r": 20.0, "a": 1.0},
		{"x": 10.0, "y": 95.0, "r": 24.0, "a": 1.0},
		{"x": -20.0, "y": -230.0, "r": 22.0, "a": 1.0},
		{"x": 60.0, "y": -150.0, "r": 18.0, "a": 1.0},
		{"x": -95.0, "y": -10.0, "r": 18.0, "a": 1.0},
	]
	return c


## The second friend of the first day (Mikey's recording 1): nothing
## wrong but mud and a bit too little stuffing.
static func second_friend() -> FriendData:
	var c := FriendData.new()
	c.name = "Second friend"
	c.look_seed = 2
	c.body_fabric = Color(0.56, 0.6, 0.7)
	c.trim = Color(0.93, 0.9, 0.84)
	c.stuffing = 0.68
	c.fluid = 0.8
	c.tickle_spot = Vector2(38, 20)
	for slot in SLOTS:
		c.parts[slot] = PartData.make(slot, c.body_fabric, 0, 0.5)
	c.parts["eye_l"].fabric = Color(0.12, 0.1, 0.12)
	c.parts["eye_r"].fabric = Color(0.12, 0.1, 0.12)
	c.mud = [
		{"x": -40.0, "y": 30.0, "r": 28.0, "a": 1.0},
		{"x": 55.0, "y": 85.0, "r": 22.0, "a": 1.0},
		{"x": 25.0, "y": -215.0, "r": 20.0, "a": 1.0},
	]
	return c


## A healthy test friend for the debug flag `friends=N`, its look and
## fabric from `seed`.
static func test_friend(seed_value: int) -> FriendData:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var c := FriendData.new()
	c.name = "Test friend %d" % seed_value
	c.look_seed = seed_value
	c.body_fabric = Color.from_hsv(rng.randf(), rng.randf_range(0.2, 0.5), rng.randf_range(0.5, 0.8))
	c.trim = Color.from_hsv(rng.randf(), rng.randf_range(0.05, 0.2), rng.randf_range(0.85, 0.97))
	c.stuffing = rng.randf_range(0.7, 1.05)
	c.tickle_spot = Vector2(rng.randf_range(-60, 60), rng.randf_range(-10, 80))
	for slot in SLOTS:
		c.parts[slot] = PartData.make(slot, c.body_fabric, rng.randi_range(0, 2) if slot.begins_with("leg") or slot.begins_with("arm") else 0, 0.5)
	c.parts["eye_l"].fabric = Color(0.12, 0.1, 0.12)
	c.parts["eye_r"].fabric = Color(0.12, 0.1, 0.12)
	return c


## A newcomer turning up at camp (0034): its own look, and one or two
## things wrong with it, mild to bad (Mikey: muddy and skinny, or no
## legs and sad, or anything between).
static func newcomer(seed_value: int) -> FriendData:
	var c := test_friend(seed_value)
	c.name = "Newcomer"
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value + 7
	var problems := ["mud", "thin", "soaked", "rotting", "missing", "eye", "out"]
	problems.shuffle()
	for p in problems.slice(0, 1 + rng.randi() % 2):
		match p:
			"mud":
				for i in 3:
					c.mud.append({"x": rng.randf_range(-80, 80), "y": rng.randf_range(-60, 110), "r": rng.randf_range(16, 28), "a": 1.0})
			"thin":
				c.stuffing = rng.randf_range(0.45, 0.65)
			"soaked":
				c.wetness = rng.randf_range(0.7, 1.0)
			"rotting":
				var part: PartData = c.parts[["arm_l", "arm_r", "leg_l", "leg_r"][rng.randi() % 4]]
				part.rot = rng.randf_range(0.6, 0.85)
				part.torn = true
				part.mold = [[0.4, 0.2, 0.15], [0.7, -0.2, 0.12]]
			"missing":
				c.parts[["arm_l", "arm_r", "leg_l", "leg_r"][rng.randi() % 4]] = null
			"eye":
				c.parts["eye_r"].attached = false
			"out":
				c.asleep = true
	return c


## A friend met on a quest and brought home (0034): something is
## usually wrong with it, as with any newcomer.
static func found_friend(seed_value: int) -> FriendData:
	var c := test_friend(seed_value)
	c.name = "Found friend"
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value + 1
	match rng.randi() % 4:
		0:
			c.mud = [{"x": -30.0, "y": 40.0, "r": 26.0, "a": 1.0}, {"x": 50.0, "y": -10.0, "r": 20.0, "a": 1.0}]
		1:
			c.stuffing = 0.55
		2:
			c.wetness = 0.9
		3:
			var part: PartData = c.parts["arm_r"]
			part.torn = true
			part.rot = 0.3
	return c


## A spare leg, waiting on the side. Different fabric: a repaired
## friend is never the same as before.
static func spare_leg() -> PartData:
	var p := PartData.make("leg_l", Color(0.37, 0.5, 0.66), 1, 0.6)
	p.attached = false
	return p
