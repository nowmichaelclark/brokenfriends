class_name Flags
extends RefCounted
## Debug switches. On the laptop: `godot --path game -- --perf`. On the
## web: add them to the address after #, joined by &, e.g. `#perf&nomusic`.

static var _cache: PackedStringArray = []
static var _read := false


static func has(name: String) -> bool:
	if not _read:
		_read = true
		for arg in OS.get_cmdline_user_args():
			_cache.append(arg.trim_prefix("--"))
		if OS.has_feature("web"):
			var page_hash = JavaScriptBridge.eval("window.location.hash", true)
			for part in str(page_hash).trim_prefix("#").split("&", false):
				_cache.append(part)
	return name in _cache


## The value of a switch written `name=value` (e.g. `days=3`), or
## `fallback` if it isn't there.
static func value(name: String, fallback := "") -> String:
	has(name)
	for item in _cache:
		if item.begins_with(name + "="):
			return item.get_slice("=", 1)
	return fallback
