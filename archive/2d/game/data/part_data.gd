class_name PartData
extends Resource
## One piece of a friend: a limb or a button eye. Parts carry their
## own fabric and quality, and they keep the marks of what happened to
## them (pillar 2: the friend is never the same as before).

## Where it goes: "arm_l", "arm_r", "leg_l", "leg_r", "eye_l", "eye_r".
@export var slot := ""
## "arm", "leg" or "eye".
@export var kind := ""
## 0..1. Better parts last longer and help on quests (later).
@export var quality := 0.5
@export var fabric := Color(0.72, 0.51, 0.35)
## 0 plain, 1 stripes, 2 dots, 3 patchwork.
@export var pattern := 0
## 0..1. Rot needs a source (decision 0011); here it starts as given.
@export var rot := 0.0
## Mold spots, in the part's own 0..1 space along and across it.
@export var mold: Array = []
## A rip in the fabric with fluff poking out.
@export var torn := false
## False while hanging by a thread (an eye) or not yet sewn on.
@export var attached := true
## True once sewn on in surgery: the stitches stay for good.
@export var stitched := false
## Hard pulls it has taken. A healthy part holds for a few, then comes
## off (decision 0025); a loosened part hangs a little lower.
@export var pulls := 0


## Rotted or worn past repair (0025): it can only go in the bin.
func is_dead() -> bool:
	return rot >= 0.95


## Hard pulls it takes to come off: about three, more for a strong part.
func pulls_needed() -> int:
	return 3 + (1 if quality >= 0.6 else 0) + (1 if quality >= 0.85 else 0)


func to_dict() -> Dictionary:
	return {
		"slot": slot, "kind": kind, "quality": quality,
		"fabric": fabric.to_html(), "pattern": pattern, "rot": rot,
		"mold": mold, "torn": torn, "attached": attached, "stitched": stitched,
		"pulls": pulls,
	}


static func from_dict(d: Dictionary) -> PartData:
	var p := PartData.new()
	p.slot = d.get("slot", "")
	p.kind = d.get("kind", "")
	p.quality = d.get("quality", 0.5)
	p.fabric = Color.html(d.get("fabric", "b7835a"))
	p.pattern = int(d.get("pattern", 0))
	p.rot = d.get("rot", 0.0)
	p.mold = d.get("mold", [])
	p.torn = d.get("torn", false)
	p.attached = d.get("attached", true)
	p.stitched = d.get("stitched", false)
	p.pulls = int(d.get("pulls", 0))
	return p


static func make(slot_name: String, color: Color, pattern_id := 0, q := 0.5) -> PartData:
	var p := PartData.new()
	p.slot = slot_name
	p.kind = slot_name.split("_")[0]
	p.fabric = color
	p.pattern = pattern_id
	p.quality = q
	return p
