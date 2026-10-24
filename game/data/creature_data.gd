class_name CreatureData
extends Resource
## A creature, as data. Mostly fluff inside, with hydraulic ligaments,
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
	return clampf(w, 0.0, 1.0)


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
	}


static func from_dict(d: Dictionary) -> CreatureData:
	var c := CreatureData.new()
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
	var pd: Dictionary = d.get("parts", {})
	for slot in SLOTS:
		var v = pd.get(slot)
		c.parts[slot] = PartData.from_dict(v) if v is Dictionary else null
	return c


## The creature from the first five minutes: passed out, muddy, a bit
## thin, dry joints, a rotting left leg torn open, and the right eye
## hanging by a thread.
static func first_friend() -> CreatureData:
	var c := CreatureData.new()
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


## A spare leg, waiting on the side. Different fabric: a repaired
## friend is never the same as before.
static func spare_leg() -> PartData:
	var p := PartData.make("leg_l", Color(0.37, 0.5, 0.66), 1, 0.6)
	p.attached = false
	return p
