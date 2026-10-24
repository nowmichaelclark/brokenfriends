extends Node
## Game-wide events. Systems announce what happened here instead of
## reaching into each other, so new listeners (sound, Driftloom music,
## achievements) can join without the others knowing they exist.
## See plan/systems-map.md.

## A friend reacted to the player: "boop", "giggle", "dislike",
## "purr", "wake", "wince", "cheer", "chomp", "glug", "bonk", "catch".
signal friend_reacted(kind: String)

## A part came off a friend ("torn" or "trashed") or went on ("sewn").
signal part_changed(slot: String, change: String)

## A care action changed the friend's condition ("washed", "dried",
## "fed_stuffing", "fed_oil").
signal cared(kind: String)

## A surgery step finished ("fluff_cleared", "ligament", "soaked", "sewn").
signal surgery_step(step: String)

## The player moved between places ("tent", "play_corner").
signal place_changed(place: String)
