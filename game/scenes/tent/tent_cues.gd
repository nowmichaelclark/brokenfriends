class_name TentCues
extends RefCounted
## What could be done next in the tent: as words for the developer
## (`#hints`), and without words for the player (decision 0032).


## Gentle guidance for the first five minutes. (Later the body alone
## should say all of this; pillar 1.)
static func hint(t: Tent) -> String:
	var c := Save.friend
	if t.held_head:
		return "Put the head on a body without one, keep it in the pocket, or bin it."
	if c.dead:
		return "This friend has died. Pull off any parts worth keeping and drop them in the pocket."
	if c.headless:
		return "A body without a head. Sew a head on, or take its parts."
	if t.held and not t.held.from_pocket and t.held.part and not t.held.part.is_dead() and t.held.part.rot < 0.5:
		return "Drop it in the pocket to keep it for another friend, or bin it."
	if Save.camp.bugs > 0:
		return "Bugs! Tap them to chase them off."
	if not Save.camp.tracks.is_empty() and t.mode != "surgery" and c.part("leg_l") and c.part("leg_l").rot < 0.5:
		return "Muddy footprints. Scrub the floor with the sponge."
	if t.mode == "surgery":
		var s := t.friend.surgery
		if s.get("eye", false):
			return "Stitch the eye back on: follow the glowing points."
		match s.get("step", ""):
			"fluff": return "Pull away the loose fluff around the wound."
			"tube": return "Connect the ligament tube to the port."
			"soak": return "Soak up the spilled fluid with the cloth."
			"sew": return "Sew it on: follow the glowing points."
	if c.asleep:
		return "Your friend isn't well. Poke them gently to wake them."
	if t.held and not t.held.from_pocket:
		return "Toss it in the bin."
	if t.held and t.held.from_pocket:
		return "Bring it to the wound."
	var leg: PartData = c.part("leg_l")
	if leg and leg.rot > 0.5:
		return "That leg is rotting. Grab it and pull hard."
	if not t.loose_parts.is_empty():
		return "Throw the rotten leg in the bin."
	if t.hud.spare_visible:
		return "A spare leg is waiting on the right. Drag it onto the wound."
	var eye: PartData = c.part("eye_r")
	if eye and not eye.attached:
		return "An eye is hanging by a thread. Drag it back into place."
	if not c.mud.is_empty():
		return "Rub the mud off with the sponge."
	if c.wetness > 0.3:
		return "They're soaked. Dry them with the towel."
	if c.stuffing < 0.9:
		return "They lost stuffing. Drag some to their mouth."
	if c.fluid < 0.6:
		return "Stiff, squeaky joints. Give them a little oil."
	return "All patched up. Poke, tickle or pet them, or visit the play corner."


## Show what could be done next, without words (decision 0032): a glow
## on the shelf or pocket or bin, a pulsing ring on the friend, a dotted
## line to drag along. The same order of things as `hint`.
static func show(t: Tent) -> void:
	t.guide.clear()
	t.hud.glow = ""
	var c := Save.friend
	if c == null or t.on_table and t.mode != "surgery":
		return
	if t.held_head:
		t.hud.glow = "pocket"
		return
	if c.dead:
		return
	if c.headless:
		if t.hud.pocket_head:
			t.hud.glow = "pocket"
			_cue(t, t.friend.socket("neck"), 50.0)
		return
	if t.held and not t.held.from_pocket and t.held.part and not t.held.part.is_dead() and t.held.part.rot < 0.5:
		t.hud.glow = "pocket"
		return
	if t.mode == "surgery":
		var srg := t.friend.surgery
		match srg.get("step", ""):
			"fluff":
				var tufts: Array = t.friend.wounds.get(srg["slot"], [])
				if not tufts.is_empty():
					_cue(t, t.friend.socket(srg["slot"]) + tufts[0], 18.0)
			"tube":
				_cue(t, srg.get("tube_end", Vector2.ZERO), 16.0, t.friend.socket(srg["slot"]) + Vector2(10, 0))
			"soak":
				_cue(t, srg.get("spill_pos", Vector2.ZERO), 30.0)
		return
	if Save.camp.bugs > 0 and not t.traces._bugs.is_empty():
		t.guide.show_cue(t.traces._bugs[0].global_position, 26.0)
		return
	if c.asleep:
		_cue(t, Vector2(40, 10), 60.0)
		return
	if t.held and not t.held.from_pocket:
		t.hud.glow = "bin"
		return
	if t.held and t.held.from_pocket:
		for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
			if c.is_open(slot) and slot.begins_with(t.held.part.kind):
				_cue(t, t.friend.socket(slot), 40.0)
				return
		return
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		var part: PartData = c.part(slot)
		if part and part.rot > 0.5 and part.attached:
			var mid := t.friend.socket(slot).lerp(t.friend.tip(slot), 0.6)
			_cue(t, mid, 40.0, mid + t.friend.limb_dir(slot) * 140.0)
			return
	if not t.loose_parts.is_empty():
		t.hud.glow = "bin"
		t.guide.show_cue(t.loose_parts[0].global_position, 50.0)
		return
	if t.hud.spare_visible:
		for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
			if c.is_open(slot) and t.hud.spare_part and slot.begins_with(t.hud.spare_part.kind):
				t.hud.glow = "pocket"
				_cue(t, t.friend.socket(slot), 40.0)
				return
	var eye: PartData = c.part("eye_r")
	if eye and not eye.attached and Save.camp.has("thread"):
		_cue(t, t.friend.eye_position(), 26.0, t.friend.eye_anchor("eye_r"))
		return
	if not c.mud.is_empty():
		t.hud.glow = "sponge"
		return
	if not Save.camp.tracks.is_empty():
		t.hud.glow = "sponge"
		var track: Dictionary = Save.camp.tracks[0]
		t.guide.show_cue(Vector2(float(track["x"]), float(track["y"])), 40.0)
		return
	if c.wetness > 0.3:
		t.hud.glow = "towel"
		return
	if c.stuffing < 0.9 and Save.camp.has("stuffing"):
		t.hud.glow = "stuffing"
		_cue(t, t.friend.mouth_pos(), 30.0)
		return
	if c.fluid < 0.6 and Save.camp.has("oil"):
		t.hud.glow = "oil"
		_cue(t, t.friend.mouth_pos(), 30.0)
		return
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		var part: PartData = c.part(slot)
		if part and not part.mold.is_empty() and Save.camp.has("spray"):
			t.hud.glow = "spray"
			return


## A cue at a point on the friend (its own units), maybe with a drag.
static func _cue(t: Tent, local: Vector2, r: float, drag_local := Vector2.INF) -> void:
	var to := t.friend.to_global(drag_local) if drag_local != Vector2.INF else Vector2.INF
	t.guide.show_cue(t.friend.to_global(local), r * t.friend.global_scale.x, to)
