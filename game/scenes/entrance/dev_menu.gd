class_name DevMenu
extends PanelContainer
## The dev menu (decision 0045: the first version is a dev release,
## everything easy to reach). Words are fine here, as in the physics
## test. Every mini game, every test friend, and the play room's stress
## test.

var entrance: Entrance

var _list: VBoxContainer


func _ready() -> void:
	set_anchors_preset(Control.PRESET_FULL_RECT)
	var style := StyleBoxFlat.new()
	style.bg_color = Color(0.12, 0.1, 0.09, 0.94)
	add_theme_stylebox_override("panel", style)
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	add_child(scroll)
	var margin := MarginContainer.new()
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 28)
	scroll.add_child(margin)
	_list = VBoxContainer.new()
	_list.add_theme_constant_override("separation", 10)
	margin.add_child(_list)
	_title("Dev menu")
	_button("Close", close)
	_title("Mini games")
	_button("Physics test (ragdolls, the board)", func(): _go("physics"))
	_button("Skateboard grind course (not built yet)", Callable()).disabled = true
	_title("Add a test friend to camp")
	var row: HFlowContainer = null
	for kind in test_friends():
		if row == null or row.get_child_count() >= 3:
			row = HFlowContainer.new()
			row.add_theme_constant_override("h_separation", 10)
			_list.add_child(row)
		row.add_child(_make_button(kind, func(): add_friend(kind)))
	_title("Play room: how many friends (stress test)")
	row = HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 10)
	_list.add_child(row)
	for n in [0, 10, 20, 40]:
		row.add_child(_make_button("camp only" if n == 0 else "+%d" % n, func(): _crowd(n)))


func open() -> void:
	# The whole screen (anchors under a CanvasLayer don't stretch it here).
	position = Vector2.ZERO
	size = get_viewport_rect().size
	visible = true
	Voice.pop()


func close() -> void:
	visible = false


func _title(text: String) -> void:
	var l := Label.new()
	l.text = text
	l.add_theme_font_size_override("font_size", 30)
	l.add_theme_color_override("font_color", Color(1.0, 0.9, 0.7))
	_list.add_child(l)


func _make_button(text: String, action: Callable) -> Button:
	var b := Button.new()
	b.text = text
	b.focus_mode = Control.FOCUS_NONE
	b.add_theme_font_size_override("font_size", 26)
	b.custom_minimum_size = Vector2(0, 64)
	if action.is_valid():
		b.pressed.connect(action)
	return b


func _button(text: String, action: Callable) -> Button:
	var b := _make_button(text, action)
	_list.add_child(b)
	return b


func _go(place: String) -> void:
	close()
	entrance.go_to.emit(place)


func _crowd(n: int) -> void:
	PlayRoom.crowd = n
	_go("play_room")


## Every test friend the menu can add: one per body plan, then the
## physics test's troubles.
static func test_friends() -> Array:
	return Looks.PLANS.keys() + ["no legs", "one arm", "soaked", "rotting leg", "open hole", "loose seam"]


## A test friend of a kind (see `test_friends`). The camp's limit
## (0013) doesn't apply: this is for testing (0045).
static func make_friend(kind: String) -> FriendData:
	if kind in Looks.PLANS:
		for seed_value in range(1, 400):
			if Looks.generate(seed_value)["body"]["plan"] == kind:
				var f := FriendData.test_friend(seed_value)
				f.name = "Test %s" % kind
				return f
	var f := FriendData.test_friend(700 + test_friends().find(kind))
	f.name = "Test: %s" % kind
	match kind:
		"no legs":
			f.parts["leg_l"] = null
			f.parts["leg_r"] = null
			f.close_hole("leg_l")
			f.close_hole("leg_r")
		"one arm":
			f.parts["arm_r"] = null
			f.close_hole("arm_r")
		"soaked":
			f.wetness = 1.0
		"rotting leg":
			f.parts["leg_l"].rot = 0.8
			f.parts["leg_l"].mold = [[0.5, 0.0, 0.15]]
		"open hole":
			f.parts["arm_l"] = null
			f.hole_rot["arm_l"] = 0.5
		"loose seam":
			var arm: PartData = f.parts["arm_l"]
			arm.pulls = arm.pulls_needed() - 1
	return f


func add_friend(kind: String) -> void:
	var camp := Save.camp
	camp.friends.append(make_friend(kind))
	camp.current = camp.friends.size() - 1
	Save.mark_dirty()
	Events.friend_arrived.emit(camp.current)
	close()
	entrance.build()
