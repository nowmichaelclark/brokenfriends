class_name CampData
extends Resource
## One camp, as data: the friends who live there, the spare parts in
## the pocket, and the camp's own clock. Time passes here while the
## player is away (decision 0011); `advance` moves the camp on.

## The friends at camp. The one on the tent floor is `friends[current]`.
var friends: Array[FriendData] = []
var current := 0
## Spare parts waiting in the pocket (scarce, decision 0027).
var spares: Array[PartData] = []
## Supplies on the shelf, counted (decisions 0027, 0042): handfuls of
## stuffing, doses of oil, lengths of thread (one per sewing), squirts of
## mold spray (one per moldy part cleaned). They run out; more comes
## from quests and newcomers (steps B5, B6).
var supplies := {"stuffing": 6, "oil": 3, "thread": 4, "spray": 2}
## The first-five-minutes spare leg has been given (it comes once).
var spare_given := false
## The camp's clock: the moment, in seconds since 1970, it last moved on.
var clock := 0.0
## When the next friend turns up, or 0 if nobody is on the way.
## (The arrivals system is step B6; until then only the second friend
## of the first day is scripted.)
var next_arrival := 0.0
var arrivals := 0
## Gifts newcomers have brought, for the tests.
var gifts := 0
## Friends out on quests (0034). A friend on a quest is not at camp.
var quests: Array[QuestData] = []
## Quests that ended during the last `advance`, for the game to show.
var just_returned: Array[QuestData] = []
## Heads kept in the pocket (0026), each a friend without a body. A head
## left a day without one dies.
var heads: Array[FriendData] = []
## Muddy footprints on the tent floor: [{x, y, a, r}] in tent units.
var tracks: Array = []
## Bugs crawling around the tent, drawn to rot.
var bugs := 0

## How many friends a camp holds, the one in front and four further
## back (decision 0013: camps have limits; the number is a guess).
const LIMIT := 5
## Newcomers turn up about once in this many hours, while there's room.
const ARRIVAL_HOURS := 36.0
## About one newcomer in this many brings something with them (0034).
const GIFT_ONE_IN := 7
## How long a head lasts without a body, seconds (0026: about a day).
const HEAD_LIFE := 86400.0
const MAX_TRACKS := 12
const MAX_BUGS := 8

var _rng := RandomNumberGenerator.new()
## Friends who arrived during the last `advance` (indexes), for the game
## to announce. Data never talks to the screen itself.
var just_arrived: Array[int] = []
## Friends who died at camp during the last `advance`, for the game to mark.
var just_died := 0


func friend() -> FriendData:
	if friends.is_empty():
		return null
	current = clampi(current, 0, friends.size() - 1)
	return friends[current]


## How dirty the camp is, 0..1: footprints and bugs. Dirt speeds rot;
## it never starts it (decision 0011).
func dirt() -> float:
	return clampf(tracks.size() / float(MAX_TRACKS) * 0.5 + bugs / float(MAX_BUGS) * 0.5, 0.0, 1.0)


## Move the camp on by `seconds` of time, an hour at most per step.
## Cheap enough to call every few seconds while playing, and for a
## month away on opening.
func advance(seconds: float, now: float) -> void:
	if seconds > 0.0:
		_rng.seed = hash(int(clock))
		_step_clock = clock
		var steps := int(ceil(seconds / 3600.0))
		var hours := seconds / 3600.0 / steps
		for i in steps:
			_step(hours)
			_returns(_step_clock)
	clock = now
	_returns(now)
	if next_arrival > 0.0 and now >= next_arrival:
		next_arrival = 0.0
		arrive(FriendData.second_friend())


var _step_clock := 0.0


func clock_now() -> float:
	return _step_clock


func _step(hours: float) -> void:
	_step_clock += hours * 3600.0
	var d := dirt()
	var rotting := false
	var muddy := false
	for f in friends:
		f.step(hours, d, _rng)
		if f.dead or f.spreads_mold():
			rotting = true
		for slot in FriendData.SLOTS:
			var part: PartData = f.parts.get(slot)
			if part and part.rot > 0.6:
				rotting = true
		if not f.dead and f.mud_amount() > 0.5:
			muddy = true
	for f in friends:
		if f.dead and not f.mourned:
			f.mourned = true
			just_died += 1
	# A head without a body lasts about a day (0026).
	for h in heads:
		if not h.dead and clock_now() - h.off_at > HEAD_LIFE:
			h.dead = true
	heads = heads.filter(func(h): return not h.dead)
	# A dead friend or a headless body with nothing left to give is laid
	# to rest.
	_lay_to_rest()
	# Mold left on a rotting part spreads to other friends (0028).
	for f in friends:
		if f.dead or not f.spreads_mold():
			continue
		for other in friends:
			if other != f and not other.dead and _rng.randf() < 0.004 * hours:
				other.catch_mold(_rng)
	# Now and then a friend turns up, looking for help or a home (0034).
	# The first day's second friend is scripted; after it, chance.
	if arrivals >= 1 and next_arrival == 0.0 and friends.size() < LIMIT \
			and _rng.randf() < hours / ARRIVAL_HOURS:
		var newcomer := FriendData.newcomer(_rng.randi())
		if _rng.randi() % GIFT_ONE_IN == 0:
			_gift()
		arrive(newcomer)
	# Rot draws bugs; muddy friends track mud across the floor (0029).
	if rotting and bugs < MAX_BUGS and _rng.randf() < 0.05 * hours:
		bugs += 1
	if muddy and tracks.size() < MAX_TRACKS and _rng.randf() < 0.03 * hours:
		tracks.append({"x": _rng.randf_range(60, 660), "y": _rng.randf_range(900, 1060),
			"a": _rng.randf_range(-0.6, 0.6), "r": 1.0})


## Send the friend at `index` out on a route (0034). False if it can't go.
func send(index: int, place: String, route: int, now: float) -> bool:
	if index < 0 or index >= friends.size():
		return false
	var f := friends[index]
	if f.dead or f.headless:
		return false
	var rng := RandomNumberGenerator.new()
	rng.seed = hash([int(now), index, place, route])
	f.asleep = false
	quests.append(QuestData.start(f, place, route, now, rng))
	remove_friend(index)
	return true


## Bring home everyone whose quest has ended by `t`.
func _returns(t: float) -> void:
	for q in quests.duplicate():
		if q.back_at > t:
			continue
		quests.erase(q)
		var found: FriendData = q.finish(self)
		if q.result != "died":
			friends.append(q.friend)
		if found:
			arrive(found)
		just_returned.append(q)


## A dead friend or headless body with no arms or legs left is laid to
## rest: there is nothing more to take from it.
func _lay_to_rest() -> void:
	for i in range(friends.size() - 1, -1, -1):
		var f := friends[i]
		if not (f.dead or f.headless) or friends.size() == 1:
			continue
		var left := 0
		for slot in ["arm_l", "arm_r", "leg_l", "leg_r"]:
			if f.parts.get(slot) != null:
				left += 1
		if left == 0:
			remove_friend(i)


func remove_friend(i: int) -> void:
	friends.remove_at(i)
	if current >= i and current > 0:
		current -= 1


## Headless bodies at camp (indexes).
func headless_bodies() -> Array[int]:
	var out: Array[int] = []
	for i in friends.size():
		if friends[i].headless:
			out.append(i)
	return out


## The friends still alive.
func has(supply: String) -> bool:
	return int(supplies.get(supply, 0)) > 0


## Use one of a supply. False if there is none left.
func use(supply: String) -> bool:
	if not has(supply):
		return false
	supplies[supply] = int(supplies[supply]) - 1
	return true


func living() -> Array[FriendData]:
	var out: Array[FriendData] = []
	for f in friends:
		if not f.dead:
			out.append(f)
	return out


## Something a newcomer brought: a few supplies or a spare part.
func _gift() -> void:
	if _rng.randf() < 0.3:
		spares.append(QuestData._found_part(_rng))
	else:
		var k: String = ["stuffing", "oil", "thread", "spray"][_rng.randi() % 4]
		supplies[k] = int(supplies[k]) + 2
	gifts += 1


## A friend comes to camp. A full camp turns them away (they find
## somewhere else).
func arrive(f: FriendData) -> void:
	if friends.size() >= LIMIT:
		return
	f.met = false
	friends.append(f)
	arrivals += 1
	just_arrived.append(friends.size() - 1)


func to_dict() -> Dictionary:
	var fl: Array = []
	for f in friends:
		fl.append(f.to_dict())
	var sp: Array = []
	for p in spares:
		sp.append(p.to_dict())
	return {
		"friends": fl, "current": current, "spares": sp,
		"spare_given": spare_given, "clock": clock,
		"next_arrival": next_arrival, "arrivals": arrivals,
		"tracks": tracks, "bugs": bugs, "supplies": supplies,
		"heads": heads.map(func(h): return h.to_dict()),
		"quests": quests.map(func(q): return q.to_dict()),
	}


static func from_dict(d: Dictionary) -> CampData:
	var c := CampData.new()
	for fd in d.get("friends", []):
		if fd is Dictionary:
			c.friends.append(FriendData.from_dict(fd))
	c.current = int(d.get("current", 0))
	for pd in d.get("spares", []):
		if pd is Dictionary:
			c.spares.append(PartData.from_dict(pd))
	c.spare_given = d.get("spare_given", false)
	c.clock = float(d.get("clock", 0.0))
	c.next_arrival = float(d.get("next_arrival", 0.0))
	c.arrivals = int(d.get("arrivals", 0))
	c.tracks = d.get("tracks", [])
	c.bugs = int(d.get("bugs", 0))
	for qd in d.get("quests", []):
		if qd is Dictionary:
			c.quests.append(QuestData.from_dict(qd))
	for hd in d.get("heads", []):
		if hd is Dictionary:
			c.heads.append(FriendData.from_dict(hd))
	var sup: Dictionary = d.get("supplies", {})
	for k in c.supplies:
		c.supplies[k] = int(sup.get(k, c.supplies[k]))
	return c


## A new game: one friend in a bad way, alone (Mikey's first five minutes).
static func first_camp(now: float) -> CampData:
	var c := CampData.new()
	c.friends.append(FriendData.first_friend())
	c.clock = now
	return c
