extends "res://tests/flow_helpers.gd"
## The playthrough beyond the tent's first five minutes: quests,
## arrivals, play and music (steps B5 to B11), and the physics test
## (prototype 4).


# --- Holes: sewn shut, or left open to rot (Mikey's playtest notes) -----

func _hole_steps() -> void:
	var left := FriendData.test_friend(5)
	left.parts["arm_l"] = null
	var shut := FriendData.test_friend(5)
	shut.parts["arm_l"] = null
	shut.close_hole("arm_l")
	for f in [left, shut]:
		var camp := CampData.new()
		camp.friends.append(f)
		camp.clock = 1000000.0
		camp.advance(3.0 * 86400.0, camp.clock + 3.0 * 86400.0)
	print("three days with a hole: open rots to %.2f, stuffing %.2f; sewn shut, stuffing %.2f" % [left.hole_rot.get("arm_l", 0.0), left.stuffing, shut.stuffing])
	_check(float(left.hole_rot.get("arm_l", 0.0)) > 0.5 and shut.hole_rot.is_empty(), "a hole left open rots; one sewn shut doesn't")
	_check(shut.stuffing > left.stuffing + 0.2 and shut.decay() < left.decay(), "a hole sewn shut stops the leak")

	# On the table: drag thread to the hole, clear the fluff, sew across.
	var camp := Save.camp
	camp.supplies["thread"] = 2
	var f := FriendData.test_friend(19)
	f.parts["arm_l"] = null
	f.hole_rot["arm_l"] = 0.7
	camp.friends.append(f)
	main.go_to("table")
	tent.choose_friend(camp.friends.size() - 1)
	await _wait(0.8)
	var c: FriendView = tent.friend
	_check(tent.hud.glow == "thread", "an open hole glows the thread")
	await _shot("30b_rotting_hole")
	var spool: Vector2 = tent.hud.tool_center(0)
	var hole := _screen(c.to_global(c.socket("arm_l"))) + Vector2(0, 80)
	await _touch(spool, true)
	await _drag_path(spool, hole, 12)
	await _touch(hole, false)
	await _wait(0.7)
	_check(tent.mode == "surgery" and c.surgery.get("shut", false) and camp.supplies["thread"] == 1, "thread dropped on the hole starts sewing it shut")
	for t in c.wounds.get("arm_l", []).duplicate():
		await _tap(_screen(c.to_global(c.socket("arm_l") + t)))
	await _sew_points(c)
	await _wait(0.6)
	_check(f.is_missing("arm_l") and not f.is_open("arm_l") and not c.wounds.has("arm_l"), "the hole is sewn shut; the friend lives on without the arm")
	await _shot("30c_sewn_shut")
	await _quest_steps()


# --- The play room (Mikey's playtest notes, Q-036) ------------------------

func _room_steps(play: PlayRoom) -> void:
	# A crowd of made-up friends, and the camp's own.
	PlayRoom.crowd = 10
	main.go_to("play_room")
	await _wait(0.5)
	var living := Save.camp.friends.filter(func(f): return not f.dead and not f.headless).size()
	_check(play.wanderers.size() == living + 10, "everyone at camp and the crowd are in the play room")
	var before: Array = play.wanderers.map(func(w): return w.view.position)
	await _wait(4.0)
	var moved := 0
	for i in play.wanderers.size():
		if play.wanderers[i].view.position.distance_to(before[i]) > 20.0:
			moved += 1
	_check(moved >= 3, "friends wander about the room (%d moved)" % moved)
	# Pinch out to see the whole room.
	var mid := play.get_viewport_rect().size * 0.5
	await _finger(0, mid + Vector2(-200, 0), true)
	await _finger(1, mid + Vector2(200, 0), true)
	for i in 8:
		await _finger_drag(1, mid + Vector2(200 - i * 22, 0))
	await _finger(1, _last, false)
	await _finger(0, mid + Vector2(-200, 0), false)
	var wide := play.camera.zoom.x
	_check(wide < 0.8, "pinching zooms the camera out (%.2f)" % wide)
	play.zoom_at(PlayRoom.ZOOM_MIN, mid)
	await _wait(0.4)
	var seen := play.get_viewport_rect().size.x / play.camera.zoom.x
	_check(seen >= PlayRoom.ROOM_W * 0.95, "zoomed out, the whole room shows")
	await _shot("19b_play_room_wide")
	# A finger on the floor looks around.
	play.zoom_at(1.0, mid)
	play.camera.position.x = PlayRoom.ROOM_W * 0.5
	var cam: Vector2 = play.camera.position
	var floor := Vector2(mid.x, play.get_viewport_rect().size.y - 60.0)
	await _touch(floor, true)
	await _drag_path(floor, floor + Vector2(-300, 0), 6)
	await _touch(_last, false)
	_check(play.camera.position.x > cam.x + 100.0, "dragging the floor looks around the room")
	await _shot("19c_play_room_close")
	# The stress test: frame time as the crowd grows (S-012). On this
	# machine's software drawing, only for comparing.
	for n in [0, 10, 20, 40]:
		PlayRoom.crowd = n
		main.go_to("play_room")
		play.zoom_at(PlayRoom.ZOOM_MIN, mid)
		await _wait(1.0)
		var frames := 0
		var t0 := Time.get_ticks_usec()
		while Time.get_ticks_usec() - t0 < 3000000:
			await get_tree().process_frame
			frames += 1
		print("PERF play room, camp + %d friends: %.1f ms a frame, %d draw calls" % [n,
			3000.0 / frames, Performance.get_monitor(Performance.RENDER_TOTAL_DRAW_CALLS_IN_FRAME)])
		if n == 40:
			await _shot("19d_play_room_crowd")
	PlayRoom.crowd = 0
	main.go_to("entrance")
	await _wait(0.4)
	# The dev menu (0045): every test friend can be made, and added.
	var e: Entrance = main.entrance
	await _tap(e.hud.dev_rect().get_center())
	await _wait(0.3)
	_check(e.dev.visible and e.dev.size.y >= e.get_viewport_rect().size.y, "the gear opens the dev menu, over the whole screen")
	await _shot("19e_dev_menu")
	var plans := {}
	for kind in DevMenu.test_friends():
		var f := DevMenu.make_friend(kind)
		plans[f.body_look.get("plan", "")] = true
	_check(plans.size() == Looks.PLANS.size(), "the dev menu makes a test friend of every body plan")
	var count := Save.camp.friends.size()
	e.dev.add_friend("no legs")
	_check(Save.camp.friends.size() == count + 1 and Save.friend.is_missing("leg_l") and not Save.friend.is_open("leg_l"),
		"a test friend joins the camp from the dev menu")
	Save.camp.remove_friend(Save.camp.friends.size() - 1)
	Save.camp.current = 0
	e.build()


## A touch for one finger (`index`), for pinching.
func _finger(index: int, p: Vector2, pressed: bool) -> void:
	var ev := InputEventScreenTouch.new()
	ev.index = index
	ev.position = p
	ev.pressed = pressed
	get_viewport().push_input(ev, true)
	_last = p
	await get_tree().process_frame


func _finger_drag(index: int, p: Vector2) -> void:
	var ev := InputEventScreenDrag.new()
	ev.index = index
	ev.position = p
	ev.relative = p - _last
	get_viewport().push_input(ev, true)
	_last = p
	await get_tree().process_frame


# --- B5: quests ---------------------------------------------------------

## Move the game's clock on, as if time had passed.
func _jump(seconds: float) -> void:
	Save._offset += seconds
	Save.advance_to(Save.now())
	await _wait(0.4)


func _twenty(make: Callable, place: String, route: int) -> Dictionary:
	var counts := {"well": 0, "hurt": 0, "died": 0}
	for i in 20:
		var camp := CampData.new()
		camp.friends.append(make.call())
		camp.friends.append(FriendData.test_friend(99))
		camp.clock = 5000000.0 + i * 977.0
		camp.send(0, place, route, camp.clock)
		camp.advance(6 * 3600.0, camp.clock + 6 * 3600.0)
		counts[camp.just_returned[0].result] += 1
	return counts


func _quest_steps() -> void:
	var strong := _twenty(func(): return FriendData.test_friend(3), "ruins", 1)
	var weak := _twenty(func():
		var f := FriendData.test_friend(3)
		f.parts["leg_r"] = null
		f.stuffing = 0.45
		return f, "ruins", 1)
	print("20 quests over the ridge: strong ", strong, ", weak ", weak)
	_check(strong["well"] > weak["well"] and weak["hurt"] + weak["died"] > strong["hurt"] + strong["died"],
		"20 quests: a weak friend fares worse than a strong one on the same route")
	var safe := _twenty(func(): return FriendData.test_friend(3), "ruins", 0)
	_check(safe["died"] == 0 and strong["died"] > 0, "the safe route never kills; the ridge sometimes does")

	var camp := CampData.new()
	camp.friends.append(FriendData.test_friend(21))
	camp.friends.append(FriendData.test_friend(40))
	camp.clock = Save.now()
	Save.camp = camp
	tent.traces.setup(camp)
	tent.friend.setup(Save.friend)
	tent.build_bench()
	tent.parts.show_spare()
	main.go_to("entrance")
	await _wait(0.5)
	await _tap(main.entrance.hud.go_center(3))
	await _wait(0.6)
	var map: Node2D = main.map
	_check(map.visible, "the map opens from the entrance")
	await _tap(_screen(Quests.PLACES["woods"]["at"]))
	var thorny := Quests.route_points("woods", 1)
	await _tap(_screen(Quests.along(thorny, 0.3)))
	_check(map.place == "woods" and map.route == 1, "tapping a place and a route chooses them")
	await _shot("31_map_route")
	await _tap(map.hud.go_center())
	_check(camp.quests.size() == 1 and camp.friends.size() == 1, "the footprints send the friend off")
	await _jump(10.0 * 60.0)
	await _shot("32_out_on_a_quest")
	var supplies_before: int = 0
	for k in camp.supplies:
		supplies_before += int(camp.supplies[k])
	await _jump(15.0 * 60.0)
	_check(camp.quests.is_empty() and camp.friends.size() >= 2, "the friend comes home when the time is up")
	main.go_to("grooming")
	tent.choose_friend(1)
	await _wait(0.8)
	await _shot("33_home_from_the_woods")
	await _arrival_steps()


# --- B6: arrivals -------------------------------------------------------

func _arrival_steps() -> void:
	var gifts := 0
	var most := 0
	var arrivals := 0
	var newcomers_died := 0
	for m in 6:
		var camp := CampData.first_camp(3000000.0 + m * 1000.0)
		camp.arrivals = 1
		camp.advance(30.0 * 86400.0, camp.clock + 30.0 * 86400.0)
		gifts += camp.gifts
		arrivals += camp.arrivals - 1
		most = maxi(most, camp.friends.size())
		for i in range(1, camp.friends.size()):
			if camp.friends[i].dead:
				newcomers_died += 1
	print("six months away, one camp each: ", arrivals, " newcomers, ", gifts, " gifts, at most ", most, " friends")
	_check(arrivals >= 12, "a month brings newcomers")
	_check(most <= CampData.LIMIT, "the camp never holds more than its limit")
	_check(gifts > 0, "now and then a newcomer brings something")
	_check(newcomers_died == 0, "a newcomer nobody has seen yet waits, no worse")
	# One turns up while the tent is open.
	var c := Save.camp
	while c.friends.size() >= CampData.LIMIT:
		c.remove_friend(c.friends.size() - 1)
	var before := c.friends.size()
	c.arrive(FriendData.newcomer(77))
	Save.advance_to(Save.now())
	await _wait(2.2)
	_check(c.friends.size() == before + 1 and c.friends.back().met, "a newcomer walks into the tent and is seen")
	await _shot("34_a_newcomer")
	await _play_steps()


# --- B9: play, second cut ---------------------------------------------

func _play_steps() -> void:
	var skill := 0.05
	var throws := 0
	while skill < 0.5:
		skill = FriendData.learn(skill)
		throws += 1
	print("throws to catch half the time: ", throws)
	_check(throws > 20, "catching takes many throws to learn")
	var rng := RandomNumberGenerator.new()
	rng.seed = 5
	var tally := {"catch": 0, "fumble": 0, "miss": 0}
	for i in 300:
		tally[FriendData.try_skill(0.95, rng)] += 1
	_check(tally["fumble"] > 0 and tally["miss"] > 0, "even a skilled friend fumbles and misses sometimes")

	# The ball at a foot: a kick (or a try at one), learned the same way.
	var c := Save.camp
	# A friend with both legs, awake, standing still.
	var kicker := FriendData.test_friend(55)
	c.friends.append(kicker)
	c.current = c.friends.size() - 1
	main.go_to("play_room")
	var play: PlayRoom = main.play
	var me := play.wanderer_of(Save.friend)
	me.hold = true
	play.camera.position = me.view.position
	await _wait(0.6)
	var kick := Save.friend.kick_skill
	var ball: RigidBody2D = play.ball
	# A ball sent at a moving foot can miss it, so up to six tries.
	for t in 6:
		if Save.friend.kick_skill > kick:
			break
		me.away = false
		me.cooldown = 0.0
		await _wait(0.8)
		var foot: Vector2 = me.view.to_global(me.view.tip("leg_r"))
		ball.freeze = true
		ball.global_position = foot + Vector2(160, -40)
		ball.freeze = false
		ball.linear_velocity = (foot - ball.global_position).normalized() * 700.0
		await _wait(0.5)
	_check(Save.friend.kick_skill > kick, "a ball at the foot gets kicked back (and kicking is learned)")
	await _shot("35_kick")
	me.hold = false
	main.go_to("grooming")
	await _music_steps()


# --- B11: music at moments ------------------------------------------------

func _music_steps() -> void:
	await _wait(1.2)
	var reasons: Array = Music.changes.map(func(ch): return ch[2])
	print("music changes: ", Music.changes.size())
	for ch in Music.changes:
		print("  ", ch[0], "s  ", ch[1], "  (", ch[2], ")")
	_check(reasons.has("the play room") and reasons.has("the map"), "music changes going to the play room and the map")
	_check(reasons.has("the tent") or reasons.has("night at camp"), "the tent has its own music")
	_check(reasons.has("a friend made well") or reasons.has("night at camp"), "a friend made well changes the music (by day)")
	_check(reasons.any(func(r): return r.begins_with("a friend came home")), "a friend home from a quest changes the music")
	_check(Music.changes.all(func(ch): return ch[2] != ""), "every change has a reason")
	# Nothing happens for half a minute: the music stays put.
	var before := Music.changes.size()
	await _wait(30.0)
	_check(Music.changes.size() == before, "no change on a timer")


# --- Prototype 4: friends as soft ragdolls --------------------------------

func _physics_steps() -> void:
	main.go_to("physics")
	var play: PhysicsPlay = main.physics
	await _wait(0.3)
	# Every kind of test friend, dropped from a height, lands, settles
	# and stays in one piece.
	var whole := true
	var still := true
	var impact := 0.0
	for n in 6:
		play.spawn([n], [PhysicsPlay.DROP_AT.x])
		var r: Ragdoll = play.friends[0]
		await _settle(r, 6.0)
		whole = whole and r.intact()
		still = still and r.settled() and r.bodies["body"].global_position.y < PhysicsPlay.FLOOR_Y
		impact = maxf(impact, r.max_impact)
		if n == 2:
			await _shot("40_ragdoll_landed")
	print("hardest landing on a drop: %d px/s (a hard pull from %d)" % [impact, Ragdoll.HARD])
	_check(whole, "a friend dropped from a height stays in one piece")
	_check(still, "a dropped friend lands and settles")

	# A loose seam, dropped hard: that part comes off, and stays off.
	play.spawn([6], [PhysicsPlay.DROP_AT.x])
	var loose: Ragdoll = play.friends[0]
	loose.push(Vector2(0, 2600))
	await _settle(loose, 6.0)
	print("hardest landing on a hard drop: %d px/s" % loose.max_impact)
	_check(loose.data.part("arm_l") == null and loose.torn.has("arm_l"), "a loose seam dropped hard loses that part")
	_check(loose.data.part("arm_r") != null and loose.data.part("leg_l") != null, "the other seams hold")
	await _shot("41_torn")

	# The neck (V-020): the notch between chin and shoulders, where the
	# board once caught, is filled; a very hard hit pulls the neck seam,
	# and only after several does the head come off (0025).
	play.spawn([0], [PhysicsPlay.DROP_AT.x])
	var necked: Ragdoll = play.friends[0]
	await _settle(necked, 6.0)
	var body: RigidBody2D = necked.bodies["body"]
	var q := PhysicsPointQueryParameters2D.new()
	q.collision_mask = 2
	var filled := true
	for side in [-1.0, 1.0]:
		q.position = body.to_global(Vector2(80.0 * side, -118.0) * necked.k())
		filled = filled and not play.get_world_2d().direct_space_state.intersect_point(q, 4).is_empty()
	_check(filled, "nothing can wedge between the head and the shoulders")
	necked.push(Vector2(0, 3400))
	await _settle(necked, 6.0)
	print("hardest head landing on a hard drop: %d px/s, neck pulls %d" % [necked.max_head_impact, necked.data.neck_pulls])
	_check(not necked.torn.has("neck"), "one very hard landing doesn't take the head off")
	play.spawn([0], [PhysicsPlay.DROP_AT.x])
	necked = play.friends[0]
	necked.data.neck_pulls = Ragdoll.NECK_PULLS - 1
	await _wait(0.1)
	necked.push(Vector2(0, -3600))
	await _settle(necked, 6.0)
	print("head hitting the ceiling: %d px/s" % necked.max_head_impact)
	_check(necked.torn.has("neck"), "a neck already pulled several times gives on a very hard hit")
	_check(necked.data.headless and necked.head_data.head_only and not necked.bodies.has("head") and necked.loose.has("head"),
		"the friend lives on in the head; the body is left without one, and the head no longer counts as the friend moving")

	# A board thrown down onto a friend lying still: the part it hits
	# feels it, though the part wasn't moving (DeepSeek review, finding 7).
	play.spawn([0], [PhysicsPlay.DROP_AT.x])
	var target: Ragdoll = play.friends[0]
	await _settle(target, 6.0)
	target.max_impact = 0.0
	target.max_head_impact = 0.0
	var head_at: Vector2 = target.bodies["head"].global_position
	play.new_board(head_at + Vector2(0, -260))
	await get_tree().physics_frame
	play.board.linear_velocity = Vector2(0, 3200)
	for w in play.board.wheels:
		w.linear_velocity = Vector2(0, 3200)
	await _wait(1.0)
	print("a board thrown down at a still friend: %d px/s at the head, %d at a limb" % [target.max_head_impact, target.max_impact])
	_check(maxf(target.max_head_impact, target.max_impact) >= Ragdoll.HARD, "a board thrown hard at a still friend counts as a hard hit")
	await _shot("42_board_hit")
	play.reset_board()

	# Drag and fling with a finger.
	play.spawn([4], [PhysicsPlay.DROP_AT.x])
	var flung: Ragdoll = play.friends[0]
	await _settle(flung, 6.0)
	var at: Vector2 = flung.bodies["body"].global_position
	await _touch(_screen(at), true)
	_check(flung.is_grabbed(), "a finger picks a friend up")
	for i in 8:
		await _drag_to(_screen(at + Vector2(i * 12.0, -i * 60.0)))
	await _touch(_last, false)
	var vy: float = flung.bodies["body"].linear_velocity.y
	_check(vy < -600.0, "letting go mid-swipe flings them (%d px/s)" % int(vy))
	await _settle(flung, 6.0)
	# The same with a mouse, as in a browser on a laptop (DeepSeek
	# review, finding 10): the engine turns the mouse into a finger
	# (project setting input_devices/pointing/emulate_touch_from_mouse).
	at = flung.bodies["body"].global_position
	var mb := InputEventMouseButton.new()
	mb.button_index = MOUSE_BUTTON_LEFT
	mb.button_mask = MOUSE_BUTTON_MASK_LEFT
	mb.pressed = true
	mb.position = _screen(at)
	mb.global_position = mb.position
	Input.parse_input_event(mb)
	await get_tree().process_frame
	await get_tree().process_frame
	_check(flung.is_grabbed(), "a mouse button picks a friend up")
	for i in 8:
		var mm := InputEventMouseMotion.new()
		mm.button_mask = MOUSE_BUTTON_MASK_LEFT
		mm.position = _screen(at + Vector2(0, -(i + 1) * 60.0))
		mm.global_position = mm.position
		mm.relative = Vector2(0, -60.0)
		mm.velocity = Vector2(0, -3600.0)
		Input.parse_input_event(mm)
		await get_tree().process_frame
	mb = mb.duplicate()
	mb.pressed = false
	mb.button_mask = 0
	mb.position = _screen(at + Vector2(0, -480.0))
	mb.global_position = mb.position
	Input.parse_input_event(mb)
	await get_tree().process_frame
	await get_tree().process_frame
	_check(not flung.is_grabbed() and flung.bodies["body"].global_position.y < at.y - 60.0, "dragging with the mouse lifts them, and letting go lets them go")
	await _settle(flung, 6.0)

	# The board rolls down the ramp with a friend sitting on it.
	play.reset_board()
	play.spawn([3], [play.board.global_position.x + 10.0])
	await _wait(1.5)
	play.board.linear_velocity = Vector2(420, 0)
	play.friends[0].push(Vector2(420, 0))
	await _wait(0.5)
	await _shot("42_board")
	# Up to 8 s: on a busy machine physics can fall behind the clock.
	var down := func() -> bool: return play.board.global_position.y > PhysicsPlay.LEDGE.y + 200.0
	for i in 32:
		if down.call():
			break
		await _wait(0.25)
	_check(down.call(), "the skateboard rolls down the ramp")

	# Three friends at once, and nobody flies apart.
	play.spawn([0, 1, 2], [200.0, 380.0, 560.0])
	await _wait(5.0)
	var ok := true
	for r in play.friends:
		ok = ok and r.intact() and r.speed() < 400.0
	if not ok:
		for r in play.friends:
			print("  three at once: intact ", r.intact(), " speed ", int(r.speed()), " torn ", r.torn.keys(), " neck pulls ", r.head_data.neck_pulls, " head hit ", int(r.max_head_impact), " dead ", r.head_data.dead, " ", r.head_data.name)
	_check(ok, "three friends at once land and stay together")
	await _shot("43_three")
	main.go_to("entrance")


## Wait until a ragdoll stops moving (or `limit` seconds pass).
func _settle(r: Ragdoll, limit: float) -> void:
	var t := 0.0
	await _wait(0.8)
	while t < limit and not r.settled():
		await _wait(0.2)
		t += 0.2
