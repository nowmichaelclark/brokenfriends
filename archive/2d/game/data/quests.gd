class_name Quests
extends RefCounted
## The places a friend can be sent from the first camp, and the routes
## there (decision 0034; the first playable uses preset routes, Gate A).
## Plain data: the map draws it, `QuestData` plays it out.
##
## Every number here is a guess for Mikey to tune (game/README,
## "Assumed in this build").

## Dangers a route can cross, and what each does to a friend who fails
## there: "water" soaks, "mud" muddies, "thorns" tear a part open,
## "wolf" takes a limb, "cliff" can kill.
const DANGERS := ["water", "mud", "thorns", "wolf", "cliff"]

## id -> place. `at` is where it sits on the map (720 x 1280 units).
## Each route: `via` (map points between camp and the place), `minutes`
## (one way and back), `chance` (for a friend in good shape), `dangers`
## (each with where along the route it sits, 0..1).
## `finds`: what can be brought back, with weights.
const PLACES := {
	"meadow": {
		"at": Vector2(190, 790),
		"finds": {"stuffing": 5, "thread": 2, "nothing": 3},
		"friend_chance": 0.0,
		"routes": [
			{"via": [Vector2(300, 940)], "minutes": 5.0, "chance": 0.98, "dangers": []},
			{"via": [Vector2(250, 900), Vector2(170, 860)], "minutes": 3.0, "chance": 0.92, "dangers": [["mud", 0.5]]},
		],
	},
	"woods": {
		"at": Vector2(560, 610),
		"finds": {"thread": 4, "spray": 3, "stuffing": 2, "part": 1, "nothing": 2},
		"friend_chance": 0.12,
		"routes": [
			{"via": [Vector2(620, 900), Vector2(660, 740)], "minutes": 60.0, "chance": 0.92, "dangers": [["mud", 0.6]]},
			{"via": [Vector2(470, 820)], "minutes": 20.0, "chance": 0.7, "dangers": [["thorns", 0.55]]},
		],
	},
	"river": {
		"at": Vector2(210, 430),
		"finds": {"oil": 4, "spray": 2, "part": 2, "nothing": 1},
		"friend_chance": 0.1,
		"routes": [
			{"via": [Vector2(380, 700), Vector2(120, 600)], "minutes": 120.0, "chance": 0.88, "dangers": [["mud", 0.4]]},
			{"via": [Vector2(300, 640)], "minutes": 45.0, "chance": 0.62, "dangers": [["water", 0.6]]},
		],
	},
	"ruins": {
		"at": Vector2(480, 230),
		"finds": {"part": 4, "oil": 2, "thread": 2, "stuffing": 2, "spray": 2},
		"friend_chance": 0.25,
		"routes": [
			{"via": [Vector2(420, 760), Vector2(330, 520), Vector2(380, 360)], "minutes": 240.0, "chance": 0.8, "dangers": [["water", 0.45], ["mud", 0.7]]},
			{"via": [Vector2(560, 820), Vector2(640, 420)], "minutes": 120.0, "chance": 0.55, "dangers": [["wolf", 0.55], ["cliff", 0.8]]},
		],
	},
}

## The camp on the map.
const CAMP := Vector2(360, 1010)


## Every point of a route, camp to place.
static func route_points(place: String, route: int) -> PackedVector2Array:
	var p: Dictionary = PLACES[place]
	var pts := PackedVector2Array([CAMP])
	for v in p["routes"][route]["via"]:
		pts.append(v)
	pts.append(p["at"])
	return pts


## A point part-way along a route (0..1 of its length).
static func along(pts: PackedVector2Array, t: float) -> Vector2:
	var total := 0.0
	for i in pts.size() - 1:
		total += pts[i].distance_to(pts[i + 1])
	var want := clampf(t, 0.0, 1.0) * total
	for i in pts.size() - 1:
		var seg := pts[i].distance_to(pts[i + 1])
		if want <= seg:
			return pts[i].lerp(pts[i + 1], want / maxf(seg, 0.001))
		want -= seg
	return pts[pts.size() - 1]


## The chance this friend comes back well from this route, 0..1: the
## route's own chance, lowered by a body in poor shape, missing legs,
## hunger and stiff joints (0028), raised a little by stamina from
## earlier trips (0033).
static func chance(f: FriendData, place: String, route: int) -> float:
	var base: float = PLACES[place]["routes"][route]["chance"]
	var legs := 0
	for slot in ["leg_l", "leg_r"]:
		if f.part(slot) != null and f.part(slot).attached:
			legs += 1
	var c := base * (0.55 + 0.45 * f.wellbeing())
	c *= [0.45, 0.75, 1.0][legs]
	if f.stuffing < 0.6:
		c *= 0.85
	if f.fluid < 0.4:
		c *= 0.9
	c += f.stamina * 0.1
	return clampf(c, 0.03, 0.98)
