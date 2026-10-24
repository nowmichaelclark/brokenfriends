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
## The first-five-minutes spare leg has been given (it comes once).
var spare_given := false
## The camp's clock: the moment, in seconds since 1970, it last moved on.
var clock := 0.0
## When the next friend turns up, or 0 if nobody is on the way.
## (The arrivals system is step B6; until then only the second friend
## of the first day is scripted.)
var next_arrival := 0.0
var arrivals := 0
## Muddy footprints on the tent floor: [{x, y, a, r}] in tent units.
var tracks: Array = []
## Bugs crawling around the tent, drawn to rot.
var bugs := 0

const MAX_TRACKS := 12
const MAX_BUGS := 8

var _rng := RandomNumberGenerator.new()
## Friends who arrived during the last `advance` (indexes), for the game
## to announce. Data never talks to the screen itself.
var just_arrived: Array[int] = []


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
		var steps := int(ceil(seconds / 3600.0))
		var hours := seconds / 3600.0 / steps
		for i in steps:
			_step(hours)
	clock = now
	if next_arrival > 0.0 and now >= next_arrival:
		next_arrival = 0.0
		arrive(FriendData.second_friend())


func _step(hours: float) -> void:
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
	# Mold left on a rotting part spreads to other friends (0028).
	for f in friends:
		if f.dead or not f.spreads_mold():
			continue
		for other in friends:
			if other != f and not other.dead and _rng.randf() < 0.004 * hours:
				other.catch_mold(_rng)
	# Rot draws bugs; muddy friends track mud across the floor (0029).
	if rotting and bugs < MAX_BUGS and _rng.randf() < 0.05 * hours:
		bugs += 1
	if muddy and tracks.size() < MAX_TRACKS and _rng.randf() < 0.03 * hours:
		tracks.append({"x": _rng.randf_range(60, 660), "y": _rng.randf_range(900, 1060),
			"a": _rng.randf_range(-0.6, 0.6), "r": 1.0})


## The friends still alive.
func living() -> Array[FriendData]:
	var out: Array[FriendData] = []
	for f in friends:
		if not f.dead:
			out.append(f)
	return out


func arrive(f: FriendData) -> void:
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
		"tracks": tracks, "bugs": bugs,
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
	return c


## A new game: one friend in a bad way, alone (Mikey's first five minutes).
static func first_camp(now: float) -> CampData:
	var c := CampData.new()
	c.friends.append(FriendData.first_friend())
	c.clock = now
	return c
