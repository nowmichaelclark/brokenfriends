class_name Baked
extends Node2D
## A drawing made once into a texture and shown as one sprite: one draw
## call however detailed the drawing. It is drawn again ("re-baked") only
## when `refresh` is given a new key. Like Driftloom's covers: drawn once,
## then only pasted.
##
## `rect` is the area the painter draws in, in this node's coordinates.
## `resolution` is how many texture pixels per unit: 2 keeps it sharp on
## phone screens and when the camera zooms in a little.

var painter: Callable
var key = null

var _vp: SubViewport
var _canvas: Canvas
var _sprite: Sprite2D


func setup(rect: Rect2, paint: Callable, resolution := 2.0) -> Baked:
	painter = paint
	_vp = SubViewport.new()
	_vp.transparent_bg = true
	_vp.disable_3d = true
	_vp.size = Vector2i(ceili(rect.size.x * resolution), ceili(rect.size.y * resolution))
	_vp.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(_vp)
	_canvas = Canvas.new()
	_canvas.baked = self
	_canvas.position = -rect.position * resolution
	_canvas.scale = Vector2(resolution, resolution)
	_vp.add_child(_canvas)
	_sprite = Sprite2D.new()
	_sprite.centered = false
	_sprite.texture = _vp.get_texture()
	_sprite.position = rect.position
	_sprite.scale = Vector2.ONE / resolution
	_sprite.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	# A transparent viewport's texture has its colors premultiplied by alpha.
	var mat := CanvasItemMaterial.new()
	mat.blend_mode = CanvasItemMaterial.BLEND_MODE_PREMULT_ALPHA
	_sprite.material = mat
	add_child(_sprite)
	return self


## Draw again only if what this shows has changed.
func refresh(new_key) -> void:
	if new_key != key:
		key = new_key
		_canvas.queue_redraw()
		_vp.render_target_update_mode = SubViewport.UPDATE_ONCE


class Canvas:
	extends Node2D
	var baked: Baked

	func _draw() -> void:
		baked.painter.call(self)
