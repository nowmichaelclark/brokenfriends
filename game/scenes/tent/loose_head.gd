class_name LooseHead
extends Node2D
## A head that has come off (decision 0026): the friend lives on in it
## for about a day. In the player's hand it can go onto a body without a
## head, into the pocket, or into the bin.

## The friend, as a head only (`FriendData.head_only`).
var friend: FriendData
var held := true
## Size in the hand.
const SIZE := 0.62

var _t := 0.0


func _ready() -> void:
	scale = Vector2(SIZE, SIZE)


func _process(delta: float) -> void:
	_t += delta
	rotation = sin(_t * 3.0) * 0.08


## True if a world point is on this head.
func hits(world: Vector2) -> bool:
	return to_local(world).length() < 110.0


func _draw() -> void:
	if friend:
		draw_head(self, friend, Vector2.ZERO)


## A head drawn on its own: ears, face, button eyes, and fluff where the
## neck tore. Also used for heads waiting in the pocket.
static func draw_head(ci: CanvasItem, f: FriendData, at: Vector2) -> void:
	var fabric := f.head_color()
	var trim := f.trim
	for t in 3:
		PlushArt.fluff(ci, at + Vector2(-30 + t * 30, 92), 16.0)
	for side in [-1.0, 1.0]:
		var ear: Vector2 = at + Vector2(72.0 * side, -78.0)
		PlushArt.ellipse(ci, ear, 34.0, 34.0, fabric)
		PlushArt.ellipse(ci, ear, 19.0, 19.0, trim, false)
	PlushArt.ellipse(ci, at, 100.0, 94.0, fabric)
	PlushArt.ellipse(ci, at + Vector2(0, 38), 48.0, 33.0, trim, false)
	PlushArt.ellipse(ci, at + Vector2(0, 24), 12.0, 8.0, Color(0.22, 0.15, 0.13), false)
	for slot in ["eye_l", "eye_r"]:
		var e: PartData = f.part(slot)
		var x := -40.0 if slot == "eye_l" else 40.0
		if e:
			PlushArt.button_eye(ci, at + Vector2(x, -12), 21.0, e.fabric, 0.0, fabric, e.stitched)
	ci.draw_arc(at + Vector2(0, 58), 12.0, PI * 1.1, PI * 1.9, 8, Color(0.25, 0.15, 0.12), 3.0, true)
