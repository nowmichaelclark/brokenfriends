extends Node
## Saves the game as JSON in the player's own storage (on the web,
## the browser's). Saves a timestamp too, so time away can be worked
## out when the game reopens (plan/systems/time-save).

const PATH := "user://broken_friends.json"
const VERSION := 1

var friend: FriendData
var spare_available := false
var _dirty := false
var _timer := 0.0


func _ready() -> void:
	load_game()


func load_game() -> void:
	friend = null
	if FileAccess.file_exists(PATH):
		var f := FileAccess.open(PATH, FileAccess.READ)
		var parsed = JSON.parse_string(f.get_as_text())
		if parsed is Dictionary and int(parsed.get("version", 0)) == VERSION:
			friend = FriendData.from_dict(parsed.get("friend", {}))
			spare_available = parsed.get("spare_available", false)
	if friend == null:
		new_game()


func new_game() -> void:
	friend = FriendData.first_friend()
	spare_available = false
	save_now()


## Ask for a save; it happens within a second, so rapid changes
## (rubbing mud off) don't write every frame.
func mark_dirty() -> void:
	_dirty = true


func save_now() -> void:
	_dirty = false
	var data := {
		"version": VERSION,
		"saved_at": Time.get_unix_time_from_system(),
		"friend": friend.to_dict(),
		"spare_available": spare_available,
	}
	var f := FileAccess.open(PATH, FileAccess.WRITE)
	if f:
		f.store_string(JSON.stringify(data, "\t"))


func _process(delta: float) -> void:
	if _dirty:
		_timer += delta
		if _timer > 1.0:
			_timer = 0.0
			save_now()


func _notification(what: int) -> void:
	if what == NOTIFICATION_WM_CLOSE_REQUEST or what == NOTIFICATION_APPLICATION_PAUSED \
			or what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		if friend:
			save_now()
