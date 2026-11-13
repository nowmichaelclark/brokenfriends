class_name TentTools
extends RefCounted
## Using the things on the shelf: the sponge, towel, stuffing, oil and
## mold spray in the grooming screen, and thread on the surgery table,
## dragged to an open hole to sew it shut (Mikey's playtest notes).

var t: Tent

var _sprayed := false


## While a tool is dragged over the friend (`l`, the friend's units).
func use(tool: String, l: Vector2) -> void:
	var friend := t.friend
	if tool == "sponge" and not friend.is_on_friend(l):
		# Scrub footprints off the floor.
		if t.traces.scrub(friend.to_global(l), 0.08) and t.sound_cd <= 0.0:
			Voice.squish()
			t.sound_cd = 0.35
			Save.mark_dirty()
		return
	if Save.friend.dead:
		return
	match tool:
		"spray":
			if friend.spray(l):
				_sprayed = true
				Voice.poof()
		"sponge":
			if friend.sponge(l) and t.sound_cd <= 0.0:
				Voice.splash()
				t.sound_cd = 0.35
		"towel":
			if friend.towel(l) and t.sound_cd <= 0.0:
				Voice.squish()
				t.sound_cd = 0.3
		"stuffing", "oil":
			if l.distance_to(friend.mouth_pos()) < 80.0 and not Save.friend.asleep:
				friend.set_expression("open", "open", 0.15)


## The tool is let go of at `l`.
func finish(tool: String, l: Vector2) -> void:
	var friend := t.friend
	if Save.friend.dead:
		return
	match tool:
		"stuffing", "oil":
			if l.distance_to(friend.mouth_pos()) < 80.0:
				friend.wake()
				if not Save.camp.use(tool):
					t.hud.shake(tool)
				elif tool == "stuffing":
					friend.feed_stuffing()
				else:
					friend.feed_oil()
		"spray":
			if _sprayed:
				Save.camp.use("spray")
				Save.mark_dirty()
			_sprayed = false
		"sponge":
			if Save.friend.mud.is_empty():
				Events.cared.emit("washed")
		"towel":
			if Save.friend.wetness < 0.05:
				Events.cared.emit("dried")
		"thread":
			var slot := open_hole_near(l)
			if slot != "" and Save.camp.use("thread"):
				Save.mark_dirty()
				t.surgery.start_shut(slot)


## An open limb hole near a point (the friend's units), or "".
func open_hole_near(l: Vector2) -> String:
	if Save.friend.headless:
		return ""
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		if Save.friend.is_open(slot) and l.distance_to(t.friend.socket(slot)) < 90.0:
			return slot
	return ""
