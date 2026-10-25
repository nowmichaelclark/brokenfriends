extends "res://tests/flow_helpers.gd"
## The playthrough beyond the tent's first five minutes: quests,
## arrivals, play and music (steps B5 to B11), and the physics test
## (prototype 4).


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
	await _wait(0.5)
	await _tap(tent.hud.map_rect().get_center())
	await _wait(0.6)
	var map: Node2D = main.map
	_check(map.visible, "the map opens from the tent")
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
	main.go_to("tent")
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
	main.go_to("play_corner")
	var play: Node2D = main.play
	play.friend.setup(Save.friend)
	await _wait(0.6)
	var kick := Save.friend.kick_skill
	var ball: RigidBody2D = play.ball
	# A ball sent at a moving foot can miss it, so up to six tries.
	for t in 6:
		if Save.friend.kick_skill > kick:
			break
		play._walk_timer = 99.0
		play._walk_target = play.friend.position.x
		play._away = false
		play._cooldown = 0.0
		await _wait(0.8)
		var foot: Vector2 = play.friend.to_global(play.friend.tip("leg_r"))
		ball.freeze = true
		ball.global_position = foot + Vector2(160, -40)
		ball.freeze = false
		ball.linear_velocity = (foot - ball.global_position).normalized() * 700.0
		await _wait(0.5)
	_check(Save.friend.kick_skill > kick, "a ball at the foot gets kicked back (and kicking is learned)")
	await _shot("35_kick")
	main.go_to("tent")
	await _music_steps()


# --- B11: music at moments ------------------------------------------------

func _music_steps() -> void:
	await _wait(1.2)
	var reasons: Array = Music.changes.map(func(ch): return ch[2])
	print("music changes: ", Music.changes.size())
	for ch in Music.changes:
		print("  ", ch[0], "s  ", ch[1], "  (", ch[2], ")")
	_check(reasons.has("the play corner") and reasons.has("the map"), "music changes going to the play corner and the map")
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
	var play := PhysicsPlay.new()
	main.add_child(play)
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

	# The board rolls down the ramp with a friend sitting on it.
	play.reset_board()
	play.spawn([3], [play.board.global_position.x + 10.0])
	await _wait(1.5)
	play.board.linear_velocity = Vector2(420, 0)
	play.friends[0].push(Vector2(420, 0))
	await _wait(0.5)
	await _shot("42_board")
	await _wait(2.5)
	_check(play.board.global_position.y > PhysicsPlay.LEDGE.y + 200.0, "the skateboard rolls down the ramp")

	# Three friends at once, and nobody flies apart.
	play.spawn([0, 1, 2], [200.0, 380.0, 560.0])
	await _wait(5.0)
	var ok := true
	for r in play.friends:
		ok = ok and r.intact() and r.speed() < 400.0
	_check(ok, "three friends at once land and stay together")
	await _shot("43_three")
	play.queue_free()
	main.go_to("tent")


## Wait until a ragdoll stops moving (or `limit` seconds pass).
func _settle(r: Ragdoll, limit: float) -> void:
	var t := 0.0
	await _wait(0.8)
	while t < limit and not r.settled():
		await _wait(0.2)
		t += 0.2
