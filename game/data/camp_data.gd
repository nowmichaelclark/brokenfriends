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


func friend() -> FriendData:
	if friends.is_empty():
		return null
	current = clampi(current, 0, friends.size() - 1)
	return friends[current]


## Move the camp on by `seconds` of time. Cheap enough to call often.
func advance(seconds: float, now: float) -> void:
	if seconds > 0.0:
		for f in friends:
			f.advance(seconds)
	clock = now
	if next_arrival > 0.0 and now >= next_arrival:
		next_arrival = 0.0
		arrive(FriendData.second_friend())


func arrive(f: FriendData) -> void:
	friends.append(f)
	arrivals += 1
	Events.friend_arrived.emit(friends.size() - 1)


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
	return c


## A new game: one friend in a bad way, alone (Mikey's first five minutes).
static func first_camp(now: float) -> CampData:
	var c := CampData.new()
	c.friends.append(FriendData.first_friend())
	c.clock = now
	return c
