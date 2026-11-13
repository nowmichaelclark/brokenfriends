class_name QuestData
extends Resource
## One friend out on a quest (decision 0034): where, which way, when it
## left and when it's due back. What happens is rolled when the friend
## leaves and shown when it comes home, so leaving the game closed
## changes nothing about the result (pillar 3: the cause was the route
## and the friend's shape, both visible before sending).

var friend: FriendData
var place := ""
var route := 0
var leave_at := 0.0
var back_at := 0.0
## "well", "hurt" or "died".
var result := ""
## Dangers that hurt the friend (see Quests.DANGERS).
var harms: Array = []
## Things brought back: supply names, or "part".
var finds: Array = []
## A friend met out there and brought home (0034).
var brings_friend := false
## For finds and harms that need choices (which limb, which colours).
var seed_value := 0


## How far along the way out and back, 0 (leaving) .. 1 (home).
func progress(now: float) -> float:
	return clampf((now - leave_at) / maxf(1.0, back_at - leave_at), 0.0, 1.0)


## Roll what will happen, as the friend leaves.
static func start(f: FriendData, place_id: String, route_id: int, now: float, rng: RandomNumberGenerator) -> QuestData:
	var q := QuestData.new()
	q.friend = f
	q.place = place_id
	q.route = route_id
	q.leave_at = now
	var r: Dictionary = Quests.PLACES[place_id]["routes"][route_id]
	q.back_at = now + float(r["minutes"]) * 60.0
	q.seed_value = rng.randi()
	var c := Quests.chance(f, place_id, route_id)
	var roll := rng.randf()
	var dangers: Array = r["dangers"].map(func(d): return d[0])
	if roll < c:
		q.result = "well"
		q.finds = _roll_finds(place_id, 1 + (1 if rng.randf() < 0.5 else 0), rng)
		q.brings_friend = rng.randf() < float(Quests.PLACES[place_id]["friend_chance"])
		# Even a good trip can soak or muddy a friend on the way.
		for d in dangers:
			if d in ["water", "mud"] and rng.randf() < 0.5:
				q.harms.append(d)
	else:
		var severity := (roll - c) / maxf(0.001, 1.0 - c)
		var deadly := "cliff" in dangers or "wolf" in dangers
		if deadly and severity > 0.7:
			q.result = "died"
		else:
			q.result = "hurt"
			q.harms = dangers.duplicate()
			if q.harms.is_empty():
				q.harms = ["tired"]
			q.finds = _roll_finds(place_id, 1 if rng.randf() < 0.4 else 0, rng)
	return q


static func _roll_finds(place_id: String, n: int, rng: RandomNumberGenerator) -> Array:
	var table: Dictionary = Quests.PLACES[place_id]["finds"]
	var total := 0
	for k in table:
		total += int(table[k])
	var out: Array = []
	for i in n:
		var pick := rng.randi() % total
		for k in table:
			pick -= int(table[k])
			if pick < 0:
				if k != "nothing":
					out.append(k)
				break
	return out


## The friend comes home (or doesn't): change it, and fill the camp's
## shelf and pocket. Returns the friend met out there, if any.
func finish(camp: CampData) -> FriendData:
	var rng := RandomNumberGenerator.new()
	rng.seed = seed_value
	var f := friend
	var limbs: Array = []
	for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
		if f.part(slot) != null:
			limbs.append(slot)
	if result == "died":
		# Something scavenged the rest: one or two parts come back (0026).
		f.dead = true
		limbs.shuffle()
		for i in mini(limbs.size(), 1 + rng.randi() % 2):
			var part: PartData = f.part(limbs[i])
			part.pulls = 0
			camp.spares.append(part)
		return null
	for h in harms:
		limbs = limbs.filter(func(slot): return f.part(slot) != null)
		match h:
			"water":
				f.wetness = 1.0
			"mud":
				for i in 3:
					f.mud.append({"x": rng.randf_range(-80, 80), "y": rng.randf_range(-60, 110), "r": rng.randf_range(16, 28), "a": 1.0})
			"thorns":
				if not limbs.is_empty():
					var part: PartData = f.part(limbs[rng.randi() % limbs.size()])
					part.torn = true
					part.rot = minf(1.0, part.rot + 0.15)
			"wolf":
				if not limbs.is_empty():
					f.parts[limbs[rng.randi() % limbs.size()]] = null
			"cliff":
				if not limbs.is_empty():
					f.part(limbs[rng.randi() % limbs.size()]).torn = true
				f.stuffing = maxf(0.3, f.stuffing - 0.2)
			"tired":
				f.stuffing = maxf(0.3, f.stuffing - 0.1)
	if result == "well":
		f.stamina = minf(1.0, f.stamina + 0.06)
	f.stuffing = maxf(0.3, f.stuffing - 0.05)
	for k in finds:
		if k == "part":
			camp.spares.append(_found_part(rng))
		else:
			camp.supplies[k] = int(camp.supplies.get(k, 0)) + (2 if k == "stuffing" else 1)
	if brings_friend:
		return FriendData.found_friend(seed_value)
	return null


static func _found_part(rng: RandomNumberGenerator) -> PartData:
	var slot: String = ["arm_l", "leg_l"][rng.randi() % 2]
	var color := Color.from_hsv(rng.randf(), rng.randf_range(0.2, 0.55), rng.randf_range(0.45, 0.8))
	var p := PartData.make(slot, color, rng.randi_range(0, 3), rng.randf_range(0.3, 0.9))
	p.attached = false
	p.rot = rng.randf_range(0.0, 0.3)
	return p


func to_dict() -> Dictionary:
	return {
		"friend": friend.to_dict(), "place": place, "route": route,
		"leave_at": leave_at, "back_at": back_at, "result": result,
		"harms": harms, "finds": finds, "brings_friend": brings_friend,
		"seed": seed_value,
	}


static func from_dict(d: Dictionary) -> QuestData:
	var q := QuestData.new()
	q.friend = FriendData.from_dict(d.get("friend", {}))
	q.place = d.get("place", "meadow")
	q.route = int(d.get("route", 0))
	q.leave_at = float(d.get("leave_at", 0.0))
	q.back_at = float(d.get("back_at", 0.0))
	q.result = d.get("result", "well")
	q.harms = d.get("harms", [])
	q.finds = d.get("finds", [])
	q.brings_friend = d.get("brings_friend", false)
	q.seed_value = int(d.get("seed", 0))
	return q
