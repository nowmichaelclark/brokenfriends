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
		draw_head(self, friend, Transform2D.IDENTITY)


## A head drawn on its own: ears, face, button eyes, and fluff where the
## neck tore. Also used for heads in the pocket and friends on the map.
static func draw_head(ci: CanvasItem, f: FriendData, xf: Transform2D) -> void:
	FriendArt.draw_loose_head(ci, f, xf, f.head_only)
