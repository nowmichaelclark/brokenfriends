extends Node2D
## Twenty generated friends side by side (decision 0037, step B8), then
## the same twenty with heads swapped around, front and back. Run:
##   godot --path game -- --gallery=/tmp/gallery
## Saves gallery_front.png, gallery_back.png and gallery_swapped.png,
## prints GALLERY OK and quits. On the web: #gallery (shows, no files).

var out_dir := ""


func _ready() -> void:
	var bg := ColorRect.new()
	bg.color = Color(0.8, 0.72, 0.58)
	bg.size = Vector2(4000, 4000)
	bg.position = Vector2(-1000, -1000)
	add_child(bg)
	var cam := Camera2D.new()
	cam.position = Vector2(360, 640)
	add_child(cam)
	cam.make_current()
	if Flags.has("by_plan"):
		await _by_plan()
		return
	var friends: Array[FriendData] = []
	for i in 20:
		friends.append(FriendData.test_friend(1000 + i * 37))
	var plans := {}
	for f in friends:
		plans[f.hl("plan")] = true
	print("gallery: ", friends.size(), " friends, plans ", plans.keys())
	var views := _lay_out(friends)
	await _shoot("gallery_front")
	for v in views:
		v.facing = "back"
	await _shoot("gallery_back")
	# Swap every friend's head with the next one's (0026): any head fits
	# any body.
	var heads: Array[FriendData] = []
	var bodies: Array[FriendData] = []
	for f in friends:
		var body := f.take_head(0.0)
		heads.append(f)
		bodies.append(body)
	for i in heads.size():
		heads[i].join_body(bodies[(i + 1) % bodies.size()])
	for v in views:
		v.queue_free()
	_lay_out(heads)
	await _shoot("gallery_swapped")
	print("GALLERY OK")
	if not OS.has_feature("web"):
		get_tree().quit(0)


func _lay_out(friends: Array[FriendData]) -> Array:
	var views: Array = []
	for i in friends.size():
		var v := FriendView.new()
		v.bake_detail = 1.0
		v.scale = Vector2(0.27, 0.27)
		v.position = Vector2(80 + (i % 5) * 140, 180 + (i / 5) * 290)
		add_child(v)
		v.setup(friends[i])
		views.append(v)
	return views


func _shoot(name: String) -> void:
	for i in 12:
		await get_tree().process_frame
	if out_dir == "":
		await get_tree().create_timer(3.0).timeout
		return
	await RenderingServer.frame_post_draw
	get_viewport().get_texture().get_image().save_png(out_dir.path_join(name + ".png"))


## Nine friends of each body plan, one picture per plan (`by_plan`).
func _by_plan() -> void:
	var buckets := {}
	for plan in Looks.PLANS:
		buckets[plan] = []
	var seed_value := 2000
	while buckets.values().any(func(b): return b.size() < 9):
		var f := FriendData.test_friend(seed_value)
		var plan: String = f.hl("plan")
		if buckets[plan].size() < 9:
			buckets[plan].append(f)
		seed_value += 1
	for plan in buckets:
		var views: Array = []
		for i in 9:
			var v := FriendView.new()
			v.bake_detail = 1.2
			v.scale = Vector2(0.5, 0.5)
			v.position = Vector2(130 + (i % 3) * 230, 260 + (i / 3) * 380)
			add_child(v)
			v.setup(buckets[plan][i])
			views.append(v)
		await _shoot("plan_" + plan)
		for v in views:
			v.queue_free()
	print("GALLERY OK")
	if not OS.has_feature("web"):
		get_tree().quit(0)
