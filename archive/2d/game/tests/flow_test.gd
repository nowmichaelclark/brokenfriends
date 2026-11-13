extends "res://tests/flow_world.gd"
## Plays through the first five minutes with real touch events and
## saves screenshots. Run with a display (or Xvfb):
##   godot --path game -- --flow-test=/tmp/shots
## Prints FLOW OK, or FLOW FAILED with what went wrong, then quits.


func _ready() -> void:
	await get_tree().process_frame
	main = get_parent()
	tent = main.tent
	await _run()


func _run() -> void:
	if "--physics-only" in OS.get_cmdline_user_args():
		await _physics_steps()
		_finish()
		return
	var c: FriendView = tent.friend
	# The entrance first: the friend standing outside, the surgery
	# table's button glowing (a rotting leg; Mikey's playtest notes).
	var entrance: Entrance = main.entrance
	await _wait(0.8)
	_check(entrance.visible and entrance.standing.size() == 1, "the game opens at the entrance, the friend outside")
	_check(entrance.hud.glow == "table", "the entrance glows the way to the surgery table")
	await _shot("00_entrance")
	await _tap(entrance.hud.go_center(1))
	await _wait(0.6)
	_check(tent.visible and tent.rooms.on_table(), "the needle opens the surgery table")
	await _shot("01_start")

	# Wake them with a gentle poke on the body.
	await _tap(_screen(c.to_global(Vector2(40, 10))))
	await _wait(1.6)
	_check(not Save.friend.asleep, "poke wakes the friend")
	await _shot("02_awake")

	# Tickle spot.
	var spot := Vector2(Save.friend.tickle_spot.x * c.body_scale(), Save.friend.tickle_spot.y)
	await _tap(_screen(c.to_global(spot)))
	await _wait(0.3)
	await _shot("03_tickle")

	# Grab the rotting leg and pull it off.
	var leg_mid := c.to_global(c.socket("leg_l").lerp(c.tip("leg_l"), 0.6))
	var start := _screen(leg_mid)
	await _touch(start, true)
	var torn := false
	for i in 40:
		var p := start + Vector2(-6.0 * i, 6.0 * i)
		await _drag_to(p)
		if i == 14:
			await _shot("04_tugging")
		if tent.mode == "part":
			torn = true
			break
	_check(torn, "pulling the rotten leg tears it off")
	_check(Save.friend.is_open("leg_l"), "leg slot is an open wound")
	await _wait(0.2)
	await _shot("05_torn")

	# Into the bin.
	var bin: Vector2 = tent.hud.trash_center()
	await _drag_path(_last, bin, 12)
	await _shot("06_over_bin")
	await _touch(bin, false)
	await _wait(0.6)
	_check(tent.hud.spare_visible, "a spare leg waits on the side after the bin")
	await _shot("07_spare_waiting")

	# Drag the spare onto the wound.
	var pocket: Vector2 = tent.hud.pocket_center()
	await _touch(pocket, true)
	var wound := _screen(c.to_global(c.socket("leg_l") + Vector2(0, 30)))
	await _drag_path(pocket, wound, 14)
	await _touch(wound, false)
	await _wait(0.7)
	_check(tent.mode == "surgery", "dropping the spare on the wound starts surgery")
	await _shot("08_surgery_fluff")

	# Step 1: pluck the loose fluff.
	var tufts: Array = c.wounds.get("leg_l", []).duplicate()
	var sock: Vector2 = c.socket("leg_l")
	for t in tufts:
		var sp := _screen(c.to_global(sock + t))
		await _tap(sp)
	await _wait(0.2)
	_check(c.surgery.get("step", "") == "tube", "clearing the fluff moves on to the tube")
	await _shot("09_tube")

	# Step 2: connect the ligament tube.
	var end: Vector2 = c.surgery.get("tube_end", Vector2.ZERO)
	var port: Vector2 = c.socket("leg_l") + Vector2(10, 0)
	await _touch(_screen(c.to_global(end)), true)
	await _drag_path(_screen(c.to_global(end)), _screen(c.to_global(port)), 10)
	await _touch(_screen(c.to_global(port)), false)
	await _wait(0.3)
	_check(c.surgery.get("step", "") == "soak", "connecting the tube moves on to soaking")
	await _shot("10_spill")

	# Step 3: soak up the spill.
	var spill: Vector2 = c.surgery.get("spill_pos", Vector2.ZERO)
	var spill_s := _screen(c.to_global(spill))
	await _touch(spill_s, true)
	for i in 60:
		await _drag_to(spill_s + Vector2(sin(i * 0.7) * 40.0, cos(i * 0.5) * 14.0))
		if c.surgery.get("step", "") != "soak":
			break
	await _touch(_last, false)
	_check(c.surgery.get("step", "") == "sew", "soaking the spill moves on to sewing")
	await _wait(0.2)
	await _shot("11_sew")

	# Step 4: sew along the points.
	var pts: Array = c.surgery.get("points", [])
	if pts.size() > 0:
		var first := _screen(c.to_global(pts[0]))
		await _touch(first, true)
		for i in range(1, pts.size()):
			await _drag_path(_last, _screen(c.to_global(pts[i])), 5)
			if i == 3:
				await _shot("12_sewing")
		await _touch(_last, false)
	await _wait(0.8)
	var leg: PartData = Save.friend.part("leg_l")
	_check(leg != null and leg.attached and leg.stitched, "the spare leg is sewn on with stitches")
	await _shot("13_leg_sewn")

	# The dangling eye.
	var eye_at := _screen(c.to_global(c.eye_position()))
	await _touch(eye_at, true)
	var anchor := _screen(c.to_global(c.eye_anchor("eye_r")))
	await _drag_path(eye_at, anchor, 10)
	await _touch(anchor, false)
	await _wait(0.7)
	_check(tent.mode == "surgery", "putting the eye back starts a stitch")
	await _shot("14_eye_sew")
	pts = c.surgery.get("points", [])
	if pts.size() > 0:
		await _touch(_screen(c.to_global(pts[0])), true)
		for i in range(1, pts.size()):
			await _drag_path(_last, _screen(c.to_global(pts[i])), 5)
		await _touch(_last, false)
	await _wait(0.8)
	_check(Save.friend.part("eye_r").attached, "the eye is stitched back on")
	await _wait(0.6)
	_check(tent.hud.glow == "room", "with the surgery done, the way to grooming glows")
	await _shot("14b_table_upright")

	# To the grooming screen: wash, dry, feed.
	await _tap(tent.hud.room_rect().get_center())
	await _wait(0.6)
	_check(not tent.rooms.on_table() and tent.hud.tools().has("sponge"), "the sponge goes to the grooming screen")
	# Wash with the sponge: scrub each mud spot.
	await _scrub_mud(c)
	_check(Save.friend.mud.is_empty(), "the sponge washes off all the mud")
	await _shot("15_washed_wet")
	# Dry with the towel.
	await _use_tool(1, c, 160)
	_check(Save.friend.wetness < 0.3, "the towel dries them")
	# Feed stuffing twice and oil once.
	for i in 2:
		await _feed(2, c)
	await _feed(3, c)
	await _wait(1.2)
	_check(Save.friend.stuffing >= 0.9, "stuffing refills them")
	_check(Save.friend.fluid > 0.9, "oil loosens the joints")
	await _shot("16_all_good")
	var sup: Dictionary = Save.camp.supplies
	_check(sup["thread"] == 2 and sup["stuffing"] == 4 and sup["oil"] == 2,
		"two sewings, two handfuls of stuffing and one dose of oil came off the shelf")
	print("wellbeing at end of tent: ", Save.friend.wellbeing())

	# Out to the entrance, then the play room.
	await _tap(tent.hud.entrance_rect().get_center())
	await _wait(0.6)
	_check(entrance.visible, "the tent's door goes back out to the entrance")
	await _tap(entrance.hud.go_center(2))
	await _wait(1.0)
	var play: PlayRoom = main.play
	_check(play.visible, "the play room opens")
	await _shot("17_play_room")
	var ball: RigidBody2D = play.ball
	var me := play.wanderer_of(Save.friend)
	me.hold = true
	for throw in 4:
		await _wait(1.2)
		# The ball near them, in view.
		ball.freeze = true
		ball.global_position = me.view.position + Vector2(160, -200)
		ball.freeze = false
		play.camera.position = me.view.position
		await _wait(0.3)
		var b := _screen(ball.global_position)
		await _touch(b, true)
		var head := _screen(me.view.to_global(me.view.head_center()))
		var from := head + Vector2(260, 120)
		await _drag_path(b, from, 6)
		await _drag_path(from, head, 4)
		await _touch(head, false)
		await _wait(0.25)
		if throw == 1:
			await _shot("18_ball")
	await _wait(1.0)
	await _shot("19_after_play")
	print("catch skill after play: ", Save.friend.catch_skill)
	me.hold = false
	await _room_steps(play)
	await _camp_steps()
	await _physics_steps()
	_finish()


func _finish() -> void:
	if failures.is_empty():
		print("FLOW OK")
	else:
		print("FLOW FAILED:")
		for f in failures:
			print("  - ", f)
	get_tree().quit(0 if failures.is_empty() else 1)


# --- B1: a camp of friends -------------------------------------------

func _camp_steps() -> void:
	main.go_to("grooming")
	await _wait(0.6)
	var camp: CampData = Save.camp
	# The second friend of the first day is on the way once the first
	# is patched up; bring the moment forward instead of waiting.
	_check(camp.next_arrival > 0.0, "a second friend is on the way after the first is patched up")
	Save.advance_to(camp.next_arrival + 1.0)
	await _wait(2.0)
	_check(camp.friends.size() == 2, "the second friend arrives")
	_check(tent.bench.size() == 1, "the second friend sits further back")
	await _shot("20_second_friend")

	# Tap the friend at the back to bring them forward.
	var v: BenchFriend = tent.bench.values()[0]
	await _tap(_screen(v.to_global(Vector2.ZERO)))
	await _wait(0.6)
	_check(camp.current == 1, "tapping a sitting friend brings them forward")
	_check(not Save.friend.mud.is_empty(), "the second friend came in muddy")
	await _shot("21_second_forward")

	# Days pass: needs move on, slowly.
	var before := Save.friend.stuffing
	camp.advance(3.0 * 86400.0, camp.clock + 3.0 * 86400.0)
	_check(Save.friend.stuffing < before, "three days away: stuffing settles")

	# A third friend, then quit and reopen: everyone is still there.
	camp.friends.append(FriendData.test_friend(7))
	Save.save_now()
	Save.load_game()
	# (Three days away may also have brought a newcomer.)
	_check(Save.camp.friends.size() >= 3 and "Test friend 7" in Save.camp.friends.map(func(f): return f.name), "three friends survive a quit and reopen")
	_check(Save.camp.current == 1, "the friend being looked after is remembered")
	main.go_to("grooming")
	tent.build_bench()
	tent.friend.setup(Save.friend)
	await _wait(0.6)
	_check(tent.bench.size() == mini(Save.camp.friends.size() - 1, tent.BENCH_X.size()), "the other friends sit further back")
	await _shot("22_three_friends")
	await _change_steps()


# --- B2: change over time ---------------------------------------------

func _alone(f: FriendData, days: float) -> CampData:
	var c := CampData.new()
	c.friends.append(f)
	c.clock = 1000000.0
	c.advance(days * 86400.0, c.clock + days * 86400.0)
	return c


func _change_steps() -> void:
	var healthy := FriendData.test_friend(5)
	_alone(healthy, 60.0)
	_check(not healthy.dead and healthy.brain == 1.0, "a healthy friend left alone for 60 days lives")
	var wounded := FriendData.first_friend()
	_alone(wounded, 30.0)
	_check(wounded.dead, "a friend left alone with a rotting, torn leg dies within a month")

	# Two weeks and a half away from a camp where the first friend was
	# never helped: the camp shows it.
	var camp := CampData.new()
	camp.friends.append(FriendData.first_friend())
	camp.friends.append(FriendData.second_friend())
	# A fixed start, so the same things happen every run; then the camp's
	# clock carries on from the game's own now.
	camp.clock = 2000000.0
	camp.advance(17.0 * 86400.0, camp.clock + 17.0 * 86400.0)
	camp.clock = Save.now()
	_check(camp.friends[0].dead, "the friend never helped has died")
	_check(not camp.friends[1].dead, "the muddy friend is still alive")
	_check(camp.bugs > 0 and not camp.tracks.is_empty(), "rot drew bugs and mud was tracked in")
	var moldy := false
	for slot in FriendData.SLOTS:
		var part: PartData = camp.friends[1].parts.get(slot)
		if part and not part.mold.is_empty():
			moldy = true
	_check(moldy, "mold spread to the other friend")
	camp.current = 1
	Save.camp = camp
	tent.traces.setup(camp)
	tent.friend.setup(Save.friend)
	tent.build_bench()
	await _wait(1.0)
	await _shot("23_after_weeks_away")

	# Chase a bug off; scrub a footprint.
	var bugs := camp.bugs
	var bug: Sprite2D = tent.traces._bugs[0]
	await _tap(_screen(bug.global_position))
	_check(camp.bugs == bugs - 1, "a tapped bug scurries off")
	var t: Dictionary = camp.tracks[0]
	var at := _screen(Vector2(float(t["x"]), float(t["y"])))
	var tracks := camp.tracks.size()
	var tc: Vector2 = tent.hud.tool_center(0)
	await _touch(tc, true)
	for i in 40:
		await _drag_to(at + Vector2(sin(i * 1.3) * 20.0, cos(i * 0.9) * 12.0) + Vector2(0, 80))
	await _touch(_last, false)
	_check(camp.tracks.size() < tracks, "the sponge scrubs footprints off the floor")

	# The friend who died, brought forward: still, grey, no answer.
	tent.choose_friend(0)
	await _wait(0.8)
	await _tap(_screen(tent.friend.to_global(Vector2(40, 10))))
	_check(Save.friend.dead and not Save.friend.asleep, "a friend who died stays still")
	await _shot("24_a_friend_died")
	await _supply_steps()


# --- B3: supplies run out ---------------------------------------------

func _supply_steps() -> void:
	var camp := Save.camp
	tent.choose_friend(1)
	await _wait(0.6)
	var c: FriendView = tent.friend
	# Spray the mold off one limb.
	var slot := ""
	for s in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		var part: PartData = Save.friend.part(s)
		if part and not part.mold.is_empty():
			slot = s
			break
	var spray: int = camp.supplies["spray"]
	var tc: Vector2 = tent.hud.tool_center(4)
	await _touch(tc, true)
	var mid := c.socket(slot).lerp(c.tip(slot), 0.5)
	for i in 8:
		await _drag_to(_screen(c.to_global(mid + Vector2(sin(i) * 8.0, 0))) + Vector2(0, 80))
	await _touch(_last, false)
	_check(Save.friend.part(slot).mold.is_empty(), "mold spray clears the mold off a limb")
	_check(camp.supplies["spray"] == spray - 1, "one squirt of spray is used")

	# Nothing left: the shelf shows it, and the empty spray won't come off it.
	for k in camp.supplies:
		camp.supplies[k] = 0
	await _wait(0.4)
	await _touch(tc, true)
	await _touch(tc, false)
	_check(tent.hud.held_tool == "" and tent.mode == "", "an empty spray stays on the shelf")
	# No thread: a dangling eye can't be sewn back on (decision 0042).
	main.go_to("table")
	Save.friend.part("eye_r").attached = false
	await _wait(1.0)
	var eye_at := _screen(c.to_global(c.eye_position()))
	await _touch(eye_at, true)
	var anchor := _screen(c.to_global(c.eye_anchor("eye_r")))
	await _drag_path(eye_at, anchor, 10)
	await _touch(anchor, false)
	await _wait(0.4)
	_check(tent.mode != "surgery", "with no thread, sewing can't start")
	await _shot("25_empty_shelf")
	await _parts_steps()


# --- B4: parts and heads ----------------------------------------------

## Grab at `at` (screen) and pull toward `dir` hard enough, holding,
## then let go: one hard pull.
func _hard_pull(at: Vector2, dir: Vector2) -> void:
	await _touch(at, true)
	for i in 10:
		await _drag_to(at + dir * (30.0 * (i + 1)))
	for i in 25:
		await _drag_to(at + dir * 300.0 + Vector2(sin(i) * 2.0, 0))
		if tent.mode == "part" or tent.mode == "head_held":
			return
	await _touch(_last, false)
	await _wait(0.2)


## The four steps of sewing a part on (fluff, tube, soak, sew).
func _do_surgery(c: FriendView, slot: String) -> void:
	for t in c.wounds.get(slot, []).duplicate():
		await _tap(_screen(c.to_global(c.socket(slot) + t)))
	await _wait(0.2)
	var end: Vector2 = c.surgery.get("tube_end", Vector2.ZERO)
	var port: Vector2 = c.socket(slot) + Vector2(10, 0)
	await _touch(_screen(c.to_global(end)), true)
	await _drag_path(_screen(c.to_global(end)), _screen(c.to_global(port)), 10)
	await _touch(_screen(c.to_global(port)), false)
	await _wait(0.3)
	var spill_s := _screen(c.to_global(c.surgery.get("spill_pos", Vector2.ZERO)))
	await _touch(spill_s, true)
	for i in 60:
		await _drag_to(spill_s + Vector2(sin(i * 0.7) * 40.0, cos(i * 0.5) * 14.0))
		if c.surgery.get("step", "") != "soak":
			break
	await _touch(_last, false)
	await _sew_points(c)


func _parts_steps() -> void:
	var camp := CampData.new()
	var a := FriendData.test_friend(11)
	var b := FriendData.test_friend(12)
	b.parts["arm_r"] = null
	camp.friends.append(a)
	camp.friends.append(b)
	camp.clock = Save.now()
	Save.camp = camp
	tent.traces.setup(camp)
	tent.friend.setup(Save.friend)
	tent.build_bench()
	main.go_to("table")
	tent.rooms.zoom_reset()
	await _wait(0.8)
	var c: FriendView = tent.friend

	# A healthy arm holds for a few hard pulls, then comes off (0025).
	var pulls := 0
	while tent.mode != "part" and pulls < 6:
		var mid := _screen(c.to_global(c.socket("arm_l").lerp(c.tip("arm_l"), 0.6)))
		await _hard_pull(mid, Vector2(-0.7, 0.7))
		pulls += 1
	_check(tent.mode == "part" and pulls == 3, "a healthy arm comes off on the third hard pull")
	await _shot("26_healthy_arm_off")
	# Into the pocket, to give to the other friend.
	var pocket: Vector2 = tent.hud.pocket_center()
	await _drag_path(_last, pocket, 12)
	await _touch(pocket, false)
	await _wait(0.3)
	_check(camp.spares.size() == 1, "the arm is kept in the pocket")

	# The other friend, missing an arm, gets it.
	tent.choose_friend(1)
	await _wait(0.6)
	c = tent.friend
	await _touch(pocket, true)
	var wound := _screen(c.to_global(c.socket("arm_r") + Vector2(0, 30)))
	await _drag_path(pocket, wound, 14)
	await _touch(wound, false)
	await _wait(0.7)
	_check(tent.mode == "surgery", "the kept arm starts surgery on the other friend")
	await _do_surgery(c, "arm_r")
	_check(b.part("arm_r") != null and b.part("arm_r").stitched, "the arm is sewn onto the other friend")
	await _shot("27_arm_given")

	# Head swap (0026): the first friend's head onto the second's body.
	tent.choose_friend(0)
	await _wait(0.6)
	c = tent.friend
	var tries := 0
	while tent.mode != "head_held" and tries < 6:
		await _hard_pull(_screen(c.to_global(c.head_center() + Vector2(0, -30))), Vector2(0, -1))
		tries += 1
	_check(tent.mode == "head_held", "three hard pulls take the head off")
	await _shot("28_head_off")
	await _drag_path(_last, pocket, 12)
	await _touch(pocket, false)
	await _wait(0.4)
	_check(camp.heads.size() == 1 and camp.friends[0].headless, "the head waits in the pocket; a body without a head is left")
	tent.choose_friend(1)
	await _wait(0.6)
	c = tent.friend
	tries = 0
	while tent.mode != "head_held" and tries < 6:
		await _hard_pull(_screen(c.to_global(c.head_center() + Vector2(0, -30))), Vector2(0, -1))
		tries += 1
	await _drag_path(_last, pocket, 12)
	await _touch(pocket, false)
	await _wait(0.4)
	_check(camp.heads.size() == 2, "both heads wait in the pocket")
	# The first head from the pocket onto the second body.
	await _touch(pocket, true)
	var neck := _screen(c.to_global(c.socket("neck")))
	await _drag_path(pocket, neck, 12)
	await _touch(neck, false)
	await _wait(0.8)
	_check(tent.mode == "surgery" and Save.friend == a, "the first friend's head goes onto the second body")
	await _shot("29_head_surgery")
	await _sew_points(tent.friend)
	await _wait(0.6)
	# The game carries on with the other head and the other body.
	_check(tent.mode == "surgery" and Save.friend == b, "the other head is taken to the other body")
	await _sew_points(tent.friend)
	await _wait(0.6)
	_check(camp.headless_bodies().is_empty() and camp.heads.is_empty(), "two friends, each on the other's body")
	_check(a.body_fabric != a.head_color(), "the first friend now has the second's body")
	await _shot("30_swapped")
	await _hole_steps()
