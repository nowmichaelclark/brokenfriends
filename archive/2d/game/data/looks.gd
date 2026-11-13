class_name Looks
extends RefCounted
## How a friend looks, made by maths from a seed (decision 0037),
## weighted the way Driftloom weights its choices: some things common,
## some rare. The tables below are the developer's sliders: change a
## weight or a range and every new friend follows.
##
## A look is two plain dictionaries, so it saves as JSON and splits
## cleanly when heads are swapped (0026): `head_look` goes with the
## head, `body_look` stays with the body.

## Body plans and how often each turns up.
const PLANS := {"bear": 30, "bunny": 18, "cat": 18, "monster": 14, "alien": 10, "ghosty": 10}

## Per plan: ears, ranges for sizes, nose, tail, and colours (hue range,
## saturation range, value range). Ghost-like friends are plush designs,
## never actual ghosts (Mikey, 2026-10-25).
const PLAN := {
	"bear": {"ears": {"round": 1}, "ear_size": [0.85, 1.2], "muzzle": [0.85, 1.1], "noses": {"button": 1},
		"head_w": [0.95, 1.1], "head_h": [0.92, 1.02], "tails": {"stub": 3, "none": 1},
		"hue": [0.03, 0.12], "sat": [0.25, 0.5], "val": [0.45, 0.78]},
	"bunny": {"ears": {"long": 3, "floppy": 2}, "ear_size": [0.95, 1.35], "muzzle": [0.55, 0.75], "noses": {"heart": 2, "triangle": 1},
		"head_w": [0.9, 1.0], "head_h": [0.95, 1.05], "tails": {"stub": 1},
		"hue": [0.0, 1.0], "sat": [0.05, 0.3], "val": [0.7, 0.95]},
	"cat": {"ears": {"pointy": 1}, "ear_size": [0.85, 1.2], "muzzle": [0.5, 0.7], "noses": {"triangle": 1},
		"head_w": [1.0, 1.12], "head_h": [0.88, 0.98], "tails": {"long": 1},
		"hue": [0.0, 1.0], "sat": [0.0, 0.45], "val": [0.3, 0.9]},
	"monster": {"ears": {"horns": 2, "pointy": 1}, "ear_size": [0.8, 1.2], "muzzle": [1.05, 1.3], "noses": {"none": 1, "button": 1},
		"head_w": [1.05, 1.18], "head_h": [0.84, 0.95], "tails": {"curly": 2, "long": 1},
		"hue": [0.0, 1.0], "sat": [0.4, 0.7], "val": [0.4, 0.75]},
	"alien": {"ears": {"antennae": 1}, "ear_size": [0.8, 1.3], "muzzle": [0.0, 0.0], "noses": {"none": 1},
		"head_w": [0.95, 1.05], "head_h": [1.04, 1.14], "tails": {"none": 1},
		"hue": [0.2, 0.55], "sat": [0.25, 0.55], "val": [0.55, 0.85]},
	"ghosty": {"ears": {"none": 1}, "ear_size": [1.0, 1.0], "muzzle": [0.0, 0.0], "noses": {"none": 1},
		"head_w": [0.95, 1.02], "head_h": [1.06, 1.14], "tails": {"none": 1},
		"hue": [0.5, 0.8], "sat": [0.03, 0.15], "val": [0.86, 0.97]},
}

## Rare touches, each with its chance.
const CHEEKS := 0.25
const PATCH := 0.15
## Voice: pitch range, and how many voice timbres exist (Voice has a
## bank for each).
const PITCH := [0.72, 1.45]
const TIMBRES := 3


static func _pick(rng: RandomNumberGenerator, weights: Dictionary) -> String:
	var total := 0
	for k in weights:
		total += int(weights[k])
	var n := rng.randi() % total
	for k in weights:
		n -= int(weights[k])
		if n < 0:
			return k
	return weights.keys()[0]


static func _range(rng: RandomNumberGenerator, r: Array) -> float:
	return rng.randf_range(float(r[0]), float(r[1]))


## A whole new look from a seed: the head's, the body's and the colours.
static func generate(seed_value: int) -> Dictionary:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value * 7919 + 13
	var plan := _pick(rng, PLANS)
	var p: Dictionary = PLAN[plan]
	var fabric := Color.from_hsv(_range(rng, p["hue"]), _range(rng, p["sat"]), _range(rng, p["val"]))
	var trim := Color.from_hsv(rng.randf(), rng.randf_range(0.04, 0.25), rng.randf_range(0.85, 0.97))
	if plan == "ghosty":
		trim = fabric.lightened(0.5)
	var head := {
		"plan": plan,
		"w": _range(rng, p["head_w"]), "h": _range(rng, p["head_h"]),
		"ears": _pick(rng, p["ears"]), "ear_size": _range(rng, p["ear_size"]),
		"muzzle": _range(rng, p["muzzle"]), "nose": _pick(rng, p["noses"]),
		"eye_gap": rng.randf_range(32.0, 46.0) * (1.12 if plan == "alien" else 1.0),
		"eye_size": rng.randf_range(0.85, 1.15) * (1.3 if plan == "alien" else 1.0),
		"eye_y": rng.randf_range(-22.0, -4.0),
		"cheeks": rng.randf() < CHEEKS or plan == "ghosty",
		"patch": rng.randf() < PATCH,
		"voice": rng.randf_range(float(PITCH[0]), float(PITCH[1])),
		"timbre": rng.randi() % TIMBRES,
	}
	var body := {
		"plan": plan,
		"w": rng.randf_range(0.88, 1.18), "h": rng.randf_range(0.92, 1.08),
		"belly": rng.randf_range(0.55, 1.0), "tail": _pick(rng, p["tails"]),
	}
	return {"head": head, "body": body, "fabric": fabric, "trim": trim}


## The look of the sketch's first bears, for friends with no look saved.
static func bear_head() -> Dictionary:
	return {"plan": "bear", "w": 1.0, "h": 0.94 / 0.94, "ears": "round", "ear_size": 1.0, "muzzle": 1.0,
		"nose": "button", "eye_gap": 40.0, "eye_size": 1.0, "eye_y": -12.0, "cheeks": false, "patch": false,
		"voice": 1.0, "timbre": 0}


static func bear_body() -> Dictionary:
	return {"plan": "bear", "w": 1.0, "h": 1.0, "belly": 1.0, "tail": "stub"}
