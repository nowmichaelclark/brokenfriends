extends Node
## Saves the camp as JSON in the player's own storage (on the web, the
## browser's), and keeps the game's clock: on opening, the camp catches
## up on the time it was left alone (plan/systems/time-save).
##
## Debug switches (debug/flags.gd): `days=N` pretends N more days have
## passed since the last save; `fast=N` runs the clock N times faster
## while playing; `friends=N` starts a new game with N friends.

const PATH := "user://broken_friends.json"
const VERSION := 2
## How often the camp moves on while the game is open, in seconds.
const TICK := 5.0

var camp: CampData
## The friend on the tent floor.
var friend: FriendData:
	get:
		return camp.friend() if camp else null

var _dirty := false
var _timer := 0.0
var _tick := 0.0
var _started := 0.0
var _fast := 1.0
var _offset := 0.0


func _ready() -> void:
	_started = Time.get_unix_time_from_system()
	_fast = maxf(1.0, float(Flags.value("fast", "1")))
	_offset = float(Flags.value("days", "0")) * 86400.0
	load_game()


## The game's time, in seconds since 1970: the real clock, plus the
## debug switches.
func now() -> float:
	var real := Time.get_unix_time_from_system()
	return real + _offset + (real - _started) * (_fast - 1.0)


func load_game() -> void:
	camp = null
	if FileAccess.file_exists(PATH):
		var f := FileAccess.open(PATH, FileAccess.READ)
		var parsed = JSON.parse_string(f.get_as_text())
		if parsed is Dictionary:
			camp = _from_save(parsed)
	if camp == null or camp.friends.is_empty():
		new_game()
		return
	# Time away: the camp catches up on everything it missed.
	advance_to(now())


func _from_save(d: Dictionary) -> CampData:
	match int(d.get("version", 0)):
		2:
			return CampData.from_dict(d.get("camp", {}))
		1:
			# The first sketch saved one friend: keep it.
			var c := CampData.new()
			c.friends.append(FriendData.from_dict(d.get("creature", {})))
			if d.get("spare_available", false):
				c.spares.append(FriendData.spare_leg())
				c.spare_given = true
			c.clock = float(d.get("saved_at", now()))
			return c
	return null


func new_game() -> void:
	camp = CampData.first_camp(now())
	for i in int(Flags.value("friends", "1")) - 1:
		camp.friends.append(FriendData.test_friend(100 + i))
	if Flags.value("supplies") != "":
		for k in camp.supplies:
			camp.supplies[k] = int(Flags.value("supplies"))
	save_now()


## Move the camp on to time `t` and announce what happened.
func advance_to(t: float) -> void:
	camp.advance(t - camp.clock, t)
	for i in camp.just_arrived:
		Events.friend_arrived.emit.call_deferred(i)
	camp.just_arrived.clear()
	for q in camp.just_returned:
		Events.quest_ended.emit.call_deferred(q.place, q.result)
	camp.just_returned.clear()


## Ask for a save; it happens within a second, so rapid changes
## (rubbing mud off) don't write every frame.
func mark_dirty() -> void:
	_dirty = true


func save_now() -> void:
	_dirty = false
	var data := {
		"version": VERSION,
		"saved_at": now(),
		"camp": camp.to_dict(),
	}
	var f := FileAccess.open(PATH, FileAccess.WRITE)
	if f:
		f.store_string(JSON.stringify(data, "\t"))


func _process(delta: float) -> void:
	_tick += delta
	if _tick >= TICK and camp:
		_tick = 0.0
		advance_to(now())
		mark_dirty()
	if _dirty:
		_timer += delta
		if _timer > 1.0:
			_timer = 0.0
			save_now()


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST or what == NOTIFICATION_APPLICATION_PAUSED \
			or what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		if camp:
			save_now()
