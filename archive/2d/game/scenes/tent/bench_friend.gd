class_name BenchFriend
extends Node2D
## A friend sitting further back in the tent. Drawn by a full FriendView
## into a small picture that is painted again only every couple of
## seconds, so each friend at the back costs one draw call, not nine
## (platforms-tech: draw once, then move). The picture breathes by
## scaling a little.

## The area around the friend's centre that the picture holds, in the
## friend's own units (head and ears to below the feet).
const RECT := Rect2(-190, -340, 380, 600)
## How often the picture is painted again, seconds.
const REPAINT := 2.0

var view: FriendView
var data: FriendData

var _vp: SubViewport
var _sprite: Sprite2D
var _size := 1.0
var _t := 0.0
var _repaint := 0.0


func setup(friend: FriendData, size: float) -> void:
	data = friend
	_size = size
	_vp = SubViewport.new()
	_vp.transparent_bg = true
	_vp.disable_3d = true
	_vp.size = Vector2i(ceili(RECT.size.x * size), ceili(RECT.size.y * size))
	_vp.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(_vp)
	view = FriendView.new()
	view.bake_detail = 1.0
	view.scale = Vector2(size, size)
	view.position = -RECT.position * size
	_vp.add_child(view)
	view.setup(friend)
	_sprite = Sprite2D.new()
	_sprite.centered = false
	_sprite.texture = _vp.get_texture()
	_sprite.position = RECT.position * size
	var mat := CanvasItemMaterial.new()
	mat.blend_mode = CanvasItemMaterial.BLEND_MODE_PREMULT_ALPHA
	_sprite.material = mat
	add_child(_sprite)
	_t = randf() * TAU
	_repaint = randf() * REPAINT


func _process(delta: float) -> void:
	_t += delta
	var b := sin(_t * 1.6) * 0.012
	scale = Vector2(1.0 - b * 0.5, 1.0 + b)
	_repaint -= delta
	if _repaint <= 0.0:
		_repaint = REPAINT
		_vp.render_target_update_mode = SubViewport.UPDATE_ONCE


## True if a world point is on this friend.
func hits(world: Vector2) -> bool:
	return view.is_on_friend(to_local(world) / _size)
