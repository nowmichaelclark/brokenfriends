class_name TentParts
extends RefCounted
## Parts and heads off the friend, in the tent: picked up, dropped,
## kept in the pocket, thrown in the bin, or brought to a wound or
## a neck to be sewn on (decisions 0025 and 0026).

var t: Tent


# --- Loose parts, the bin and the spare ---------------------------------

func spawn_loose(part: PartData, w: Vector2, pocket: bool) -> LoosePart:
	var lp := LoosePart.new()
	lp.part = part
	lp.trim = Save.friend.trim
	lp.from_pocket = pocket
	lp.floor_y = t.fx.floor_y
	lp.position = w
	t.parts_layer.add_child(lp)
	if not pocket:
		t.loose_parts.append(lp)
	return lp


func release_part(sp: Vector2, w: Vector2) -> void:
	var lp := t.held
	t.held = null
	t.hud.trash_hover = false
	t.hud.trash_visible = false
	t.hud.pocket_open = false
	if lp == null:
		return
	if not lp.from_pocket and t.hud.over_pocket(sp):
		if lp.part.is_dead():
			# Past repair (0025): only the bin will take it.
			t.hud.shake("")
			lp.drop(Vector2.ZERO)
			Voice.thud()
			return
		# Keep it for another friend (donation, 0025).
		t.loose_parts.erase(lp)
		lp.part.pulls = 0
		Save.camp.spares.push_front(lp.part)
		lp.queue_free()
		Save.mark_dirty()
		show_spare()
		Voice.pop()
		return
	if lp.from_pocket:
		var slot := _open_slot_near(w, lp.part.kind)
		lp.queue_free()
		if slot != "" and Save.camp.use("thread"):
			Save.mark_dirty()
			t.surgery.start(slot, lp.part)
		elif slot != "":
			# No thread: the spare goes back to the pocket (0042).
			t.hud.shake("thread")
			show_spare()
			Voice.thud()
		else:
			show_spare()
			Voice.pop()
		return
	if t.hud.over_trash(sp) or sp.distance_to(t.hud.trash_center()) < 120.0:
		_trash(lp)
	else:
		lp.drop(t.drag_vel * 0.6)
		Voice.thud()


func _open_slot_near(w: Vector2, kind: String) -> String:
	for slot in ["leg_l", "leg_r", "arm_l", "arm_r"]:
		if not slot.begins_with(kind):
			continue
		if Save.friend.is_missing(slot) and t.friend.to_global(t.friend.socket(slot)).distance_to(w) < 140.0:
			return slot
	return ""


func _trash(lp: LoosePart) -> void:
	t.loose_parts.erase(lp)
	var target := t.to_world(t.hud.trash_center())
	var tw := t.create_tween().set_parallel()
	tw.tween_property(lp, "position", target, 0.25).set_trans(Tween.TRANS_SINE)
	tw.tween_property(lp, "scale", Vector2(0.15, 0.15), 0.25)
	tw.chain().tween_callback(lp.queue_free)
	Voice.clunk()
	Events.part_changed.emit(lp.part.slot, "trashed")
	offer_spare()


## Once the first rotten leg is gone, a spare waits on the side. It
## comes once: after that, parts are found on quests (decision 0027).
func offer_spare() -> void:
	if Save.camp.spare_given:
		return
	for slot in ["leg_l", "leg_r"]:
		if Save.friend.is_open(slot):
			Save.camp.spare_given = true
			Save.camp.spares.append(FriendData.spare_leg())
			Save.mark_dirty()
			show_spare()
			return


## Show the first spare part in the pocket, if there is one, and any
## head kept there.
func show_spare() -> void:
	var spares := Save.camp.spares
	t.hud.spare_part = spares[0] if not spares.is_empty() else null
	t.hud.spare_visible = not spares.is_empty()
	t.hud.spare_count = spares.size()
	t.hud.pocket_head = Save.camp.heads[0] if not Save.camp.heads.is_empty() else null


# --- Heads (decision 0026) ----------------------------------------------

func spawn_head(f: FriendData, w: Vector2) -> LooseHead:
	var lh := LooseHead.new()
	lh.friend = f
	lh.position = w
	t.parts_layer.add_child(lh)
	return lh


## The head has come off: the friend lives on in the head, now in the
## player's hand; what stays is a body without a head.
func head_off(w: Vector2) -> void:
	var head := Save.friend
	var body := head.take_head(Save.now())
	Save.camp.friends[Save.camp.current] = body
	t.friend.setup(body)
	t.fx.spawn("fluff", t.fx.to_local(t.friend.to_global(t.friend.socket("neck"))), 20, 22.0)
	Voice.tear(1.0)
	Events.part_changed.emit("head", "torn")
	t.held_head = spawn_head(head, w)
	t.mode = "head_held"
	t.hud.trash_visible = true
	t.hud.pocket_open = true
	t.build_bench()
	Save.mark_dirty()


func release_head(sp: Vector2, w: Vector2) -> void:
	var lh := t.held_head
	t.held_head = null
	t.hud.trash_hover = false
	t.hud.trash_visible = false
	t.hud.pocket_open = false
	t.mode = ""
	if lh == null:
		return
	var head := lh.friend
	lh.queue_free()
	var heads := Save.camp.heads
	if t.hud.over_trash(sp) or sp.distance_to(t.hud.trash_center()) < 120.0:
		# Thrown away: that friend is gone (0026).
		heads.erase(head)
		Voice.clunk()
		Events.part_changed.emit("head", "trashed")
	elif Save.friend.headless and w.distance_to(t.friend.to_global(t.friend.socket("neck"))) < 150.0:
		t.surgery.start_head(head)
	elif not head in heads:
		heads.append(head)
		Voice.pop()
	Save.mark_dirty()
	show_spare()
